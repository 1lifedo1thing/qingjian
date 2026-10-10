//! 设置程序的状态：配置文件（唯一事实源）、主题库、当前页。改动一律原地写回配置文件再重读，输入法热加载它。

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use qingjian_platform::{Appearance, Config, ConfigError, LayoutMode};
use qingjian_render::{TextSizes, Theme, ThemeLibrary};
use render_engine::taffy::NodeId;
use render_engine::{
    App, FamilyId, FamilyList, FamilyTable, RenderError, SvgImage, TextEdit, TextPainter, Ui, Waker,
};

use crate::action::Action;
use crate::cloud::CloudTest;
use crate::import::{ImportJob, Report};
use crate::library::{Kind, Library};
use crate::message::{FieldId, Message, Popup, SizeKind};
use crate::page_id::PageId;
use crate::phrases::PhraseEditor;
use crate::previews::Previews;
use crate::shortcut::Shortcut;
use crate::stats::Stats;
use crate::update::UpdateCheck;
use crate::view;

/// 字号步进的范围（点）。
const SIZES: std::ops::RangeInclusive<f32> = 6.0..=72.0;

pub struct Settings {
    path: PathBuf,

    pub config: Config,

    pub themes: ThemeLibrary,

    pub page: PageId,

    /// 系统是深色：设置窗口自己跟它走，候选窗「跟随系统」时也按它。
    pub system_dark: bool,

    pub previews: RefCell<Previews>,

    pub logo: Option<Arc<SvgImage>>,

    /// 侧栏品牌字用的宋体字族。
    pub serif: FamilyId,

    /// 开着的下拉菜单。
    pub menu: Option<Popup>,

    /// 本机字族，第一次打开字体菜单时扫。
    pub fonts: Option<Vec<String>>,

    /// 点过一次、等再点一次确认的动作（清空输入日志）；点了别的就作废。
    pub armed: Option<Action>,

    /// 做完一件事的那句话和它显示在哪一行下面（[`Action::slot`]、`"cloud"`），点了别的就收掉。
    pub notice: Option<(&'static str, String)>,

    /// 输入框的编辑状态；回车或离开时才存进配置。
    pub fields: HashMap<FieldId, TextEdit>,

    pub cloud_test: CloudTest,

    pub update: UpdateCheck,

    /// 正在录的快捷键。
    pub recording: Option<Shortcut>,

    /// 词库与码表的清单，进词库页、导入、移除后重扫。
    pub library: Library,

    pub import: ImportJob,

    /// 开着的短语编辑卡片。
    pub phrase: Option<PhraseEditor>,

    /// 统计页的数，进统计页时读。
    pub stats: Option<Stats>,

    waker: Option<Waker>,

    /// 「关于」页的数据来源展开着。
    pub sources_open: bool,
}

impl Settings {
    pub fn load(path: PathBuf) -> Result<Self, ConfigError> {
        // macOS 偏好设置以前把密钥写在配置同目录的 .env 里，读进环境才知道密钥设没设、测试连接也用得上
        let _ = dotenvy::from_path(path.with_file_name(".env"));
        let config = if path.exists() {
            Config::load(&path)?
        } else {
            Config::default()
        };
        let themes = ThemeLibrary::load(crate::paths::themes_dir().as_deref());
        let logo = SvgImage::parse(include_bytes!("../assets/logo.svg"))
            .map(Arc::new)
            .inspect_err(|error| tracing::warn!(%error, "logo 解析失败"))
            .ok();
        Ok(Self {
            path,
            config,
            themes,
            page: PageId::Input,
            system_dark: false,
            previews: RefCell::new(Previews::new()),
            logo,
            serif: FamilyId::default(),
            menu: None,
            fonts: None,
            armed: None,
            notice: None,
            sources_open: false,
            fields: HashMap::new(),
            cloud_test: CloudTest::default(),
            update: UpdateCheck::load(),
            recording: None,
            library: Library::scan(),
            import: ImportJob::default(),
            phrase: None,
            stats: None,
            waker: None,
        }
        .with_fields())
    }

    fn with_fields(mut self) -> Self {
        for id in FieldId::ALL {
            self.reset_field(id);
        }
        self
    }

    /// 输入框换回配置里的值（密钥框总是空的，不显示已存的密钥）。
    fn reset_field(&mut self, id: FieldId) {
        let predict = &self.config.predict;
        let value = match id {
            FieldId::BaseUrl => predict.base_url.clone(),
            FieldId::Model => predict.model.clone(),
            FieldId::ApiKey | FieldId::PhraseCode | FieldId::PhraseText => return,
        };
        self.fields.insert(id, TextEdit::new(value));
    }

    /// 存一个输入框：没改、清空了都不存（清空等于不改）。
    fn commit(&mut self, id: FieldId) {
        let Some(text) = self
            .fields
            .get(&id)
            .map(|edit| edit.text().trim().to_owned())
        else {
            return;
        };
        match id {
            FieldId::BaseUrl | FieldId::Model => {
                let current = match id {
                    FieldId::BaseUrl => &self.config.predict.base_url,
                    _ => &self.config.predict.model,
                };
                if !text.is_empty() && text != *current {
                    self.write("predict", id.key(), text);
                }
            }
            FieldId::PhraseCode | FieldId::PhraseText => return,
            FieldId::ApiKey if text.is_empty() => {}
            // 密码框看不见内容，粘贴多了（带上了终端提示符、命令）用户发现不了
            FieldId::ApiKey if text.chars().any(|c| !c.is_ascii_graphic()) => {
                self.notice = Some((
                    "cloud",
                    "密钥没有保存：里面有空格或非英文字符，多半是粘贴时多带了别的内容".to_owned(),
                ));
            }
            FieldId::ApiKey => {
                qingjian_platform::logs::secrets::register(&text);
                self.write("predict", id.key(), text);
                self.notice = Some(("cloud", "密钥已保存".to_owned()));
            }
        }
        self.reset_field(id);
    }

    /// 导入完了：之前关掉过的同名词库重新打开，重扫清单，把结果显示出来。
    fn imported(&mut self, report: Report) {
        let kind = report.kind;
        let (section, key) = kind.disabled_key();
        let disabled = match kind {
            Kind::Dictionary => &self.config.dictionaries.disabled,
            Kind::CodeTable => &self.config.aux_code.disabled,
        };
        let kept: Vec<String> = disabled
            .iter()
            .filter(|stem| !report.imported.contains(stem))
            .cloned()
            .collect();
        if kept.len() != disabled.len() {
            self.write_list(section, key, &kept);
        }
        self.library = Library::scan();
        self.notice = Some((kind.slot(), report.lines.join("\n")));
    }

    fn open_phrase(&mut self, index: Option<usize>) {
        let (editor, code, text) =
            match index.and_then(|i| Some((i, self.config.custom_phrases.get(i)?))) {
                Some((i, phrase)) => (
                    PhraseEditor::edit(i, phrase),
                    phrase.code.clone(),
                    phrase.text.clone(),
                ),
                None => (PhraseEditor::new(), String::new(), String::new()),
            };
        self.fields.insert(FieldId::PhraseCode, TextEdit::new(code));
        self.fields
            .insert(FieldId::PhraseText, TextEdit::multiline(text));
        self.phrase = Some(editor);
    }

    fn close_phrase(&mut self) {
        self.phrase = None;
        self.fields.remove(&FieldId::PhraseCode);
        self.fields.remove(&FieldId::PhraseText);
    }

    /// 存卡片（`delete` 时删掉这条）：整张表校验写回，失败把原因留在卡片里。
    fn save_phrase(&mut self, delete: bool) {
        let Some(editor) = self.phrase.clone() else {
            return;
        };
        let field = |id| {
            self.fields
                .get(&id)
                .map(|edit: &TextEdit| edit.text().to_owned())
                .unwrap_or_default()
        };
        let phrase = (!delete).then(|| qingjian_core::CustomPhrase {
            code: field(FieldId::PhraseCode).trim().to_owned(),
            text: field(FieldId::PhraseText),
            position: editor.position,
            enabled: editor.enabled,
        });
        let saved = editor
            .apply(&self.config.custom_phrases, phrase)
            .and_then(|list| self.write_phrases(&list));
        match saved {
            Ok(()) => self.close_phrase(),
            Err(error) => {
                if let Some(editor) = self.phrase.as_mut() {
                    editor.error = Some(error);
                    editor.confirm_delete = false;
                }
            }
        }
    }

    fn write_phrases(&mut self, list: &[qingjian_core::CustomPhrase]) -> Result<(), String> {
        Config::set_custom_phrases(&self.path, list)?;
        self.reload();
        Ok(())
    }

    /// 快捷键页的各项（含翻页键、模式键）写回缺省；辅码触发键在「输入」页，不动。
    fn reset_shortcuts(&mut self) {
        let defaults = qingjian_platform::ShortcutConfig::default();
        self.write(
            "general",
            "page_keys",
            qingjian_platform::PAGE_KEY_OPTIONS[0],
        );
        self.write(
            "shortcut",
            "expression",
            defaults.mode.expression.to_string(),
        );
        self.write("shortcut", "question", defaults.mode.question.to_string());
        self.write("shortcut", "question_mark", defaults.mode.question_mark);
        self.write("shortcut", "translation", defaults.translation.key());
        self.write(
            "shortcut",
            "translation_second",
            defaults.translation_second.key(),
        );
        self.write(
            "shortcut",
            "delete_candidate",
            defaults.delete_candidate.key(),
        );
        self.write(
            "shortcut",
            "translate_selection",
            defaults.translate_selection.key_string(),
        );
        self.recording = None;
    }

    /// 候选窗该用深色：按「外观」设置，跟随系统时看系统。
    pub fn candidate_dark(&self) -> bool {
        match self.config.general.appearance() {
            Appearance::System => self.system_dark,
            Appearance::Light => false,
            Appearance::Dark => true,
        }
    }

    /// 当前主题，按外观与设置里的字号换好。
    pub fn theme(&self, id: &str) -> Theme {
        let general = &self.config.general;
        self.themes
            .resolve(id, self.candidate_dark())
            .with_text_sizes(TextSizes::new(
                general.candidate_font_size.0,
                general.annotation_font_size.0,
            ))
    }

    /// 字号框里显示的数：设置里写了用它，没写用主题的。
    pub fn size(&self, kind: SizeKind) -> f32 {
        let general = &self.config.general;
        let (own, name) = match kind {
            SizeKind::Candidate => (general.candidate_font_size, "candidate"),
            SizeKind::Annotation => (general.annotation_font_size, "annotation"),
        };
        own.get()
            .or_else(|| self.theme(general.theme_id()).theme_size(name))
            .unwrap_or(13.0)
    }

    fn write(&mut self, section: &str, key: &str, value: impl Into<toml_edit::Value>) {
        if let Err(error) = Config::set_value(&self.path, section, key, value) {
            tracing::error!(%error, section, key, "写配置失败");
            return;
        }
        self.reload();
    }

    fn write_list(&mut self, section: &str, key: &str, values: &[String]) {
        if let Err(error) = Config::set_array(&self.path, section, key, values) {
            tracing::error!(%error, section, key, "写配置失败");
            return;
        }
        self.reload();
    }

    fn reload(&mut self) {
        match Config::load(&self.path) {
            Ok(config) => self.config = config,
            Err(error) => tracing::error!(%error, "重读配置失败"),
        }
    }
}

impl App for Settings {
    type Message = Message;

    fn view(
        &self,
        ui: &mut Ui<Message>,
        scale: f32,
        size: (u32, u32),
    ) -> Result<NodeId, RenderError> {
        view::view(ui, self, scale, size)
    }

    fn update(&mut self, message: Message) -> bool {
        let armed = self.armed.take();
        if !matches!(
            message,
            Message::Run(_) | Message::TestCloud | Message::CheckUpdate | Message::Import(_)
        ) {
            self.notice = None;
        }
        match message {
            Message::Page(page) => {
                // 输入框在别的页上会直接消失、收不到失焦，这里补存
                for id in FieldId::ALL {
                    self.commit(id);
                }
                self.page = page;
                self.recording = None;
                self.close_phrase();
                if page == PageId::Dictionaries {
                    self.library = Library::scan();
                }
                if page == PageId::Stats {
                    self.stats = Some(Stats::load(&self.config.general));
                }
            }
            Message::Record(shortcut) => self.recording = Some(shortcut),
            Message::StopRecording => self.recording = None,
            Message::Recorded(shortcut, press) => match shortcut.accept(&press, &self.config) {
                Ok(value) => {
                    let (section, key) = shortcut.config_key();
                    self.write(section, key, value);
                    self.recording = None;
                }
                Err(reason) => self.notice = Some((shortcut.title(), reason)),
            },
            Message::ResetShortcuts => self.reset_shortcuts(),
            Message::OpenPhrase(index) => self.open_phrase(index),
            Message::PhrasePosition(delta) => {
                if let Some(editor) = self.phrase.as_mut() {
                    editor.position = editor
                        .position
                        .saturating_add_signed(delta as isize)
                        .clamp(1, 9);
                }
            }
            Message::SavePhrase => self.save_phrase(false),
            Message::DeletePhrase => {
                let confirmed = self
                    .phrase
                    .as_ref()
                    .is_some_and(|editor| editor.confirm_delete);
                if confirmed {
                    self.save_phrase(true);
                } else if let Some(editor) = self.phrase.as_mut() {
                    editor.confirm_delete = true;
                }
            }
            Message::ClosePhrase => self.close_phrase(),
            Message::TogglePhrase(index) => {
                let mut list = self.config.custom_phrases.clone();
                if let Some(phrase) = list.get_mut(index) {
                    phrase.enabled = !phrase.enabled;
                    self.write_phrases(&list).unwrap_or_else(|error| {
                        self.notice = Some(("phrases", error));
                    });
                }
            }
            Message::Import(kind) => match self.import.start(kind, self.waker.clone()) {
                Ok(true) => self.notice = Some((kind.slot(), "正在导入…".to_owned())),
                Ok(false) => {}
                Err(error) => self.notice = Some((kind.slot(), error)),
            },
            Message::Remove(kind, stem) => {
                if let Err(error) = crate::import::remove(kind, &stem) {
                    self.notice = Some((kind.slot(), error));
                }
                self.library = Library::scan();
            }
            Message::ModeKey(key, letter) => {
                let mode = self.config.shortcut.mode.sanitized();
                let (current, other, other_key) = if key == "expression" {
                    (mode.expression, mode.question, "question")
                } else {
                    (mode.question, mode.expression, "expression")
                };
                if letter == other {
                    self.write("shortcut", other_key, current.to_string());
                }
                self.write("shortcut", key, letter.to_string());
            }
            Message::Edit(id, edit) => {
                if let Some(copied) = self
                    .fields
                    .get_mut(&id)
                    .and_then(|field| field.apply(&edit))
                {
                    render_engine::set_clipboard(&copied);
                }
            }
            Message::CheckUpdate => {
                if !self.update.running() {
                    let started = self.update.start(&self.config.update, self.waker.clone());
                    self.notice = Some((
                        "update",
                        started.err().unwrap_or_else(|| "正在检查…".to_owned()),
                    ));
                }
            }
            Message::Commit(id) => self.commit(id),
            Message::Cancel(id) => self.reset_field(id),
            Message::TestCloud => {
                if !self.cloud_test.running() {
                    let text = match self
                        .cloud_test
                        .start(&self.config.predict, self.waker.clone())
                    {
                        Ok(()) => "正在测试…".to_owned(),
                        Err(error) => error,
                    };
                    self.notice = Some(("cloud", text));
                }
            }
            Message::Set(section, key, value) => {
                self.menu = None;
                self.write(section, key, value);
            }
            Message::SetList(section, key, values) => self.write_list(section, key, &values),
            Message::OpenMenu(popup) => {
                if popup == Popup::Font && self.fonts.is_none() {
                    self.fonts = Some(crate::fonts::families());
                }
                self.menu = Some(popup);
            }
            Message::CloseMenu => self.menu = None,
            Message::Size(kind, delta) => {
                let value = (self.size(kind) + delta as f32).clamp(*SIZES.start(), *SIZES.end());
                let key = match kind {
                    SizeKind::Candidate => "candidate_font_size",
                    SizeKind::Annotation => "annotation_font_size",
                };
                self.write("general", key, f64::from(value));
            }
            Message::ResetSizes => {
                self.write("general", "candidate_font_size", 0.0);
                self.write("general", "annotation_font_size", 0.0);
            }
            Message::Run(action) => {
                // 清空要点两下：第一下只是把按钮换成确认
                if action == Action::ClearInputLog && armed != Some(action) {
                    self.armed = Some(action);
                    self.notice = None;
                } else {
                    self.notice = action.run(&self.path).map(|text| (action.slot(), text));
                }
            }
            Message::ToggleSources => self.sources_open = !self.sources_open,
            Message::Quit => return false,
        }
        true
    }

    fn setup(&mut self, text: &mut TextPainter) {
        let mut table = FamilyTable::new(FamilyList::system());
        self.serif = table.intern(&FamilyList(vec![
            "Songti SC".to_owned(),
            "SimSun".to_owned(),
            "Noto Serif CJK SC".to_owned(),
        ]));
        text.use_families(&Arc::new(table));
    }

    fn set_waker(&mut self, waker: Waker) {
        self.waker = Some(waker);
    }

    fn wake(&mut self) -> bool {
        let mut changed = false;
        if let Some(text) = self.cloud_test.finish() {
            self.notice = Some(("cloud", text));
            changed = true;
        }
        if let Some(report) = self.import.finish() {
            self.imported(report);
            changed = true;
        }
        if let Some(text) = self.update.finish(&self.config.update) {
            self.notice = Some(("update", text));
            changed = true;
        }
        changed
    }

    fn set_system_dark(&mut self, dark: bool) {
        self.system_dark = dark;
    }
}

/// 竖排还是横排（预览用）。
pub fn horizontal(layout: LayoutMode) -> bool {
    layout == LayoutMode::Horizontal
}

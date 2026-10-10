//! 设置程序的状态：配置文件（唯一事实源）、主题库、当前页。改动一律原地写回配置文件再重读，输入法热加载它。

use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::Arc;

use qingjian_platform::{Appearance, Config, ConfigError, LayoutMode};
use qingjian_render::{TextSizes, Theme, ThemeLibrary};
use render_engine::taffy::NodeId;
use render_engine::{
    App, FamilyId, FamilyList, FamilyTable, RenderError, SvgImage, TextPainter, Ui,
};

use crate::message::{Message, SizeKind};
use crate::previews::Previews;
use crate::view;

/// 字号步进的范围（点）。
const SIZES: std::ops::RangeInclusive<f32> = 6.0..=72.0;

pub struct Settings {
    path: PathBuf,

    pub config: Config,

    pub themes: ThemeLibrary,

    pub page: usize,

    /// 系统是深色：设置窗口自己跟它走，候选窗「跟随系统」时也按它。
    pub system_dark: bool,

    pub previews: RefCell<Previews>,

    pub logo: Option<Arc<SvgImage>>,

    /// 侧栏品牌字用的宋体字族。
    pub serif: FamilyId,

    /// 字体下拉菜单开着。
    pub font_menu: bool,

    /// 本机字族，第一次打开字体菜单时扫。
    pub fonts: Option<Vec<String>>,
}

impl Settings {
    pub fn load(path: PathBuf) -> Result<Self, ConfigError> {
        let config = if path.exists() {
            Config::load(&path)?
        } else {
            Config::default()
        };
        let themes = ThemeLibrary::load(qingjian_platform::dirs::themes_dir().as_deref());
        let logo = SvgImage::parse(include_bytes!("../assets/logo.svg"))
            .map(Arc::new)
            .inspect_err(|error| tracing::warn!(%error, "logo 解析失败"))
            .ok();
        Ok(Self {
            path,
            config,
            themes,
            page: 1,
            system_dark: false,
            previews: RefCell::new(Previews::new()),
            logo,
            serif: FamilyId::default(),
            font_menu: false,
            fonts: None,
        })
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
        let general = self.config.general.clone();
        match message {
            Message::Page(page) => self.page = page,
            Message::Appearance(appearance) => {
                self.write("general", "appearance", appearance.key());
            }
            Message::Theme(id) => self.write("general", "theme", id),
            Message::Layout(layout) => self.write("general", "layout", layout.key()),
            Message::Grid => self.write("general", "horizontal_grid", !general.horizontal_grid),
            Message::Preedit(mode) => self.write("general", "preedit", mode.key()),
            Message::Renderer(renderer) => self.write("general", "renderer", renderer.key()),
            Message::OpenFonts => {
                if self.fonts.is_none() {
                    self.fonts = Some(crate::fonts::families());
                }
                self.font_menu = true;
            }
            Message::PickFont(family) => {
                self.font_menu = false;
                self.write("general", "font", family.unwrap_or_default());
            }
            Message::CloseMenu => self.font_menu = false,
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
            Message::Animations => self.write("general", "animations", !general.animations),
            Message::StatusBar => {
                let enabled = self.config.status_bar.enabled;
                self.write("status_bar", "enabled", !enabled);
            }
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

    fn set_system_dark(&mut self, dark: bool) {
        self.system_dark = dark;
    }
}

/// 竖排还是横排（预览用）。
pub fn horizontal(layout: LayoutMode) -> bool {
    layout == LayoutMode::Horizontal
}

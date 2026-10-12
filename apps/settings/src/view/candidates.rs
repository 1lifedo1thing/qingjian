//! 「候选」页：每页几个、译文、繁体、emoji，排序（学习、本地整句模型），英文词。

use qingjian_platform::MAX_PAGE_SIZE;
use render_engine::taffy::NodeId;
use render_engine::{RenderError, SettingRow, Stepper, Ui, group, section_title};

use super::Page;
use super::controls::{LANGUAGES, select, toggle, toggle_info, toggle_with};
use crate::message::{Message, Popup};

pub fn candidates(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let config = &page.settings.config;
    let general = &config.general;
    let mut out = Vec::new();

    let size = general.page_size();
    let stepper = Stepper::new(&size.to_string())
        .limits(size > 1, size < MAX_PAGE_SIZE)
        .build(
            ui,
            theme,
            "page-size",
            Message::set("general", "page_size", (size - 1) as i64),
            Message::set("general", "page_size", (size + 1) as i64),
        )?;
    let language = LANGUAGES
        .iter()
        .find(|(code, _)| *code == general.learning_language)
        .map_or(general.learning_language.as_str(), |(_, label)| label);
    let names: Vec<&str> = LANGUAGES.iter().map(|(_, name)| *name).collect();
    let language = select(
        ui,
        page,
        Popup::LearningLanguage,
        (language, &names),
        120.0,
        false,
    )?;
    let rows = vec![
        SettingRow::new("每页候选数")
            .control(stepper)
            .build(ui, theme)?,
        SettingRow::new("译文")
            .info("不显示时生词标记也关掉")
            .control(language)
            .build(ui, theme)?,
        toggle_info(
            ui,
            page,
            "繁体",
            Some("只换字形，不换用语"),
            ("general", "traditional"),
            general.traditional,
            false,
        )?,
        toggle(ui, page, "emoji", ("general", "emoji"), general.emoji)?,
    ];
    out.push(section_title(ui, theme, "显示")?);
    out.push(group(ui, theme, &rows)?);

    let rows = vec![
        toggle_info(
            ui,
            page,
            "学习输入习惯",
            Some("关掉后不再学，已学的照样用"),
            ("general", "learning"),
            general.learning,
            false,
        )?,
        toggle_info(
            ui,
            page,
            "本地整句模型",
            Some("在本机给整句重新排序，不联网"),
            ("model", "enabled"),
            config.model.enabled,
            false,
        )?,
    ];
    out.push(section_title(ui, theme, "排序")?);
    out.push(group(ui, theme, &rows)?);

    let rows = vec![
        toggle_info(
            ui,
            page,
            "打拼音时也给英文词",
            Some("hello、compa → company"),
            ("general", "english_in_chinese"),
            general.english_in_chinese,
            false,
        )?,
        toggle_with(
            ui,
            page,
            "中文排在英文词前面",
            ("general", "chinese_first"),
            general.chinese_first,
            !general.english_in_chinese,
        )?,
    ];
    out.push(section_title(ui, theme, "英文词")?);
    out.push(group(ui, theme, &rows)?);
    Ok(out)
}

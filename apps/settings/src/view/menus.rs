//! 下拉框弹出的菜单：贴在下拉框下面（下面放不下就往上），选中即写配置。

use qingjian_platform::Scheme;
use render_engine::taffy::NodeId;
use render_engine::{Menu, RenderError, Ui};

use super::Page;
use super::controls::LANGUAGES;
use crate::message::{Message, Popup};

/// 下拉框的本地名，菜单靠它找位置。
pub fn anchor(popup: Popup) -> &'static str {
    match popup {
        Popup::Font => "font",
        Popup::Scheme => "scheme",
        Popup::LearningLanguage => "learning-language",
    }
}

/// 下拉框还没画过（拿不到位置）时不出。
pub fn menu(
    ui: &mut Ui<Message>,
    page: &Page,
    popup: Popup,
    window: (f32, f32),
) -> Result<Option<NodeId>, RenderError> {
    let Some(rect) = ui.rect(ui.key(anchor(popup))) else {
        return Ok(None);
    };
    let general = &page.settings.config.general;
    let (labels, values, selected): (Vec<String>, Vec<String>, Option<usize>) = match popup {
        Popup::Font => {
            let families = page.settings.fonts.as_deref().unwrap_or_default();
            let labels = std::iter::once("系统字体".to_owned())
                .chain(families.iter().cloned())
                .collect();
            let values: Vec<String> = std::iter::once(String::new())
                .chain(families.iter().cloned())
                .collect();
            let selected = values.iter().position(|value| *value == general.font);
            (labels, values, selected)
        }
        Popup::Scheme => {
            let current = general.scheme();
            (
                Scheme::ALL
                    .iter()
                    .map(|scheme| scheme.label().to_owned())
                    .collect(),
                Scheme::ALL
                    .iter()
                    .map(|scheme| scheme.key().to_owned())
                    .collect(),
                Scheme::ALL.iter().position(|scheme| *scheme == current),
            )
        }
        Popup::LearningLanguage => (
            LANGUAGES
                .iter()
                .map(|(_, label)| (*label).to_owned())
                .collect(),
            LANGUAGES
                .iter()
                .map(|(code, _)| (*code).to_owned())
                .collect(),
            LANGUAGES
                .iter()
                .position(|(code, _)| *code == general.learning_language),
        ),
    };
    let key = match popup {
        Popup::Font => "font",
        Popup::Scheme => "scheme",
        Popup::LearningLanguage => "learning_language",
    };
    let items: Vec<&str> = labels.iter().map(String::as_str).collect();
    Menu::new(&items, selected, rect, window)
        .build(
            ui,
            &page.theme,
            "menu",
            move |index| Message::set("general", key, values[index].clone()),
            Message::CloseMenu,
        )
        .map(Some)
}

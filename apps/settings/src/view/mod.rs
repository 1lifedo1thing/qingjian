//! 视图：竹青侧栏（logo、搜索、线性图标的导航、竹节标记）+ 右边当前页的设置项，整页可滚。

mod candidates;
mod sidebar;
mod themes;

use render_engine::taffy::{Dimension, FlexDirection, LengthPercentage, NodeId, Rect, Size, Style};
use render_engine::{
    Interact, KeyCode, Menu, RenderError, ScrollArea, TITLEBAR, Ui, Visual, WidgetTheme,
};

use crate::message::Message;
use crate::palette::{DARK, LIGHT, Palette};
use crate::settings::Settings;

pub const PAGES: [&str; 11] = [
    "通用",
    "候选窗口",
    "快捷键",
    "自定义短语",
    "模糊音",
    "词库",
    "辅码",
    "云服务",
    "统计",
    "高级",
    "关于",
];

/// 右边内容区左右的边距（点）。
const CONTENT_PADDING: f32 = 36.0;

/// 建一帧要用的东西。
pub struct Page<'a> {
    pub settings: &'a Settings,

    pub theme: WidgetTheme,

    pub palette: &'a Palette,

    pub scale: f32,

    /// 右边内容区扣掉左右边距后的宽度（像素）。
    pub content_width: f32,
}

impl Page<'_> {
    pub fn px(&self, points: f32) -> f32 {
        points * self.scale
    }

    pub fn gap(&self, points: f32) -> Size<LengthPercentage> {
        let value = LengthPercentage::length(self.px(points));
        Size {
            width: value,
            height: value,
        }
    }
}

pub fn view(
    ui: &mut Ui<Message>,
    settings: &Settings,
    scale: f32,
    (width, height): (u32, u32),
) -> Result<NodeId, RenderError> {
    let palette = if settings.system_dark { &DARK } else { &LIGHT };
    let page = Page {
        settings,
        theme: WidgetTheme::new(palette.widgets, scale),
        palette,
        scale,
        content_width: width as f32 - (sidebar::WIDTH + CONTENT_PADDING * 2.0) * scale,
    };
    let sidebar = sidebar::sidebar(ui, &page)?;
    let content = content(ui, &page)?;
    let mut children = vec![sidebar, content];
    if settings.font_menu
        && let Some(menu) = font_menu(ui, &page, (width as f32, height as f32))?
    {
        children.push(menu);
    }
    let root = ui.scene().node(
        Style {
            size: Size {
                width: Dimension::length(width as f32),
                height: Dimension::length(height as f32),
            },
            flex_direction: FlexDirection::Row,
            ..Style::default()
        },
        Visual::solid(palette.window, 0.0),
        &children,
    )?;
    let key = ui.key("root");
    let menu_open = settings.font_menu;
    ui.interact(
        root,
        key,
        Interact::new().on_key(move |press| {
            if menu_open && press.key == KeyCode::Escape {
                return Some(Message::CloseMenu);
            }
            let close = press.key == KeyCode::Escape
                || (press.modifiers.command && press.key == KeyCode::Character("w".to_owned()));
            close.then_some(Message::Quit)
        }),
    );
    Ok(root)
}

/// 字体下拉菜单：贴在字体下拉框下面，第一项是系统字体。下拉框还没画过（拿不到位置）时不出。
fn font_menu(
    ui: &mut Ui<Message>,
    page: &Page,
    window: (f32, f32),
) -> Result<Option<NodeId>, RenderError> {
    let Some(anchor) = ui.rect(ui.key("font")) else {
        return Ok(None);
    };
    let families = page.settings.fonts.as_deref().unwrap_or_default();
    let mut items: Vec<&str> = vec!["系统字体"];
    items.extend(families.iter().map(String::as_str));
    let current = &page.settings.config.general.font;
    let selected = if current.is_empty() {
        Some(0)
    } else {
        families
            .iter()
            .position(|name| name == current)
            .map(|index| index + 1)
    };
    let picks: Vec<Option<String>> = std::iter::once(None)
        .chain(families.iter().cloned().map(Some))
        .collect();
    Menu::new(&items, selected, anchor, window)
        .build(
            ui,
            &page.theme,
            "font-menu",
            |index| Message::PickFont(picks[index].clone()),
            Message::CloseMenu,
        )
        .map(Some)
}

fn content(ui: &mut Ui<Message>, page: &Page) -> Result<NodeId, RenderError> {
    let children = if page.settings.page == 1 {
        candidates::candidates(ui, page)?
    } else {
        Vec::new()
    };
    let column = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            gap: page.gap(10.0),
            padding: Rect {
                left: LengthPercentage::length(page.px(CONTENT_PADDING)),
                right: LengthPercentage::length(page.px(CONTENT_PADDING)),
                top: LengthPercentage::length(page.px(TITLEBAR + 12.0)),
                bottom: LengthPercentage::length(page.px(28.0)),
            },
            ..Style::default()
        },
        Visual::Group,
        &children,
    )?;
    // 每页各记各的滚动位置
    ScrollArea::new(Style {
        flex_grow: 1.0,
        ..Style::default()
    })
    .build(ui, &page.theme, ("content", page.settings.page), column)
}

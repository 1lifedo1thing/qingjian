//! 「中英文」页：英文模式（开关、切换键、英文候选、Shift + 字母），标点，悬浮状态条。

use qingjian_platform::{DEFAULT_ENGLISH_CANDIDATES_ON, ShiftLetter, SwitchKey};
use render_engine::taffy::{FlexDirection, NodeId, Style};
use render_engine::{Checkbox, RenderError, SettingRow, Ui, Visual, group, section_title};

use super::Page;
use super::controls::{choice, toggle, toggle_info};
use crate::message::Message;

/// 全角 / 半角两档，配置里是布尔。
const WIDTHS: [bool; 2] = [true, false];

pub fn language(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let config = &page.settings.config;
    let general = &config.general;
    let mut out = Vec::new();

    let mut rows = vec![toggle_info(
        ui,
        page,
        "英文模式",
        Some("关掉后固定中文，要英文就切到别的输入法"),
        ("general", "english_mode"),
        general.english_mode,
        false,
    )?];
    // macOS 的切换键是 Caps Lock，由系统决定
    if cfg!(windows) {
        rows.push(switch_keys(ui, page)?);
    }
    let only_in_apps = !config.apps.english_candidates_everywhere();
    let apps = if only_in_apps {
        vec!["*".to_owned()]
    } else {
        DEFAULT_ENGLISH_CANDIDATES_ON
            .iter()
            .map(|&app| app.to_owned())
            .collect()
    };
    let only = Checkbox::new(only_in_apps, "只在写字的应用里给")
        .disabled(!general.english_candidates)
        .build(
            ui,
            theme,
            "english-only-in-apps",
            Message::SetList("apps", "english_candidates_on", apps),
        )?;
    let switch = render_engine::Switch::new(general.english_candidates).build(
        ui,
        theme,
        "english-candidates",
        Message::set("general", "english_candidates", !general.english_candidates),
    )?;
    rows.push(
        SettingRow::new("英文模式也给候选")
            .info("Tab 或方向键选词，不选时跟直接打字一样；下面勾上就只在浏览器、聊天、文档类应用里给")
            .control(switch)
            .below(only)
            .build(ui, theme)?,
    );
    let shift = choice(
        ui,
        page,
        "shift-letter",
        &ShiftLetter::ALL,
        general.shift_letter,
        ShiftLetter::label,
        |value| Message::set("general", "shift_letter", value.key()),
    )?;
    rows.push(
        SettingRow::new("Shift + 字母")
            .info("进组句才能打出「C盘」")
            .control(shift)
            .build(ui, theme)?,
    );
    out.push(section_title(ui, theme, "英文")?);
    out.push(group(ui, theme, &rows)?);

    let chinese = choice(
        ui,
        page,
        "full-width",
        &WIDTHS,
        general.full_width_punctuation,
        width_label,
        |value| Message::set("general", "full_width_punctuation", value),
    )?;
    let mut rows = vec![
        SettingRow::new("中文标点")
            .control(chinese)
            .build(ui, theme)?,
    ];
    // macOS 英文模式一律半角
    if cfg!(windows) {
        let english = choice(
            ui,
            page,
            "english-full-width",
            &[false, true],
            general.english_full_width_punctuation,
            width_label,
            |value| Message::set("general", "english_full_width_punctuation", value),
        )?;
        rows.push(
            SettingRow::new("英文标点")
                .control(english)
                .build(ui, theme)?,
        );
    }
    rows.push(toggle_info(
        ui,
        page,
        "标点先上屏候选",
        Some("nihao, 一气打完「你好，」"),
        ("general", "punct_commits"),
        general.punct_commits,
        false,
    )?);
    out.push(section_title(ui, theme, "标点")?);
    out.push(group(ui, theme, &rows)?);

    if cfg!(windows) {
        let status = toggle(
            ui,
            page,
            "悬浮状态条",
            ("status_bar", "enabled"),
            config.status_bar.enabled,
        )?;
        out.push(section_title(ui, theme, "状态条")?);
        out.push(group(ui, theme, &[status])?);
    }
    Ok(out)
}

fn width_label(full: bool) -> &'static str {
    if full { "全角" } else { "半角" }
}

/// 中英切换键可以多选，一个都不勾就只剩按钮能切。
fn switch_keys(ui: &mut Ui<Message>, page: &Page) -> Result<NodeId, RenderError> {
    let keys = page.settings.config.shortcut.switch_mode;
    let mut boxes = Vec::new();
    for key in SwitchKey::ALL {
        let on = keys.contains(key);
        let values = keys
            .with(key, !on)
            .config_values()
            .into_iter()
            .map(str::to_owned)
            .collect();
        boxes.push(Checkbox::new(on, key.label()).build(
            ui,
            &page.theme,
            ("switch-key", key.key()),
            Message::SetList("shortcut", "switch_mode", values),
        )?);
    }
    let column = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            align_items: Some(render_engine::taffy::AlignItems::FLEX_START),
            gap: page.gap(8.0),
            ..Style::default()
        },
        Visual::Group,
        &boxes,
    )?;
    SettingRow::new("切换键")
        .control(column)
        .build(ui, &page.theme)
}

//! 「高级」页：配置文件与数据目录，日志，输入日志。

use qingjian_platform::LogLevel;
use render_engine::taffy::{FlexDirection, NodeId, Style};
use render_engine::{
    Button, RenderError, SettingRow, Switch, Ui, Visual, group, paragraph, section_title,
};

use super::Page;
use super::controls::toggle_info;
use crate::action::Action;
use crate::message::Message;

pub fn advanced(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let general = &page.settings.config.general;
    let mut out = Vec::new();

    let buttons = actions(
        ui,
        page,
        &[
            ("在编辑器中打开", Action::OpenConfig),
            ("打开数据目录", Action::OpenDataDir),
        ],
    )?;
    let row = SettingRow::new("配置文件")
        .control(buttons)
        .build(ui, theme)?;
    out.push(section_title(ui, theme, "配置")?);
    out.push(group(ui, theme, &[row])?);

    let verbose = general.log_level == LogLevel::Debug;
    let next = if verbose {
        LogLevel::Info
    } else {
        LogLevel::Debug
    };
    let switch = Switch::new(verbose).build(
        ui,
        theme,
        "verbose-log",
        Message::set("general", "log_level", next.key()),
    )?;
    let buttons = actions(
        ui,
        page,
        &[
            ("打开日志目录", Action::OpenLogDir),
            ("打包到桌面", Action::ExportLogs),
        ],
    )?;
    let rows = vec![
        SettingRow::new("详细日志")
            .info("会记下敲的内容，排查完关掉")
            .control(switch)
            .build(ui, theme)?,
        with_notice(
            ui,
            page,
            SettingRow::new("日志文件").control(buttons),
            "logs",
        )?,
    ];
    out.push(section_title(ui, theme, "日志")?);
    out.push(group(ui, theme, &rows)?);

    let armed = page.settings.armed == Some(Action::ClearInputLog);
    let mut clear = Button::new(if armed {
        "再点一次清空"
    } else {
        "清空"
    });
    if armed {
        clear = clear.primary();
    }
    let clear = clear.build(
        ui,
        theme,
        "clear-input-log",
        Message::Run(Action::ClearInputLog),
    )?;
    let rows = vec![
        toggle_info(
            ui,
            page,
            "记录输入日志",
            Some("只写在这台电脑上，用来评测候选排序"),
            ("general", "input_log"),
            general.input_log,
            false,
        )?,
        with_notice(
            ui,
            page,
            SettingRow::new("清空输入日志").control(clear),
            "input-log",
        )?,
    ];
    out.push(section_title(ui, theme, "输入日志")?);
    out.push(group(ui, theme, &rows)?);
    Ok(out)
}

/// 一排按钮，各触发一个动作。
pub fn actions(
    ui: &mut Ui<Message>,
    page: &Page,
    buttons: &[(&str, Action)],
) -> Result<NodeId, RenderError> {
    let mut nodes = Vec::new();
    for &(label, action) in buttons {
        nodes.push(Button::new(label).build(
            ui,
            &page.theme,
            ("action", label),
            Message::Run(action),
        )?);
    }
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            gap: page.gap(8.0),
            ..Style::default()
        },
        Visual::Group,
        &nodes,
    )
}

/// 这一行刚做完事时，在控件下面挂上那句话。
pub fn with_notice(
    ui: &mut Ui<Message>,
    page: &Page,
    row: SettingRow,
    slot: &str,
) -> Result<NodeId, RenderError> {
    let mut row = row;
    if let Some((owner, text)) = &page.settings.notice
        && *owner == slot
    {
        row = row.below(paragraph(ui, &page.theme, text, None)?);
    }
    row.build(ui, &page.theme)
}

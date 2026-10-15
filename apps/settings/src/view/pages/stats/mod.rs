//! 「统计」页：今天、最近 7 天、累计三张数字卡，累计跟一本书比；词汇（学习语言的译词见过、看熟、上屏过，有等级表时按级列）；
//! 云服务的请求次数与 Token 用量（Windows）。

use bamboo_slip::taffy::{
    AlignItems, Dimension, FlexDirection, JustifyContent, LengthPercentage, NodeId, Rect, Size,
    Style,
};
use bamboo_slip::{
    BoxPaint, Fill, RenderError, SettingRow, Ui, Visual, group, paragraph, section_title,
};
use qingjian_core::{Usage, VocabularySummary};
use qingjian_predict::ApiUsageStats;

use crate::message::Message;
use crate::stats::{group_digits, hanzi_text, scale_line};
use crate::support::Feature;
use crate::view::Page;

/// 等级表的四列。
const LEVEL_COLUMNS: [&str; 4] = ["词表", "见过", "看熟", "上屏过"];

pub fn stats(ui: &mut Ui<Message>, page: &Page) -> Result<Vec<NodeId>, RenderError> {
    let theme = &page.theme;
    let Some(stats) = &page.settings.stats else {
        return Ok(Vec::new());
    };
    let usage = &stats.usage;
    let mut out = Vec::new();

    let cards = [
        card(ui, page, "今天", &usage.today)?,
        card(ui, page, "最近 7 天", &usage.week)?,
        card(ui, page, "累计", &usage.total)?,
    ];
    let cards = ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            gap: page.gap(10.0),
            ..Style::default()
        },
        Visual::Group,
        &cards,
    )?;
    out.push(section_title(ui, theme, "输入量")?);
    out.push(cards);
    let scale = scale_line(usage.total.hanzi);
    let mut row = SettingRow::new(&scale).info(
        "数的是上屏的文字：选一个词算一个中文词，整句按词切开数；英文候选、原样上屏的英文词与英文译词算英文词。只在这台电脑上数",
    );
    let since;
    if let Some(date) = &usage.since {
        since = format!("自 {date} 起，有输入的 {} 天", usage.days);
        row = row.detail(&since);
    }
    let row = row.build(ui, theme)?;
    out.push(group(ui, theme, &[row])?);

    match &stats.vocabulary {
        Some((language, vocabulary)) => {
            out.push(section_title(ui, theme, &format!("词汇（{language}）"))?);
            let mut rows = vec![note(ui, page, &vocabulary_line(vocabulary))?];
            if !vocabulary.levels.is_empty() {
                rows.push(levels(ui, page, vocabulary)?);
            }
            out.push(group(ui, theme, &rows)?);
        }
        None => {
            out.push(section_title(ui, theme, "词汇")?);
            let off = note(ui, page, "学习语言关着，不统计词汇")?;
            out.push(group(ui, theme, &[off])?);
        }
    }

    if !Feature::ApiUsage.here() {
        return Ok(out);
    }
    out.push(section_title(ui, theme, "云服务用量")?);
    let rows = match &stats.api {
        Some(api) => vec![note(ui, page, &requests_line(api))?, tokens(ui, page, api)?],
        None => vec![note(
            ui,
            page,
            "用量记录读不出来，原有记录还在，稍后再进这一页看",
        )?],
    };
    out.push(group(ui, theme, &rows)?);
    Ok(out)
}

/// 「请求 12 次，收到 11 次，失败 1 次」，还在等的、没报用量的另说。
fn requests_line(api: &ApiUsageStats) -> String {
    if api.requests == 0 {
        return "还没有发过请求：云联想、译词补充、翻译选中文字和「测试连接」都算".to_owned();
    }
    let mut line = format!(
        "请求 {} 次，收到 {} 次，失败 {} 次",
        group_digits(api.requests),
        group_digits(api.responses),
        group_digits(api.failures)
    );
    let unresolved = api.unresolved_requests();
    if unresolved > 0 {
        line += &format!("，还没有结果 {} 次", group_digits(unresolved));
    }
    if api.unreported_usage > 0 {
        line += &format!(
            "；{} 次服务没报用量，不在下面的数里",
            group_digits(api.unreported_usage)
        );
    }
    line
}

/// Token 表：表头一行，累计一行。缓存输入已经算在输入里。
fn tokens(ui: &mut Ui<Message>, page: &Page, api: &ApiUsageStats) -> Result<NodeId, RenderError> {
    let header = ["输入", "输出", "合计", "其中缓存"].map(str::to_owned);
    let values = [
        api.input_tokens,
        api.output_tokens,
        api.total_tokens,
        api.cached_input_tokens,
    ]
    .map(group_digits);
    let rows = [
        level_row(ui, page, "Token", &header, true)?,
        level_row(ui, page, "累计", &values, false)?,
    ];
    let space = LengthPercentage::length(page.px(12.0));
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            gap: page.gap(6.0),
            padding: Rect {
                top: space,
                bottom: space,
                left: LengthPercentage::length(0.0),
                right: LengthPercentage::length(0.0),
            },
            ..Style::default()
        },
        Visual::Group,
        &rows,
    )
}

/// 一张数字卡：标题、大字的汉字数，下面两行小字。
fn card(
    ui: &mut Ui<Message>,
    page: &Page,
    title: &str,
    usage: &Usage,
) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    let colors = &theme.colors;
    let title = text(ui, page, title, theme.small + 1.0, 600, colors.secondary)?;
    let hanzi = text(ui, page, &hanzi_text(usage.hanzi), 22.0, 600, colors.text)?;
    let words = text(
        ui,
        page,
        &format!(
            "中文词 {} · 英文词 {}",
            group_digits(usage.words),
            group_digits(usage.english_words)
        ),
        theme.small,
        400,
        colors.secondary,
    )?;
    let commits = text(
        ui,
        page,
        &format!("上屏 {} 次", group_digits(usage.commits)),
        theme.small,
        400,
        colors.secondary,
    )?;
    let space = LengthPercentage::length(page.px(14.0));
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            flex_basis: Dimension::length(0.0),
            gap: page.gap(4.0),
            padding: Rect {
                left: LengthPercentage::length(page.px(16.0)),
                right: space,
                top: space,
                bottom: space,
            },
            ..Style::default()
        },
        Visual::Box(BoxPaint {
            fill: Some(Fill::Solid(colors.surface)),
            border: Some((page.px(1.0), colors.line)),
            radius: page.px(theme.radius * 1.5),
        }),
        &[title, hanzi, words, commits],
    )
}

/// 「见过 120 个词，看熟 80 个；上屏过 60 个，直接打出过 3 个；本周新见 12 个」
fn vocabulary_line(summary: &VocabularySummary) -> String {
    if summary.seen == 0 {
        return "还没见过译词：打中文时候选右侧的译词就是词汇的来源".to_owned();
    }
    format!(
        "见过 {} 个词，看熟 {} 个；上屏过 {} 个，直接打出过 {} 个；本周新见 {} 个",
        group_digits(summary.seen),
        group_digits(summary.familiar),
        group_digits(summary.committed),
        group_digits(summary.used),
        group_digits(summary.new_this_week)
    )
}

/// 按级的表：表头一行，一级一行。
fn levels(
    ui: &mut Ui<Message>,
    page: &Page,
    vocabulary: &VocabularySummary,
) -> Result<NodeId, RenderError> {
    let mut rows = Vec::new();
    let header: Vec<String> = LEVEL_COLUMNS
        .iter()
        .map(|title| (*title).to_owned())
        .collect();
    rows.push(level_row(ui, page, "等级", &header, true)?);
    for level in &vocabulary.levels {
        let values = [level.total, level.seen, level.familiar, level.committed].map(group_digits);
        rows.push(level_row(ui, page, &level.name, &values, false)?);
    }
    let space = LengthPercentage::length(page.px(12.0));
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Column,
            gap: page.gap(6.0),
            padding: Rect {
                top: space,
                bottom: space,
                left: LengthPercentage::length(0.0),
                right: LengthPercentage::length(0.0),
            },
            ..Style::default()
        },
        Visual::Group,
        &rows,
    )
}

fn level_row(
    ui: &mut Ui<Message>,
    page: &Page,
    name: &str,
    values: &[String],
    header: bool,
) -> Result<NodeId, RenderError> {
    let theme = &page.theme;
    let colors = &theme.colors;
    let (weight, color) = if header {
        (600, colors.secondary)
    } else {
        (400, colors.text)
    };
    let mut cells = vec![cell(ui, page, name, weight, color, false)?];
    for value in values {
        cells.push(cell(ui, page, value, weight, color, true)?);
    }
    ui.scene().node(
        Style {
            flex_direction: FlexDirection::Row,
            ..Style::default()
        },
        Visual::Group,
        &cells,
    )
}

/// 表格的一格：五格平分，数字靠右。
fn cell(
    ui: &mut Ui<Message>,
    page: &Page,
    value: &str,
    weight: u16,
    color: bamboo_slip::Color,
    number: bool,
) -> Result<NodeId, RenderError> {
    let label = text(ui, page, value, page.theme.small + 1.0, weight, color)?;
    ui.scene().node(
        Style {
            flex_grow: 1.0,
            flex_basis: Dimension::length(0.0),
            flex_direction: FlexDirection::Row,
            justify_content: Some(if number {
                JustifyContent::FLEX_END
            } else {
                JustifyContent::FLEX_START
            }),
            align_items: Some(AlignItems::CENTER),
            size: Size {
                width: Dimension::auto(),
                height: Dimension::auto(),
            },
            ..Style::default()
        },
        Visual::Group,
        &[label],
    )
}

/// 卡片里的一段话。
fn note(ui: &mut Ui<Message>, page: &Page, text: &str) -> Result<NodeId, RenderError> {
    let note = paragraph(ui, &page.theme, text, Some(page.theme.colors.text))?;
    let space = LengthPercentage::length(page.px(12.0));
    ui.scene().node(
        Style {
            padding: Rect {
                top: space,
                bottom: space,
                left: LengthPercentage::length(0.0),
                right: LengthPercentage::length(0.0),
            },
            ..Style::default()
        },
        Visual::Group,
        &[note],
    )
}

fn text(
    ui: &mut Ui<Message>,
    page: &Page,
    text: &str,
    points: f32,
    weight: u16,
    color: bamboo_slip::Color,
) -> Result<NodeId, RenderError> {
    ui.scene().node(
        Style::default(),
        Visual::Text {
            text: text.to_owned(),
            style: page.theme.text(points, weight, color),
        },
        &[],
    )
}

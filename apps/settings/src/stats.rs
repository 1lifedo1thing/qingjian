//! 统计页的数：输入量（`usage.tsv`）与词汇（`user-vocab.tsv`，有等级表时按级分）。输入法记、这里只读，进统计页时读一次。
//! 还有几种给人看的写法：千位分隔、万 / 亿字、跟一本书比。

use std::path::PathBuf;

use qingjian_core::{Language, UsageSummary, VocabularySummary, book_scale};
use qingjian_learning::{UsageStats, VocabularyBook};
use qingjian_platform::GeneralConfig;
use qingjian_translate::LevelTable;

use crate::paths;

pub struct Stats {
    pub usage: UsageSummary,

    /// 学习语言的名字与词汇汇总；学习语言关着为 `None`。
    pub vocabulary: Option<(&'static str, VocabularySummary)>,
}

impl Stats {
    pub fn load(general: &GeneralConfig) -> Self {
        let today = jiff::Zoned::now().date();
        let dir = paths::user_dir().unwrap_or_default();
        let usage = UsageStats::open(dir.join("usage.tsv")).summary_on(today);
        let vocabulary = (!general.learning_language_off()).then(|| {
            let language = general
                .learning_language
                .parse::<Language>()
                .unwrap_or(Language::English);
            let mut book = VocabularyBook::open(dir.join("user-vocab.tsv"));
            if let Some(path) = level_table(language)
                && let Ok(table) = LevelTable::from_path(&path)
            {
                book = book.with_levels(language, table);
            }
            (language_name(language), book.summary_on(language, today))
        });
        Self { usage, vocabulary }
    }
}

fn language_name(language: Language) -> &'static str {
    match language {
        Language::Chinese => "中文",
        Language::English => "英语",
        Language::Japanese => "日语",
        Language::Spanish => "西班牙语",
        Language::Vietnamese => "越南语",
    }
}

/// 词汇等级表：macOS 在 `Qingjian.app/Contents/Resources/`，其他平台在随包根的 `assets/levels/`。
fn level_table(language: Language) -> Option<PathBuf> {
    let name = format!("levels-{}.tsv", language.code());
    let found = crate::paths::mac_resources()
        .into_iter()
        .map(|dir| dir.join(&name))
        .find(|path| path.is_file());
    if found.is_some() {
        return found;
    }
    crate::paths::bundled(&format!("assets/levels/{name}"))
}

/// 千位分隔：`12345` → `12,345`。
pub fn group_digits(value: u64) -> String {
    let digits = value.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// 字数：万以下写整数（`2,500 字`），万以上一位小数（`12.3 万字`），亿以上同理。
pub fn hanzi_text(value: u64) -> String {
    const WAN: f64 = 10_000.0;
    const YI: f64 = 100_000_000.0;
    let value_f = value as f64;
    if value_f >= YI {
        format!("{} 亿字", trim_decimal(value_f / YI))
    } else if value_f >= WAN {
        format!("{} 万字", trim_decimal(value_f / WAN))
    } else {
        format!("{} 字", group_digits(value))
    }
}

/// 「累计 12.3 万字，约等于 1.7 本《活着》（约 12 万字）」；一个字没有时不比。
pub fn scale_line(hanzi: u64) -> String {
    if hanzi == 0 {
        return "还没有输入记录，打几个字再来看".to_owned();
    }
    let (book, ratio) = book_scale(hanzi);
    format!(
        "累计 {}，约等于 {} 本《{}》（约 {}）",
        hanzi_text(hanzi),
        format_ratio(ratio),
        book.title,
        hanzi_text(book.hanzi)
    )
}

/// 一位小数，`.0` 去掉。
fn trim_decimal(value: f64) -> String {
    let text = format!("{value:.1}");
    text.strip_suffix(".0").map_or(text.clone(), str::to_owned)
}

/// 倍数：不到 10 倍留一位小数，再多就取整。
fn format_ratio(ratio: f64) -> String {
    if ratio >= 10.0 {
        format!("{}", ratio.round() as u64)
    } else {
        trim_decimal(ratio)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_read_in_chinese_units() {
        assert_eq!(group_digits(1_234_567), "1,234,567");
        assert_eq!(hanzi_text(9_999), "9,999 字");
        assert_eq!(hanzi_text(123_000), "12.3 万字");
        assert_eq!(hanzi_text(250_000_000), "2.5 亿字");
        assert!(scale_line(204_000).starts_with("累计 20.4 万字，约等于"));
    }
}

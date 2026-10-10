//! 词库与码表的清单：随包的、用户导入的各有哪些，名字、条数、许可证。进词库页（或导入、移除）时扫一次，不每帧读盘。

use std::path::{Path, PathBuf};

use qingjian_dictionary::{Dictionary, aux_code_table_info};
use qingjian_platform::{code_tables, extra_dictionaries};

use crate::paths;

/// 词库还是码表。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Dictionary,
    CodeTable,
}

impl Kind {
    /// 导入、移除的结果显示在哪一行下面。
    pub fn slot(self) -> &'static str {
        match self {
            Self::Dictionary => "dicts",
            Self::CodeTable => "codes",
        }
    }

    /// 配置里记关掉了哪些的那一项。
    pub fn disabled_key(self) -> (&'static str, &'static str) {
        match self {
            Self::Dictionary => ("dictionaries", "disabled"),
            Self::CodeTable => ("aux_code", "disabled"),
        }
    }

    /// 用户导入的放在哪。
    pub fn user_dir(self) -> Option<PathBuf> {
        let name = match self {
            Self::Dictionary => "dicts",
            Self::CodeTable => "codes",
        };
        paths::user_dir().map(|dir| dir.join(name))
    }

    fn bundled_dir(self) -> Option<PathBuf> {
        let name = match self {
            Self::Dictionary => "dicts",
            Self::CodeTable => "codes",
        };
        bundled_dir(name)
    }

    fn list(self, dir: &Path) -> Vec<(String, PathBuf)> {
        match self {
            Self::Dictionary => extra_dictionaries::list(dir),
            Self::CodeTable => code_tables::list(dir),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    /// 文件名（不含扩展名），配置里按它开关。
    pub stem: String,

    pub name: String,

    pub entries: usize,

    pub license: String,

    /// 打不开的坏文件：照样列出来好让人移除。
    pub broken: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Library {
    pub domains: Vec<Entry>,

    pub dictionaries: Vec<Entry>,

    pub bundled_codes: Vec<Entry>,

    pub codes: Vec<Entry>,
}

impl Library {
    pub fn scan() -> Self {
        let scan = |kind: Kind, dir: Option<PathBuf>| -> Vec<Entry> {
            dir.map(|dir| {
                kind.list(&dir)
                    .into_iter()
                    .map(|(stem, path)| entry(kind, stem, &path))
                    .collect()
            })
            .unwrap_or_default()
        };
        Self {
            domains: scan(Kind::Dictionary, Kind::Dictionary.bundled_dir()),
            dictionaries: scan(Kind::Dictionary, Kind::Dictionary.user_dir()),
            bundled_codes: scan(Kind::CodeTable, Kind::CodeTable.bundled_dir()),
            codes: scan(Kind::CodeTable, Kind::CodeTable.user_dir()),
        }
    }
}

fn entry(kind: Kind, stem: String, path: &Path) -> Entry {
    match kind {
        Kind::Dictionary => match Dictionary::from_path(path) {
            Ok(dictionary) => {
                let metadata = dictionary.metadata();
                Entry {
                    name: metadata.map_or_else(|| stem.clone(), |m| m.name.clone()),
                    license: metadata.map_or_else(String::new, |m| m.license.clone()),
                    entries: dictionary.len(),
                    broken: false,
                    stem,
                }
            }
            Err(error) => {
                tracing::warn!(path = %path.display(), %error, "词库打不开");
                Entry {
                    name: stem.clone(),
                    stem,
                    entries: 0,
                    license: String::new(),
                    broken: true,
                }
            }
        },
        Kind::CodeTable => {
            let info = aux_code_table_info(path);
            Entry {
                stem,
                name: info.name,
                entries: info.entries,
                license: info.license,
                broken: info.broken,
            }
        }
    }
}

/// 随包的 `dicts` / `codes` 目录，与各平台输入法读的同一处：
/// macOS 在 `Qingjian.app/Contents/Resources/`（设置程序将来也装在包里，先找自己所在的包，再找装好的输入法），
/// 其他平台在随包根的 `data/generated/`；都找不到时用开发布局（仓库的 `data/generated/`）。
fn bundled_dir(name: &str) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if cfg!(target_os = "macos") {
        if let Some(contents) = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent()?.parent().map(Path::to_owned))
        {
            candidates.push(contents.join("Resources").join(name));
        }
        let app = "Library/Input Methods/Qingjian.app/Contents/Resources";
        if let Some(home) = std::env::var_os("HOME") {
            candidates.push(PathBuf::from(home).join(app).join(name));
        }
        candidates.push(PathBuf::from("/").join(app).join(name));
    } else if let Some(root) = std::env::var_os("QINGJIAN_RESOURCES") {
        candidates.push(PathBuf::from(root).join("data/generated").join(name));
    }
    candidates.into_iter().find(|dir| dir.is_dir()).or_else(|| {
        qingjian_platform::resources::bundled_resource(&format!("data/generated/{name}"))
    })
}

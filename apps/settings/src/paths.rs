//! 各平台的用户目录：配置、数据、主题、日志放在哪，与各平台输入法读写的位置一致。
//! Windows 用 `qingjian_platform::dirs`；macOS 照 `apps/macos/src/app/paths.rs`，Linux 照 `apps/linux/server/src/paths.rs`。

use std::path::PathBuf;

/// 用户数据目录：学习数据、统计、输入日志都在这里。
pub fn user_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        qingjian_platform::dirs::user_dir()
    } else if cfg!(target_os = "macos") {
        home().map(|home| home.join("Library/Application Support/Qingjian"))
    } else {
        xdg("XDG_DATA_HOME", ".local/share")
    }
}

pub fn config_path() -> Option<PathBuf> {
    if cfg!(any(windows, target_os = "macos")) {
        user_dir().map(|dir| dir.join("config.toml"))
    } else {
        xdg("XDG_CONFIG_HOME", ".config").map(|dir| dir.join("config.toml"))
    }
}

pub fn themes_dir() -> Option<PathBuf> {
    user_dir().map(|dir| dir.join("themes"))
}

/// 输入法的运行日志目录。
pub fn log_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        qingjian_platform::dirs::log_dir()
    } else if cfg!(target_os = "macos") {
        home().map(|home| home.join("Library/Logs/Qingjian"))
    } else {
        xdg("XDG_STATE_HOME", ".local/state").map(|dir| dir.join("logs"))
    }
}

/// 输入日志（每次上屏一行）。
pub fn input_log() -> Option<PathBuf> {
    user_dir().map(|dir| dir.join("input-log.jsonl"))
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// XDG 目录下的 `qingjian`：环境变量不是绝对路径时按规范回到主目录下的缺省位置。
fn xdg(name: &str, fallback: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| home().map(|home| home.join(fallback)))
        .map(|dir| dir.join("qingjian"))
}

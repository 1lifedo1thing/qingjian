//! 青简设置程序（新）：三平台共用、画在 render-engine 上，读写输入法同一份配置文件，改动即时生效。
//!
//! - `qingjian-settings-ui`：开窗口，读写用户配置（`paths::config_path`）。
//! - `--config <文件>`：换一份配置文件读写（试用时不碰真配置）。
//! - `--png <目录>`：不开窗口，拷一份配置到目录里，按脚本点几下，每步存一张截图。

mod action;
mod cloud;
mod fonts;
mod icons;
mod message;
mod page_id;
mod palette;
mod paths;
mod previews;
mod settings;
mod shortcut;
mod snapshot;
mod update;
mod view;

use std::path::PathBuf;

use render_engine::{WindowOptions, run_window};

use settings::Settings;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let mut config = paths::config_path();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--config" => config = args.next().map(PathBuf::from),
            "--png" => {
                let directory = args.next().map(PathBuf::from).ok_or("--png 后面要给目录")?;
                return snapshot::run(config.as_deref(), &directory);
            }
            other => return Err(format!("不认识的参数：{other}").into()),
        }
    }
    let config = config.ok_or("找不到配置文件的位置")?;
    let settings = Settings::load(config)?;
    let mut options = WindowOptions::new("青简设置", (820.0, 620.0));
    options.min_size = (640.0, 420.0);
    run_window(settings, options)?;
    Ok(())
}

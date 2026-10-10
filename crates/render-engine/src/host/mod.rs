//! 窗口宿主（`host` 特性）：winit 开窗收事件、softbuffer 把位图贴到窗口，三个桌面平台一份代码。
//! 系统事件翻成 [`crate::Event`] 交给 [`crate::Runner`]；有动画在播时按显示器的刷新节奏接着要帧，播完就停。
//! macOS 上标题栏透明、内容铺到窗口顶上（保留系统的红绿灯与拖动、缩放），应用自己在顶上留出 [`TITLEBAR`] 的空。

mod blit;
mod error;
mod handler;
mod input;
mod options;

use winit::event_loop::EventLoop;

use crate::app::{App, Runner};
use crate::fonts::FontLibrary;

pub use error::HostError;
pub use options::WindowOptions;

/// 窗口顶上让给系统标题栏按钮的高度（点）：macOS 内容铺到标题栏下面，侧栏要躲开红绿灯；其他平台标题栏在窗口外，不用让。
pub const TITLEBAR: f32 = if cfg!(target_os = "macos") { 28.0 } else { 0.0 };

/// 开窗口跑 `app`，直到窗口关掉或应用要退出。
pub fn run<A: App + 'static>(app: A, options: WindowOptions) -> Result<(), HostError> {
    let library = FontLibrary::system(&options.locale)?;
    let runner = Runner::new(app, library);
    let event_loop = EventLoop::new()?;
    let mut handler = handler::Handler::new(runner, options);
    event_loop.run_app(&mut handler)?;
    match handler.error.take() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

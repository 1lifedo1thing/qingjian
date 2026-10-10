//! 窗口宿主样例：winit 开窗、softbuffer 贴图，画一页能点的设置页，跟随 DPI 与深浅色。
//!
//! - `cargo run --release --example window`：开窗口。侧栏能切页、开关能点，Tab / Shift+Tab 走焦点、空格触发，Esc 退出。
//!   每次重画把建树（含布局）/ 栅格 / 贴图耗时打到标准输出。
//! - `cargo run --release --example window -- --png <目录>`：不开窗口，按脚本发事件，每步存一张截图，外加一张总览 `sheet.png`。

mod demo;
mod frame;
mod page;
mod snapshot;

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use render_engine::{Event, FontLibrary, Interaction, KeyCode, KeyPress, Modifiers, Pixmap, Stage};
use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Theme, Window, WindowId};

use demo::{Demo, Message};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("--png") {
        let directory = args.next().map(PathBuf::from).ok_or("--png 后面要给目录")?;
        return snapshot::run(&directory);
    }
    let library = FontLibrary::system("zh-CN")?;
    let event_loop = EventLoop::new()?;
    let mut app = App {
        stage: Stage::new(library),
        interaction: Interaction::new(),
        demo: Demo::new(false),
        modifiers: Modifiers::default(),
        window: None,
    };
    event_loop.run_app(&mut app)?;
    Ok(())
}

struct App {
    stage: Stage,

    interaction: Interaction<Message>,

    demo: Demo,

    /// 最近一次的修饰键，按键事件本身不带。
    modifiers: Modifiers,

    window: Option<Host>,
}

/// 窗口和它的贴图面；softbuffer 的面要借着窗口，两个都用 `Arc` 持有。
struct Host {
    window: Arc<Window>,

    surface: Surface<Arc<Window>, Arc<Window>>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("render-engine")
            .with_inner_size(LogicalSize::new(780.0, 660.0))
            .with_min_inner_size(LogicalSize::new(480.0, 320.0));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                eprintln!("开窗失败：{error}");
                event_loop.exit();
                return;
            }
        };
        let surface =
            Context::new(window.clone()).and_then(|context| Surface::new(&context, window.clone()));
        match surface {
            Ok(surface) => {
                self.demo.dark = window.theme() == Some(Theme::Dark);
                self.window = Some(Host { window, surface });
            }
            Err(error) => {
                eprintln!("建贴图面失败：{error}");
                event_loop.exit();
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(host) = self.window.as_mut() else {
            return;
        };
        let input = match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                return;
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                host.window.request_redraw();
                return;
            }
            WindowEvent::ThemeChanged(theme) => {
                self.demo.dark = theme == Theme::Dark;
                host.window.request_redraw();
                return;
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = redraw(host, &mut self.stage, &mut self.interaction, &self.demo)
                {
                    eprintln!("重画失败：{error}");
                }
                return;
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                let state = modifiers.state();
                self.modifiers = Modifiers {
                    shift: state.shift_key(),
                    alt: state.alt_key(),
                    command: if cfg!(target_os = "macos") {
                        state.super_key()
                    } else {
                        state.control_key()
                    },
                };
                return;
            }
            WindowEvent::CursorMoved { position, .. } => Event::PointerMoved {
                x: position.x as f32,
                y: position.y as f32,
            },
            WindowEvent::CursorLeft { .. } => Event::PointerLeft,
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => match state {
                ElementState::Pressed => Event::PointerDown,
                ElementState::Released => Event::PointerUp,
            },
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let Some(key) = key_code(&event.logical_key) else {
                    return;
                };
                Event::Key(KeyPress::new(key, self.modifiers))
            }
            _ => return,
        };
        let response = self.interaction.handle(&input);
        let rebuild = response.needs_rebuild();
        for message in response.messages {
            if !self.demo.update(message) {
                event_loop.exit();
                return;
            }
        }
        if rebuild {
            host.window.request_redraw();
        }
    }
}

fn key_code(key: &Key) -> Option<KeyCode> {
    Some(match key {
        Key::Named(NamedKey::Tab) => KeyCode::Tab,
        Key::Named(NamedKey::Enter) => KeyCode::Enter,
        Key::Named(NamedKey::Space) => KeyCode::Space,
        Key::Named(NamedKey::Escape) => KeyCode::Escape,
        Key::Named(NamedKey::ArrowLeft) => KeyCode::ArrowLeft,
        Key::Named(NamedKey::ArrowRight) => KeyCode::ArrowRight,
        Key::Named(NamedKey::ArrowUp) => KeyCode::ArrowUp,
        Key::Named(NamedKey::ArrowDown) => KeyCode::ArrowDown,
        Key::Named(NamedKey::Home) => KeyCode::Home,
        Key::Named(NamedKey::End) => KeyCode::End,
        Key::Character(text) => KeyCode::Character(text.to_string()),
        _ => return None,
    })
}

fn redraw(
    host: &mut Host,
    stage: &mut Stage,
    interaction: &mut Interaction<Message>,
    demo: &Demo,
) -> Result<(), Box<dyn std::error::Error>> {
    let size = host.window.inner_size();
    let (Some(width), Some(height)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
    else {
        return Ok(());
    };
    let scale = host.window.scale_factor() as f32;
    let frame = frame::render(stage, interaction, demo, scale, (width.get(), height.get()))?;

    let start = Instant::now();
    host.surface.resize(width, height)?;
    let mut buffer = host.surface.buffer_mut()?;
    let painted = &frame.painted;
    blit(
        &painted.pixmap,
        (painted.content_x, painted.content_y),
        &mut buffer,
        width.get(),
        height.get(),
    );
    buffer.present()?;
    let shown = start.elapsed();

    println!(
        "{width}×{height} @{scale}：建树 {:.2} ms，栅格 {:.2} ms，贴图 {:.2} ms，共 {:.2} ms",
        ms(frame.build),
        ms(frame.raster),
        ms(shown),
        ms(frame.build + frame.raster + shown),
    );
    Ok(())
}

/// 位图里内容区左上角对齐窗口左上角拷过去；softbuffer 每像素是 `0x00RRGGBB`。
/// 根节点铺了不透明底色，预乘与否结果一样，直接取 RGB。
fn blit(pixmap: &Pixmap, (left, top): (u32, u32), buffer: &mut [u32], width: u32, height: u32) {
    let pixels = pixmap.pixels();
    let stride = pixmap.width();
    for y in 0..height {
        let row = &mut buffer[(y * width) as usize..((y + 1) * width) as usize];
        let source_y = y + top;
        if source_y >= pixmap.height() {
            row.fill(0);
            continue;
        }
        for (x, out) in row.iter_mut().enumerate() {
            let source_x = x as u32 + left;
            *out = if source_x < stride {
                let pixel = pixels[(source_y * stride + source_x) as usize];
                (u32::from(pixel.red()) << 16)
                    | (u32::from(pixel.green()) << 8)
                    | u32::from(pixel.blue())
            } else {
                0
            };
        }
    }
}

fn ms(duration: std::time::Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

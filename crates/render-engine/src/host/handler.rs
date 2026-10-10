//! winit 的事件循环回调：开窗、翻译事件、按需重画。

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Instant;

use softbuffer::{Context, Surface};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Theme, Window, WindowId};

use super::blit::blit;
use super::input::{key_code, modifiers, scroll};
use super::{HostError, WindowOptions};
use crate::app::{App, Runner};
use crate::ui::{Event, KeyPress, Modifiers};

pub(super) struct Handler<A: App> {
    runner: Runner<A>,

    options: WindowOptions,

    /// 最近一次的修饰键，按键事件本身不带。
    modifiers: Modifiers,

    window: Option<Host>,

    /// 中途出的错，事件循环退出后交给调用方。
    pub(super) error: Option<HostError>,
}

/// 窗口和它的贴图面；softbuffer 的面要借着窗口，两个都用 `Arc` 持有。
struct Host {
    window: Arc<Window>,

    surface: Surface<Arc<Window>, Arc<Window>>,
}

impl<A: App> Handler<A> {
    pub(super) fn new(runner: Runner<A>, options: WindowOptions) -> Self {
        Self {
            runner,
            options,
            modifiers: Modifiers::default(),
            window: None,
            error: None,
        }
    }

    fn open(&mut self, event_loop: &ActiveEventLoop) -> Result<(), HostError> {
        let (width, height) = self.options.size;
        let (min_width, min_height) = self.options.min_size;
        let attributes = Window::default_attributes()
            .with_title(self.options.title.clone())
            .with_inner_size(LogicalSize::new(width, height))
            .with_min_inner_size(LogicalSize::new(min_width, min_height));
        #[cfg(target_os = "macos")]
        let attributes = {
            use winit::platform::macos::WindowAttributesExtMacOS;
            attributes
                .with_titlebar_transparent(true)
                .with_title_hidden(true)
                .with_fullsize_content_view(true)
        };
        let window = Arc::new(event_loop.create_window(attributes)?);
        let context = Context::new(window.clone())?;
        let surface = Surface::new(&context, window.clone())?;
        self.runner
            .app_mut()
            .set_system_dark(window.theme() == Some(Theme::Dark));
        self.window = Some(Host { window, surface });
        Ok(())
    }

    /// 画下一帧，返回是否还有动画要接着画。
    fn redraw(&mut self) -> Result<bool, HostError> {
        let Some(host) = self.window.as_mut() else {
            return Ok(false);
        };
        let size = host.window.inner_size();
        let (Some(width), Some(height)) =
            (NonZeroU32::new(size.width), NonZeroU32::new(size.height))
        else {
            return Ok(false);
        };
        let scale = host.window.scale_factor() as f32;
        let start = Instant::now();
        let Some(painted) = self
            .runner
            .frame(scale, (width.get(), height.get()), start)?
        else {
            return Ok(false);
        };
        let painted_at = Instant::now();
        host.surface.resize(width, height)?;
        let mut buffer = host.surface.buffer_mut()?;
        blit(
            &painted.pixmap,
            (painted.content_x, painted.content_y),
            &mut buffer,
            width.get(),
            height.get(),
        );
        buffer.present()?;
        tracing::debug!(
            width = width.get(),
            height = height.get(),
            paint_ms = (painted_at - start).as_secs_f64() * 1000.0,
            blit_ms = painted_at.elapsed().as_secs_f64() * 1000.0,
            "重画"
        );
        Ok(painted.next_frame.is_some())
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: HostError) {
        tracing::error!(%error, "窗口宿主出错，退出");
        self.error = Some(error);
        event_loop.exit();
    }
}

impl<A: App> ApplicationHandler for Handler<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(error) = self.open(event_loop)
        {
            self.fail(event_loop, error);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(window) = self.window.as_ref().map(|host| host.window.clone()) else {
            return;
        };
        let input = match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                return;
            }
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                self.runner.invalidate();
                window.request_redraw();
                return;
            }
            WindowEvent::ThemeChanged(theme) => {
                self.runner.app_mut().set_system_dark(theme == Theme::Dark);
                window.request_redraw();
                return;
            }
            WindowEvent::RedrawRequested => {
                match self.redraw() {
                    // 还有动画在播：按显示器的刷新节奏接着要帧
                    Ok(true) => window.request_redraw(),
                    Ok(false) => {}
                    Err(error) => self.fail(event_loop, error),
                }
                return;
            }
            WindowEvent::ModifiersChanged(state) => {
                self.modifiers = modifiers(state.state());
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
            WindowEvent::MouseWheel { delta, .. } => {
                let (dx, dy) = scroll(delta, window.scale_factor() as f32);
                Event::Scroll { dx, dy }
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                let Some(key) = key_code(&event.logical_key) else {
                    return;
                };
                Event::Key(KeyPress::new(key, self.modifiers))
            }
            _ => return,
        };
        let redraw = self.runner.handle(&input);
        if self.runner.quit_requested() {
            event_loop.exit();
        } else if redraw {
            window.request_redraw();
        }
    }
}

//! 一个应用的一帧一帧：输入事件交给交互层、消息交给应用；要重建时整棵建树、布局、认回交互状态，交给舞台画（与上一帧配对过渡）；
//! 不用重建时只推进在播的动画。

use std::time::{Duration, Instant};

use super::App;
use crate::error::RenderError;
use crate::fonts::FontLibrary;
use crate::stage::{Painted, Stage};
use crate::ui::{Event, Interaction, Ui};

pub struct Runner<A: App> {
    app: A,

    stage: Stage,

    interaction: Interaction<A::Message>,

    /// 状态变了，下一帧要重建树。
    stale: bool,

    /// 应用要退出。
    quit: bool,

    /// 上一次重建（建树加布局）花了多久。
    last_build: Option<Duration>,
}

impl<A: App> Runner<A> {
    pub fn new(mut app: A, library: FontLibrary) -> Self {
        let mut stage = Stage::new(library);
        app.setup(stage.text());
        Self {
            app,
            stage,
            interaction: Interaction::new(),
            stale: true,
            quit: false,
            last_build: None,
        }
    }

    pub fn app(&self) -> &A {
        &self.app
    }

    /// 改了应用状态（不经消息，如系统外观）：下一帧重建。
    pub fn app_mut(&mut self) -> &mut A {
        self.stale = true;
        &mut self.app
    }

    pub fn stage(&mut self) -> &mut Stage {
        &mut self.stage
    }

    pub fn interaction(&self) -> &Interaction<A::Message> {
        &self.interaction
    }

    /// 窗口大小、倍数变了这类：下一帧重建。
    pub fn invalidate(&mut self) {
        self.stale = true;
    }

    pub fn quit_requested(&self) -> bool {
        self.quit
    }

    pub fn last_build(&self) -> Option<Duration> {
        self.last_build
    }

    /// 处理一个输入事件，返回要不要重画。
    pub fn handle(&mut self, event: &Event) -> bool {
        let response = self.interaction.handle(event);
        let rebuild = response.needs_rebuild();
        for message in response.messages {
            if !self.app.update(message) {
                self.quit = true;
            }
        }
        self.stale |= rebuild;
        rebuild
    }

    /// 出下一帧：要重建时重建，否则推进在播的动画；什么都不用画时为 `None`。
    pub fn frame(
        &mut self,
        scale: f32,
        size: (u32, u32),
        now: Instant,
    ) -> Result<Option<Painted>, RenderError> {
        if self.stale {
            return self.rebuild(scale, size, now).map(Some);
        }
        self.stage.tick_at(now)
    }

    /// 按当前状态整棵重建并画。
    pub fn rebuild(
        &mut self,
        scale: f32,
        size: (u32, u32),
        now: Instant,
    ) -> Result<Painted, RenderError> {
        let start = Instant::now();
        self.stale = false;
        // 布局挪动可能让指针换了元素，悬停状态变了要按新状态再建一次；再变也不追了，下次事件会改正
        let mut attempts = 0;
        let (scene, root, content) = loop {
            let mut ui = Ui::new(&self.interaction);
            let root = self.app.view(&mut ui, scale, size)?;
            let (mut scene, elements) = ui.finish();
            let content = scene.layout(root, self.stage.text())?;
            attempts += 1;
            if !self.interaction.attach(&scene, root, elements)? || attempts == 2 {
                break (scene, root, content);
            }
        };
        self.last_build = Some(start.elapsed());
        // 窗口大小变了位置不可比，不过渡；滚动挪动的也不可比：内容整体平移不该一个个节点滑过去
        let compat = ((u64::from(size.0) << 32) | u64::from(size.1))
            ^ self.interaction.scroll_generation().rotate_left(17);
        self.stage.present(scene, root, content, compat, scale, now)
    }
}

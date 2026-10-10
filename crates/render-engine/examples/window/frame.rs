//! 出一帧：建树 → 布局 → 交给交互层 → 交给舞台画（与上一帧配对过渡）。窗口与截图两条路共用。

use std::time::{Duration, Instant};

use render_engine::{Interaction, Painted, RenderError, Stage, Ui};

use crate::demo::{Demo, Message};
use crate::page;

/// 画好的一帧与建树（含布局）花了多久。
pub struct Frame {
    pub painted: Painted,

    pub build: Duration,
}

pub fn render(
    stage: &mut Stage,
    interaction: &mut Interaction<Message>,
    demo: &Demo,
    scale: f32,
    size: (u32, u32),
) -> Result<Frame, RenderError> {
    let start = Instant::now();
    // 布局挪动可能让指针换了元素，悬停状态变了要按新状态再建一次；再变也不追了，下次事件会改正
    let mut attempts = 0;
    let (scene, root, content) = loop {
        let mut ui = Ui::new(interaction);
        let root = page::view(&mut ui, demo, scale, size)?;
        let (mut scene, elements) = ui.finish();
        let content = scene.layout(root, stage.text())?;
        attempts += 1;
        if !interaction.attach(&scene, root, elements)? || attempts == 2 {
            break (scene, root, content);
        }
    };
    let built = Instant::now();
    // 窗口大小变了位置不可比，不过渡
    let compat = (u64::from(size.0) << 32) | u64::from(size.1);
    let painted = stage.present(scene, root, content, compat, scale, Instant::now())?;
    Ok(Frame {
        painted,
        build: built - start,
    })
}

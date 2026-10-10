//! 动画胶片：点一下开关，按固定间隔推进过渡，把开关那一块逐帧截下来排成一条，顺带量每个动画帧的耗时。

use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use render_engine::taffy::{FlexDirection, LengthPercentage, Rect, Size, Style};
use render_engine::tiny_skia::IntRect;
use render_engine::{
    BoxPaint, Color, Event, Fill, FontLibrary, FontSpec, Interaction, Key, Painted, Pixmap, Scene,
    Stage, TextStyle, Visual,
};

use crate::demo::{Demo, Message};
use crate::frame;

const SCALE: f32 = 2.0;

const SIZE: (u32, u32) = (1560, 1320);

/// 每隔多久截一帧。
const STEP: Duration = Duration::from_millis(30);

const FRAMES: u32 = 10;

pub fn run(directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut stage = Stage::new(FontLibrary::system("zh-CN")?);
    let mut interaction = Interaction::new();
    let mut demo = Demo::new(false);
    let key = Key::ROOT.child("switch").child(0_usize);
    frame::render(&mut stage, &mut interaction, &demo, SCALE, SIZE)?;
    let (x, y, width, height) = interaction.rect(key).ok_or("找不到开关")?;
    let pad = 16.0;
    let area = (x - pad, y - pad, width + pad * 2.0, height + pad * 2.0);

    interaction.handle(&Event::PointerMoved {
        x: x + width / 2.0,
        y: y + height / 2.0,
    });
    interaction.handle(&Event::PointerDown);
    frame::render(&mut stage, &mut interaction, &demo, SCALE, SIZE)?;
    // 按住到拉宽的过渡播完再松开
    let held = Instant::now() + Duration::from_millis(300);
    stage.tick_at(held)?;
    let response = interaction.handle(&Event::PointerUp);
    for message in response.messages {
        if !matches!(message, Message::Quit) {
            demo.update(message);
        }
    }
    let first = frame::render(&mut stage, &mut interaction, &demo, SCALE, SIZE)?.painted;
    let start = Instant::now();

    let mut frames = vec![crop(&first, area)?];
    let mut spent = Duration::ZERO;
    for index in 1..=FRAMES {
        let began = Instant::now();
        let painted = stage.tick_at(start + STEP * index)?;
        spent += began.elapsed();
        if let Some(painted) = painted {
            frames.push(crop(&painted, area)?);
        }
    }
    println!(
        "动画帧平均 {:.2} ms（整窗 {}×{}，{} 帧）",
        spent.as_secs_f64() * 1000.0 / f64::from(FRAMES),
        SIZE.0,
        SIZE.1,
        FRAMES
    );
    let strip = strip(&mut stage, frames)?;
    let path = directory.join("film.png");
    strip.save_png(&path)?;
    println!("{}：开关动画逐帧", path.display());
    Ok(())
}

fn crop(
    painted: &Painted,
    (x, y, width, height): (f32, f32, f32, f32),
) -> Result<Pixmap, Box<dyn std::error::Error>> {
    let rect = IntRect::from_xywh(
        (x + painted.content_x as f32) as i32,
        (y + painted.content_y as f32) as i32,
        width as u32,
        height as u32,
    )
    .ok_or("区域无效")?;
    Ok(painted.pixmap.clone_rect(rect).ok_or("超出位图")?)
}

/// 横排一条，每帧放大两倍，下面标毫秒。
fn strip(stage: &mut Stage, frames: Vec<Pixmap>) -> Result<Pixmap, Box<dyn std::error::Error>> {
    let mut scene = Scene::new();
    let caption = TextStyle::new(FontSpec::new(22.0, 30.0), 11.0, Color::gray(0, 200), 1.0);
    let mut cells = Vec::new();
    for (index, pixmap) in frames.into_iter().enumerate() {
        let (width, height) = (pixmap.width() as f32 * 2.0, pixmap.height() as f32 * 2.0);
        let image = scene.node(
            Style {
                size: Size {
                    width: render_engine::taffy::Dimension::length(width),
                    height: render_engine::taffy::Dimension::length(height),
                },
                ..Style::default()
            },
            Visual::Box(BoxPaint {
                fill: Some(Fill::Image {
                    pixmap: Arc::new(pixmap),
                    slice: None,
                    pixels_per_px: 0.5,
                }),
                border: Some((1.0, Color::gray(0, 30))),
                radius: 0.0,
            }),
            &[],
        )?;
        let label = scene.node(
            Style::default(),
            Visual::Text {
                text: format!("{} ms", index as u32 * STEP.as_millis() as u32),
                style: caption,
            },
            &[],
        )?;
        cells.push(scene.node(
            Style {
                flex_direction: FlexDirection::Column,
                gap: Size {
                    width: LengthPercentage::length(6.0),
                    height: LengthPercentage::length(6.0),
                },
                ..Style::default()
            },
            Visual::Group,
            &[image, label],
        )?);
    }
    let pad = LengthPercentage::length(24.0);
    let root = scene.node(
        Style {
            flex_direction: FlexDirection::Row,
            gap: Size {
                width: LengthPercentage::length(12.0),
                height: LengthPercentage::length(12.0),
            },
            padding: Rect {
                left: pad,
                right: pad,
                top: pad,
                bottom: pad,
            },
            ..Style::default()
        },
        Visual::solid(Color::rgb(0xF5, 0xF5, 0xF7), 0.0),
        &cells,
    )?;
    let content = scene.layout(root, stage.text())?;
    Ok(stage.rasterize(&scene, root, content, 1.0)?.pixmap)
}

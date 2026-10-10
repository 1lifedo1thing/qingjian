//! 截图模式：不开窗口，按脚本发一串事件，每步存一张 PNG，最后拼一张总览（总览也用引擎画）。

use std::path::Path;
use std::sync::Arc;

use render_engine::taffy::{
    Dimension, FlexDirection, FlexWrap, LengthPercentage, Rect, Size, Style,
};
use render_engine::{
    BoxPaint, Color, Event, Fill, FontLibrary, FontSpec, Interaction, Key, KeyCode, KeyPress,
    Modifiers, Pixmap, Scene, Stage, TextStyle, Visual,
};

use crate::demo::{Demo, Message};
use crate::frame;

const SCALE: f32 = 2.0;

const SIZE: (u32, u32) = (1560, 1320);

/// 一步：说明，以及截图前要发的事件。
enum Step {
    Shot(&'static str),
    Move(Key),
    Down,
    Up,
    Press(KeyCode, bool),
    Dark,
}

pub fn run(directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(directory)?;
    let mut stage = Stage::new(FontLibrary::system("zh-CN")?);
    let mut interaction = Interaction::new();
    let mut demo = Demo::new(false);
    let sidebar = |index: usize| Key::ROOT.child("sidebar").child(index);
    let switch = |index: usize| Key::ROOT.child("switch").child(index);
    let steps = [
        Step::Shot("默认"),
        Step::Move(sidebar(2)),
        Step::Shot("悬停侧栏「外观」"),
        Step::Move(switch(3)),
        Step::Shot("悬停关着的开关"),
        Step::Move(switch(0)),
        Step::Down,
        Step::Shot("按住开着的开关：旋钮拉宽"),
        Step::Up,
        Step::Shot("松开：关掉了，焦点在它上但不画环"),
        Step::Press(KeyCode::Tab, false),
        Step::Shot("Tab：焦点环到下一个开关"),
        Step::Press(KeyCode::Space, false),
        Step::Shot("空格：切换它"),
        Step::Move(Key::ROOT.child("恢复默认")),
        Step::Down,
        Step::Shot("按住「恢复默认」"),
        Step::Up,
        Step::Move(sidebar(2)),
        Step::Down,
        Step::Up,
        Step::Shot("点「外观」切页"),
        Step::Dark,
        Step::Move(sidebar(1)),
        Step::Down,
        Step::Up,
        Step::Press(KeyCode::Tab, false),
        Step::Shot("深色，Tab 后的焦点环"),
    ];

    let mut shots: Vec<(&str, Pixmap)> = Vec::new();
    // 先出一帧，交互层才有元素的位置
    frame::render(&mut stage, &mut interaction, &demo, SCALE, SIZE)?;
    for step in steps {
        let event = match step {
            Step::Shot(caption) => {
                let frame = frame::render(&mut stage, &mut interaction, &demo, SCALE, SIZE)?;
                let painted = frame.painted;
                let pixmap = painted
                    .pixmap
                    .clone_rect(
                        render_engine::tiny_skia::IntRect::from_xywh(
                            painted.content_x as i32,
                            painted.content_y as i32,
                            painted.content_width,
                            painted.content_height,
                        )
                        .ok_or("截图区域无效")?,
                    )
                    .ok_or("截图区域超出位图")?;
                let path = directory.join(format!("{:02}.png", shots.len() + 1));
                pixmap.save_png(&path)?;
                println!("{}：{caption}", path.display());
                shots.push((caption, pixmap));
                continue;
            }
            Step::Move(key) => {
                let (x, y, width, height) = interaction.rect(key).ok_or("找不到元素")?;
                Event::PointerMoved {
                    x: x + width / 2.0,
                    y: y + height / 2.0,
                }
            }
            Step::Down => Event::PointerDown,
            Step::Up => Event::PointerUp,
            Step::Press(key, shift) => Event::Key(KeyPress::new(
                key,
                Modifiers {
                    shift,
                    ..Modifiers::default()
                },
            )),
            Step::Dark => {
                demo.dark = true;
                frame::render(&mut stage, &mut interaction, &demo, SCALE, SIZE)?;
                continue;
            }
        };
        let response = interaction.handle(&event);
        let rebuild = response.needs_rebuild();
        for message in response.messages {
            if matches!(message, Message::Quit) {
                continue;
            }
            demo.update(message);
        }
        if rebuild {
            frame::render(&mut stage, &mut interaction, &demo, SCALE, SIZE)?;
        }
    }

    let sheet = sheet(&mut stage, shots)?;
    let path = directory.join("sheet.png");
    sheet.save_png(&path)?;
    println!("{}：总览", path.display());
    Ok(())
}

/// 两列的总览：每格一张缩到一半的截图，上面写说明。
fn sheet(
    stage: &mut Stage,
    shots: Vec<(&str, Pixmap)>,
) -> Result<Pixmap, Box<dyn std::error::Error>> {
    let (cell_w, cell_h) = (SIZE.0 as f32 / 2.0, SIZE.1 as f32 / 2.0);
    let gap = 32.0;
    let mut scene = Scene::new();
    let caption = TextStyle::new(FontSpec::new(28.0, 40.0), 14.0, Color::gray(0, 220), 1.0);
    let mut cells = Vec::new();
    for (index, (text, pixmap)) in shots.into_iter().enumerate() {
        let label = scene.node(
            Style::default(),
            Visual::Text {
                text: format!("{}. {text}", index + 1),
                style: caption,
            },
            &[],
        )?;
        let image = scene.node(
            Style {
                size: Size {
                    width: Dimension::length(cell_w),
                    height: Dimension::length(cell_h),
                },
                ..Style::default()
            },
            Visual::Box(BoxPaint {
                fill: Some(Fill::Image {
                    pixmap: Arc::new(pixmap),
                    slice: None,
                    pixels_per_px: 2.0,
                }),
                border: Some((1.0, Color::gray(0, 40))),
                radius: 0.0,
            }),
            &[],
        )?;
        cells.push(scene.node(
            Style {
                flex_direction: FlexDirection::Column,
                gap: Size {
                    width: LengthPercentage::length(0.0),
                    height: LengthPercentage::length(8.0),
                },
                ..Style::default()
            },
            Visual::Group,
            &[label, image],
        )?);
    }
    let padding = LengthPercentage::length(gap);
    let root = scene.node(
        Style {
            size: Size {
                width: Dimension::length(cell_w * 2.0 + gap * 3.0),
                height: Dimension::auto(),
            },
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            gap: Size {
                width: LengthPercentage::length(gap),
                height: LengthPercentage::length(gap),
            },
            padding: Rect {
                left: padding,
                right: padding,
                top: padding,
                bottom: padding,
            },
            ..Style::default()
        },
        Visual::solid(Color::rgb(0xE4, 0xE4, 0xE8), 0.0),
        &cells,
    )?;
    let content = scene.layout(root, stage.text())?;
    Ok(stage.rasterize(&scene, root, content, 1.0)?.pixmap)
}

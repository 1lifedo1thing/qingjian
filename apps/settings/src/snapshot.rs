//! 截图模式：拷一份配置到输出目录（真配置不动），按脚本点几下，每步存一张 PNG。给没法亲手点的时候看效果用。

use std::path::Path;
use std::time::Instant;

use render_engine::tiny_skia::IntRect;
use render_engine::{App, Event, FontLibrary, Key, Runner};

use crate::settings::Settings;

const SCALE: f32 = 2.0;

const SIZE: (u32, u32) = (1640, 1240);

pub fn run(config: Option<&Path>, directory: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(directory)?;
    let copy = directory.join("config.toml");
    match config.filter(|path| path.exists()) {
        Some(path) => {
            std::fs::copy(path, &copy)?;
        }
        None => {
            let _ = std::fs::remove_file(&copy);
        }
    }
    let settings = Settings::load(copy)?;
    let themes: Vec<String> = settings
        .themes
        .themes()
        .iter()
        .map(|theme| theme.id().to_owned())
        .collect();
    let mut runner = Runner::new(settings, FontLibrary::system("zh-CN")?);
    runner.stage().set_animations(false);
    runner.rebuild(SCALE, SIZE, Instant::now())?;

    let mut index = 0;
    let mut shot =
        |runner: &mut Runner<Settings>, caption: &str| -> Result<(), Box<dyn std::error::Error>> {
            let painted = runner.rebuild(SCALE, SIZE, Instant::now())?;
            let rect = IntRect::from_xywh(
                painted.content_x as i32,
                painted.content_y as i32,
                painted.content_width,
                painted.content_height,
            )
            .ok_or("截图区域无效")?;
            let pixmap = painted.pixmap.clone_rect(rect).ok_or("截图区域超出位图")?;
            index += 1;
            let path = directory.join(format!("{index:02}.png"));
            pixmap.save_png(&path)?;
            println!("{}：{caption}", path.display());
            Ok(())
        };

    shot(&mut runner, "打开时（按配置文件）")?;
    click(&mut runner, Key::ROOT.child("layout").child(1_usize))?;
    shot(&mut runner, "排布选「横排」：预览跟着变")?;
    if let Some(id) = themes.get(1) {
        click(&mut runner, Key::ROOT.child("themes").child(id))?;
        shot(&mut runner, "点第二个主题")?;
    }
    if let Some(id) = themes.last() {
        click(&mut runner, Key::ROOT.child("themes").child(id))?;
        shot(
            &mut runner,
            "点最后一个主题（樱花带装饰，预览要缩进卡片里）",
        )?;
    }
    click(&mut runner, Key::ROOT.child("appearance").child(2_usize))?;
    shot(&mut runner, "外观选「深色」：预览与主题卡片换深色")?;
    click(&mut runner, Key::ROOT.child("font"))?;
    shot(&mut runner, "点字体下拉框：弹出本机字族")?;
    hover(&mut runner, Key::ROOT.child("font-menu").child(3_usize))?;
    shot(&mut runner, "悬停第三个字族")?;
    runner.handle(&Event::PointerDown);
    runner.handle(&Event::PointerUp);
    shot(&mut runner, "选中后菜单收起，下拉框显示新字族")?;
    hover(&mut runner, Key::ROOT.child(("content", 1_usize)))?;
    runner.handle(&Event::Scroll {
        dx: 0.0,
        dy: 5000.0,
    });
    shot(&mut runner, "滚到底")?;
    runner.app_mut().set_system_dark(true);
    shot(&mut runner, "系统深色：设置窗口自己也换深色")?;
    Ok(())
}

fn hover(runner: &mut Runner<Settings>, key: Key) -> Result<(), Box<dyn std::error::Error>> {
    let (x, y, width, height) = runner.interaction().rect(key).ok_or("找不到元素")?;
    runner.handle(&Event::PointerMoved {
        x: x + width / 2.0,
        y: y + height / 2.0,
    });
    Ok(())
}

/// 元素在滚动区外时先滚到看得见（离视口边留 40 像素）。
fn reveal(runner: &mut Runner<Settings>, key: Key) -> Result<(), Box<dyn std::error::Error>> {
    let content = Key::ROOT.child(("content", 1_usize));
    let (_, top, _, height) = runner.interaction().rect(content).ok_or("找不到滚动区")?;
    let (_, y, _, h) = runner.interaction().rect(key).ok_or("找不到元素")?;
    let margin = 40.0;
    let dy = if y + h > top + height - margin {
        y + h - (top + height - margin)
    } else if y < top + margin {
        y - top - margin
    } else {
        return Ok(());
    };
    hover(runner, content)?;
    runner.handle(&Event::Scroll { dx: 0.0, dy });
    runner.rebuild(SCALE, SIZE, Instant::now())?;
    Ok(())
}

fn click(runner: &mut Runner<Settings>, key: Key) -> Result<(), Box<dyn std::error::Error>> {
    reveal(runner, key)?;
    hover(runner, key)?;
    runner.handle(&Event::PointerDown);
    runner.handle(&Event::PointerUp);
    runner.rebuild(SCALE, SIZE, Instant::now())?;
    Ok(())
}

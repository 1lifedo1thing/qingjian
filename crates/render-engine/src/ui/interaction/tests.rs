use taffy::{Dimension, FlexDirection, LengthPercentage, NodeId, Rect, Size, Style};

use super::*;
use crate::fonts::FontLibrary;
use crate::scene::Visual;
use crate::text::TextPainter;
use crate::ui::{Interact, KeyCode, KeyPress, Modifiers, Ui};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Ok,
    Cancel,
    Close,
}

/// 200×100 的根（登记了 Esc）里左右两个按钮：左边「确定」里有个图标，右边「取消」按 `cancel_disabled` 禁用。
fn frame(interaction: &mut Interaction<Message>, cancel_disabled: bool) -> bool {
    let mut ui = Ui::new(interaction);
    let fixed = |width: f32, height: f32| Style {
        size: Size {
            width: Dimension::length(width),
            height: Dimension::length(height),
        },
        ..Style::default()
    };
    let icon = ui
        .scene()
        .node(fixed(20.0, 20.0), Visual::Group, &[])
        .unwrap();
    let icon_key = ui.key("icon");
    ui.interact(icon, icon_key, Interact::new());
    let ok = ui
        .scene()
        .node(
            Style {
                padding: Rect {
                    left: LengthPercentage::length(10.0),
                    right: LengthPercentage::length(10.0),
                    top: LengthPercentage::length(10.0),
                    bottom: LengthPercentage::length(10.0),
                },
                ..fixed(100.0, 100.0)
            },
            Visual::Group,
            &[icon],
        )
        .unwrap();
    let ok_key = ui.key("ok");
    ui.interact(
        ok,
        ok_key,
        Interact::new().on_click(Message::Ok).focusable(),
    );
    let cancel = ui
        .scene()
        .node(fixed(100.0, 100.0), Visual::Group, &[])
        .unwrap();
    let cancel_key = ui.key("cancel");
    ui.interact(
        cancel,
        cancel_key,
        Interact::new()
            .on_click(Message::Cancel)
            .focusable()
            .disabled(cancel_disabled),
    );
    let root = ui
        .scene()
        .node(
            Style {
                flex_direction: FlexDirection::Row,
                ..Style::default()
            },
            Visual::Group,
            &[ok, cancel],
        )
        .unwrap();
    let root_key = ui.key("root");
    ui.interact(
        root,
        root_key,
        Interact::new().on_key(|press| (press.key == KeyCode::Escape).then_some(Message::Close)),
    );
    let (mut scene, elements) = ui.finish();
    layout(&mut scene, root);
    interaction.attach(&scene, root, elements).unwrap()
}

fn layout(scene: &mut Scene, root: NodeId) {
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    scene.layout(root, &mut text).unwrap();
}

fn key(name: &str) -> Key {
    Key::ROOT.child(name)
}

fn press(code: KeyCode, shift: bool) -> Event {
    Event::Key(KeyPress::new(
        code,
        Modifiers {
            shift,
            ..Modifiers::default()
        },
    ))
}

#[test]
fn hover_covers_target_and_ancestors() {
    let mut interaction = Interaction::new();
    frame(&mut interaction, false);
    let response = interaction.handle(&Event::PointerMoved { x: 15.0, y: 15.0 });
    assert!(response.changed);
    assert!(interaction.state(key("icon")).hovered);
    assert!(interaction.state(key("ok")).hovered);
    assert!(interaction.state(key("root")).hovered);
    assert!(!interaction.state(key("cancel")).hovered);

    let response = interaction.handle(&Event::PointerMoved { x: 50.0, y: 50.0 });
    assert!(response.changed);
    assert!(!interaction.state(key("icon")).hovered);
    let response = interaction.handle(&Event::PointerMoved { x: 51.0, y: 50.0 });
    assert!(!response.changed);
}

#[test]
fn click_bubbles_to_nearest_handler() {
    let mut interaction = Interaction::new();
    frame(&mut interaction, false);
    interaction.handle(&Event::PointerMoved { x: 15.0, y: 15.0 });
    interaction.handle(&Event::PointerDown);
    assert!(interaction.state(key("ok")).pressed);
    assert!(interaction.state(key("ok")).focused);
    assert!(!interaction.state(key("ok")).focus_ring);
    let response = interaction.handle(&Event::PointerUp);
    assert_eq!(response.messages, vec![Message::Ok]);
}

#[test]
fn dragging_out_cancels_click() {
    let mut interaction = Interaction::new();
    frame(&mut interaction, false);
    interaction.handle(&Event::PointerMoved { x: 50.0, y: 50.0 });
    interaction.handle(&Event::PointerDown);
    interaction.handle(&Event::PointerMoved { x: 150.0, y: 50.0 });
    assert!(!interaction.state(key("ok")).pressed);
    let response = interaction.handle(&Event::PointerUp);
    assert!(response.messages.is_empty());
}

#[test]
fn disabled_element_swallows_click() {
    let mut interaction = Interaction::new();
    frame(&mut interaction, true);
    interaction.handle(&Event::PointerMoved { x: 150.0, y: 50.0 });
    interaction.handle(&Event::PointerDown);
    let response = interaction.handle(&Event::PointerUp);
    assert!(response.messages.is_empty());
}

#[test]
fn tab_cycles_focus_and_space_clicks() {
    let mut interaction = Interaction::new();
    frame(&mut interaction, false);
    interaction.handle(&press(KeyCode::Tab, false));
    assert!(interaction.state(key("ok")).focus_ring);
    interaction.handle(&press(KeyCode::Tab, false));
    assert!(interaction.state(key("cancel")).focus_ring);
    interaction.handle(&press(KeyCode::Tab, false));
    assert!(interaction.state(key("ok")).focus_ring);
    interaction.handle(&press(KeyCode::Tab, true));
    assert!(interaction.state(key("cancel")).focused);

    let response = interaction.handle(&press(KeyCode::Space, false));
    assert_eq!(response.messages, vec![Message::Cancel]);
    let response = interaction.handle(&press(KeyCode::Escape, false));
    assert_eq!(response.messages, vec![Message::Close]);
}

#[test]
fn tab_skips_disabled() {
    let mut interaction = Interaction::new();
    frame(&mut interaction, true);
    interaction.handle(&press(KeyCode::Tab, false));
    interaction.handle(&press(KeyCode::Tab, false));
    assert!(interaction.state(key("ok")).focused);
}

#[test]
fn escape_without_focus_reaches_root() {
    let mut interaction = Interaction::new();
    frame(&mut interaction, false);
    let response = interaction.handle(&press(KeyCode::Escape, false));
    assert_eq!(response.messages, vec![Message::Close]);
}

#[test]
fn state_survives_rebuild_and_lost_focus_clears() {
    let mut interaction = Interaction::new();
    frame(&mut interaction, false);
    interaction.handle(&Event::PointerMoved { x: 150.0, y: 50.0 });
    interaction.handle(&press(KeyCode::Tab, true));
    assert!(interaction.state(key("cancel")).focused);

    assert!(!frame(&mut interaction, false));
    assert!(interaction.state(key("cancel")).hovered);
    assert!(interaction.state(key("cancel")).focused);

    // 禁用后失去焦点
    frame(&mut interaction, true);
    interaction.handle(&press(KeyCode::Tab, false));
    assert!(interaction.state(key("ok")).focused);
}

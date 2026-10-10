use taffy::{FlexDirection, NodeId, Style};

use super::*;
use crate::fonts::FontLibrary;
use crate::scene::{Scene, Visual};
use crate::text::TextPainter;
use crate::ui::{Event, Interaction, Key, KeyCode, KeyPress, Modifiers, Ui};

#[derive(Debug, Clone, PartialEq)]
enum Message {
    Pick(usize),
    Toggle,
    Down,
    Up,
}

/// 一排：分段选择（选中第 `selected` 个）、开关、到了上限的步进框。
fn frame(interaction: &mut Interaction<Message>, text: &mut TextPainter, selected: usize) {
    let theme = WidgetTheme::new(WidgetColors::LIGHT, 1.0);
    let mut ui = Ui::new(interaction);
    let segmented = Segmented::new(&["一", "二", "三"], selected)
        .build(&mut ui, &theme, "choice", Message::Pick)
        .unwrap();
    let switch = Switch::new(true)
        .build(&mut ui, &theme, "switch", Message::Toggle)
        .unwrap();
    let stepper = Stepper::new("40 pt")
        .limits(true, false)
        .build(&mut ui, &theme, "size", Message::Down, Message::Up)
        .unwrap();
    let root = ui
        .scene()
        .node(
            Style {
                flex_direction: FlexDirection::Row,
                ..Style::default()
            },
            Visual::Group,
            &[segmented, switch, stepper],
        )
        .unwrap();
    let (mut scene, elements) = ui.finish();
    layout(&mut scene, root, text);
    interaction.attach(&scene, root, elements).unwrap();
}

fn layout(scene: &mut Scene, root: NodeId, text: &mut TextPainter) {
    scene.layout(root, text).unwrap();
}

fn press(interaction: &mut Interaction<Message>, key: KeyCode) -> Vec<Message> {
    interaction
        .handle(&Event::Key(KeyPress::new(key, Modifiers::default())))
        .messages
}

fn click(interaction: &mut Interaction<Message>, key: Key) -> Vec<Message> {
    let (x, y, width, height) = interaction.rect(key).unwrap();
    interaction.handle(&Event::PointerMoved {
        x: x + width / 2.0,
        y: y + height / 2.0,
    });
    interaction.handle(&Event::PointerDown);
    interaction.handle(&Event::PointerUp).messages
}

#[test]
fn segmented_takes_focus_once_and_moves_with_arrows() {
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    let mut interaction = Interaction::new();
    frame(&mut interaction, &mut text, 0);
    press(&mut interaction, KeyCode::Tab);
    assert!(interaction.state(Key::ROOT.child("choice")).focus_ring);
    assert!(press(&mut interaction, KeyCode::ArrowLeft).is_empty());
    assert_eq!(
        press(&mut interaction, KeyCode::ArrowRight),
        vec![Message::Pick(1)]
    );
    // 单个选项不单独拿焦点：下一个 Tab 直接到开关
    press(&mut interaction, KeyCode::Tab);
    assert!(interaction.state(Key::ROOT.child("switch")).focused);
    assert_eq!(
        press(&mut interaction, KeyCode::Space),
        vec![Message::Toggle]
    );
}

#[test]
fn clicking_an_option_picks_it() {
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    let mut interaction = Interaction::new();
    frame(&mut interaction, &mut text, 0);
    let third = Key::ROOT.child("choice").child(2_usize);
    assert_eq!(click(&mut interaction, third), vec![Message::Pick(2)]);
    assert!(interaction.state(Key::ROOT.child("choice")).focused);
}

#[test]
fn stepper_at_limit_ignores_increase() {
    let mut text = TextPainter::new(FontLibrary::system("en").unwrap());
    let mut interaction = Interaction::new();
    frame(&mut interaction, &mut text, 0);
    let size = Key::ROOT.child("size");
    assert!(click(&mut interaction, size.child("increase")).is_empty());
    assert_eq!(
        click(&mut interaction, size.child("decrease")),
        vec![Message::Down]
    );
    assert!(press(&mut interaction, KeyCode::ArrowUp).is_empty());
    assert_eq!(
        press(&mut interaction, KeyCode::ArrowDown),
        vec![Message::Down]
    );
}

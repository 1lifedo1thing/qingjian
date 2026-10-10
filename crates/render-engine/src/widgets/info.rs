//! 悬停说明：标题旁一个 ⓘ，指针停在上面（或键盘把焦点移过来）时在它下面浮出一段说明。
//! 说明是浮层（[`crate::Ui::overlay`]），不被滚动区裁掉；位置贴着 ⓘ 上一帧的位置，第一帧不出。

use std::hash::Hash;
use std::sync::Arc;

use taffy::{LengthPercentageAuto, NodeId, Position, Rect, Size, Style};

use super::WidgetTheme;
use super::info_mark::InfoMark;
use super::paragraph::paragraph;
use super::parts::{centered, focus_ring, padding, solid};
use crate::error::RenderError;
use crate::scene::{Effect, EffectKind, Visual};
use crate::ui::{Interact, Ui};

/// 说明最宽多少（点），再长就折行。
const MAX_WIDTH: f32 = 280.0;

pub fn info<M>(
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
    local: impl Hash,
    text: &str,
) -> Result<NodeId, RenderError> {
    let key = ui.key(local);
    let state = ui.state(key);
    let colors = &theme.colors;
    let size = theme.px(13.0);
    let open = state.hovered || state.focus_ring;
    let mut children = Vec::new();
    if state.focus_ring {
        children.push(focus_ring(ui, theme, size / 2.0)?);
    }
    let node = ui.scene().node(
        centered(size, size),
        Visual::Icon {
            icon: Arc::new(InfoMark),
            size,
            color: if open { colors.text } else { colors.secondary },
        },
        &children,
    )?;
    ui.interact(node, key, Interact::new().focusable());
    if open && let Some((x, y, _, height)) = ui.rect(key) {
        let note = paragraph(ui, theme, text, Some(colors.text))?;
        let bubble = ui.scene().node(
            Style {
                position: Position::Absolute,
                inset: Rect {
                    left: LengthPercentageAuto::length(x - theme.px(10.0)),
                    top: LengthPercentageAuto::length(y + height + theme.px(6.0)),
                    right: LengthPercentageAuto::auto(),
                    bottom: LengthPercentageAuto::auto(),
                },
                max_size: Size {
                    width: LengthPercentageAuto::length(theme.px(MAX_WIDTH)),
                    height: LengthPercentageAuto::auto(),
                },
                padding: padding(theme, 7.0, 10.0),
                ..Style::default()
            },
            solid(
                Some(colors.popup),
                Some((theme.px(1.0), colors.line)),
                theme.px(theme.radius),
            ),
            &[note],
        )?;
        ui.scene().set_effects(
            bubble,
            vec![Effect {
                kind: EffectKind::DropShadow,
                x: 0.0,
                y: theme.px(4.0),
                blur: theme.px(12.0),
                spread: 0.0,
                color: colors.shadow,
            }],
        );
        ui.overlay(bubble);
    }
    Ok(node)
}

//! 单行输入框：文字、光标、选区与输入法的组字都画出来，编辑交给使用方保存的 [`TextEdit`]。
//!
//! 光标与选区不用量字宽：文字按光标、选区切成几段放进一行，光标是段与段之间的一条竖线，由布局摆位置。
//! 文字比框长时框里裁掉；光标在末尾时整行靠右，打字时总能看到最后几个字。点击只拿焦点、不挪光标。

mod edit;

use std::hash::Hash;

use taffy::{AlignItems, JustifyContent, NodeId, Style};

use super::WidgetTheme;
use super::parts::{DISABLED_OPACITY, padding, row, size, solid};
use crate::animation::Easing;
use crate::error::RenderError;
use crate::scene::Visual;
use crate::text::Decoration;
use crate::ui::{Interact, KeyCode, Ui};

pub use edit::{Edit, TextEdit};

/// 密文显示成的圆点。
const BULLET: char = '•';

pub struct TextField<'a> {
    edit: &'a TextEdit,

    placeholder: &'a str,

    /// 密文：显示成圆点，不能复制、剪切。
    secret: bool,

    /// 宽度（点）。
    width: f32,

    disabled: bool,
}

impl<'a> TextField<'a> {
    pub fn new(edit: &'a TextEdit) -> Self {
        Self {
            edit,
            placeholder: "",
            secret: false,
            width: 240.0,
            disabled: false,
        }
    }

    /// 空着时显示的灰字。
    pub fn placeholder(self, placeholder: &'a str) -> Self {
        Self {
            placeholder,
            ..self
        }
    }

    pub fn secret(self, secret: bool) -> Self {
        Self { secret, ..self }
    }

    pub fn width(self, points: f32) -> Self {
        Self {
            width: points,
            ..self
        }
    }

    pub fn disabled(self, disabled: bool) -> Self {
        Self { disabled, ..self }
    }

    /// `on_edit` 收每次编辑；回车与失去焦点产出 `on_commit`（使用方此时保存）；
    /// 有 `on_cancel` 时 Esc 产出它（使用方把编辑状态换回保存的值），没有时 Esc 照常往上冒。
    pub fn build<M: Clone + 'static>(
        self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        local: impl Hash,
        on_edit: impl Fn(Edit) -> M + Clone + 'static,
        on_commit: M,
        on_cancel: Option<M>,
    ) -> Result<NodeId, RenderError> {
        let key = ui.key(local);
        let state = ui.state(key);
        let colors = &theme.colors;
        let focused = state.focused && !self.disabled;
        let pieces = self.pieces(ui, theme, focused)?;
        let line = ui.scene().node(
            Style {
                flex_shrink: 0.0,
                align_items: Some(AlignItems::CENTER),
                ..row(theme, 0.0)
            },
            Visual::Group,
            &pieces,
        )?;
        // 光标在末尾、字又比框长时靠右；上一帧量出来的宽度够用，差一帧只在刚好超出的那个字上
        let overflowing = match (ui.rect(key.child("line")), ui.rect(key.child("view"))) {
            (Some(line), Some(view)) => line.2 > view.2,
            _ => false,
        };
        let at_end = self.edit.caret() == self.edit.text().len();
        ui.interact(line, key.child("line"), Interact::new());
        let view = ui.scene().node(
            Style {
                flex_grow: 1.0,
                justify_content: Some(if overflowing && at_end {
                    JustifyContent::FLEX_END
                } else {
                    JustifyContent::FLEX_START
                }),
                align_items: Some(AlignItems::CENTER),
                ..Style::default()
            },
            Visual::Group,
            &[line],
        )?;
        ui.scene().set_clip(view, 0.0);
        ui.interact(view, key.child("view"), Interact::new());
        let border = if focused { colors.accent } else { colors.line };
        let node = ui.scene().node(
            Style {
                padding: padding(theme, 0.0, 8.0),
                size: size(
                    Some(theme.px(self.width)),
                    Some(theme.px(theme.control_height)),
                ),
                flex_shrink: 0.0,
                ..row(theme, 0.0)
            },
            solid(
                Some(colors.surface),
                Some((theme.px(if focused { 1.5 } else { 1.0 }), border)),
                theme.px(theme.radius),
            ),
            &[view],
        )?;
        ui.transition(node, key, theme.hover, Easing::Ease);
        if self.disabled {
            ui.scene().set_opacity(node, DISABLED_OPACITY);
        }
        let secret = self.secret;
        let editing = on_edit.clone();
        let commit = on_commit.clone();
        ui.interact(
            node,
            key,
            Interact::new()
                .on_text(move |input| Some(on_edit(Edit::Text(input.clone()))))
                .on_key(move |press| {
                    let command = press.modifiers.command;
                    match &press.key {
                        KeyCode::Enter => Some(commit.clone()),
                        KeyCode::Escape => on_cancel.clone(),
                        KeyCode::ArrowLeft
                        | KeyCode::ArrowRight
                        | KeyCode::Home
                        | KeyCode::End
                        | KeyCode::Backspace
                        | KeyCode::Delete => Some(editing(Edit::Key(press.clone()))),
                        KeyCode::Character(text)
                            if command
                                && (text == "a" || (!secret && (text == "c" || text == "x"))) =>
                        {
                            Some(editing(Edit::Key(press.clone())))
                        }
                        _ => None,
                    }
                })
                .on_blur(on_commit)
                .disabled(self.disabled),
        );
        Ok(node)
    }

    /// 一行里的各段：选区前、选区、选区后，或光标前、组字、光标、光标后。
    fn pieces<M>(
        &self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        focused: bool,
    ) -> Result<Vec<NodeId>, RenderError> {
        let colors = &theme.colors;
        let text = self.edit.text();
        let preedit = self.edit.preedit();
        let mut pieces = Vec::new();
        if text.is_empty() && preedit.is_empty() {
            if focused {
                pieces.push(self.caret(ui, theme)?);
            }
            if !self.placeholder.is_empty() {
                pieces.push(segment(
                    ui,
                    theme,
                    self.placeholder,
                    colors.secondary,
                    None,
                )?);
            }
            return Ok(pieces);
        }
        let shown = |part: &str| -> String {
            if self.secret {
                std::iter::repeat_n(BULLET, part.chars().count()).collect()
            } else {
                part.to_owned()
            }
        };
        let selection = self.edit.selection();
        if !focused {
            pieces.push(segment(ui, theme, &shown(text), colors.text, None)?);
        } else if !selection.is_empty() {
            pieces.push(segment(
                ui,
                theme,
                &shown(&text[..selection.start]),
                colors.text,
                None,
            )?);
            let selected = segment(
                ui,
                theme,
                &shown(&text[selection.clone()]),
                colors.text,
                None,
            )?;
            pieces.push(ui.scene().node(
                Style::default(),
                solid(Some(colors.selected), None, theme.px(2.0)),
                &[selected],
            )?);
            pieces.push(segment(
                ui,
                theme,
                &shown(&text[selection.end..]),
                colors.text,
                None,
            )?);
        } else {
            let caret = self.edit.caret();
            pieces.push(segment(
                ui,
                theme,
                &shown(&text[..caret]),
                colors.text,
                None,
            )?);
            pieces.push(segment(
                ui,
                theme,
                preedit,
                colors.text,
                Some(Decoration::Underline),
            )?);
            pieces.push(self.caret(ui, theme)?);
            pieces.push(segment(
                ui,
                theme,
                &shown(&text[caret..]),
                colors.text,
                None,
            )?);
        }
        Ok(pieces)
    }

    fn caret<M>(&self, ui: &mut Ui<M>, theme: &WidgetTheme) -> Result<NodeId, RenderError> {
        ui.scene().node(
            Style {
                size: size(Some(theme.px(1.5)), Some(theme.px(16.0))),
                flex_shrink: 0.0,
                ..Style::default()
            },
            Visual::solid(theme.colors.accent, 0.0),
            &[],
        )
    }
}

/// 一段文字；空串也建（占位为零宽），各段的位置才稳定。
fn segment<M>(
    ui: &mut Ui<M>,
    theme: &WidgetTheme,
    text: &str,
    color: crate::color::Color,
    decoration: Option<Decoration>,
) -> Result<NodeId, RenderError> {
    let mut style = theme.text(theme.font.size - 1.0, 400, color);
    if let Some(decoration) = decoration {
        style = style.decorated(decoration);
    }
    ui.scene().node(
        Style {
            flex_shrink: 0.0,
            ..Style::default()
        },
        Visual::Text {
            text: text.to_owned(),
            style,
        },
        &[],
    )
}

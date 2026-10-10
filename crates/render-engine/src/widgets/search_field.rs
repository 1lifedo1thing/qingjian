//! 搜索框：放大镜加占位文字。文字输入那一期之前只是样子，悬停有反馈、还不能打字。

use std::hash::Hash;
use std::sync::Arc;

use taffy::{NodeId, Style};

use super::WidgetTheme;
use super::parts::{centered, label, over, padding, row, size, solid};
use crate::animation::Easing;
use crate::error::RenderError;
use crate::scene::{Icon, Visual};
use crate::ui::{Interact, Ui};

pub struct SearchField<'a> {
    placeholder: &'a str,

    icon: Option<Arc<dyn Icon>>,
}

impl<'a> SearchField<'a> {
    pub fn new(placeholder: &'a str) -> Self {
        Self {
            placeholder,
            icon: None,
        }
    }

    pub fn icon(self, icon: Arc<dyn Icon>) -> Self {
        Self {
            icon: Some(icon),
            ..self
        }
    }

    pub fn build<M: Clone + 'static>(
        self,
        ui: &mut Ui<M>,
        theme: &WidgetTheme,
        local: impl Hash,
    ) -> Result<NodeId, RenderError> {
        let key = ui.key(local);
        let state = ui.state(key);
        let colors = &theme.colors;
        let mut children = Vec::new();
        if let Some(icon) = self.icon {
            let side = theme.px(13.0);
            children.push(ui.scene().node(
                centered(side, side),
                Visual::Icon {
                    icon,
                    size: side,
                    color: colors.secondary,
                },
                &[],
            )?);
        }
        children.push(label(
            ui,
            theme,
            self.placeholder,
            theme.font.size - 0.5,
            400,
            colors.secondary,
        )?);
        let fill = if state.hovered {
            over(colors.well, colors.hover)
        } else {
            colors.well
        };
        let node = ui.scene().node(
            Style {
                padding: padding(theme, 0.0, 9.0),
                size: size(None, Some(theme.px(theme.control_height + 4.0))),
                ..row(theme, 7.0)
            },
            solid(Some(fill), None, theme.px(theme.radius + 1.0)),
            &children,
        )?;
        ui.transition(node, key, theme.hover, Easing::Ease);
        ui.interact(node, key, Interact::new());
        Ok(node)
    }
}

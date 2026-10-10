//! 一行设置：左边标题（可带一行说明），右边控件，控件下面还能挂附加项（勾选框、链接）。
//! 控件很宽的（主题卡片）用 [`SettingRow::stacked`] 放到标题下面。

use taffy::{AlignItems, JustifyContent, NodeId, Style};

use super::WidgetTheme;
use super::parts::{column, label, padding, row};
use crate::error::RenderError;
use crate::scene::Visual;
use crate::ui::Ui;

pub struct SettingRow<'a> {
    title: &'a str,

    detail: Option<&'a str>,

    control: Option<NodeId>,

    below: Vec<NodeId>,

    stacked: bool,
}

impl<'a> SettingRow<'a> {
    pub fn new(title: &'a str) -> Self {
        Self {
            title,
            detail: None,
            control: None,
            below: Vec::new(),
            stacked: false,
        }
    }

    /// 标题下面一行次要色的说明；能不写就不写。
    pub fn detail(self, detail: &'a str) -> Self {
        Self {
            detail: Some(detail),
            ..self
        }
    }

    pub fn control(self, node: NodeId) -> Self {
        Self {
            control: Some(node),
            ..self
        }
    }

    /// 挂在控件下面、右对齐。
    pub fn below(mut self, node: NodeId) -> Self {
        self.below.push(node);
        self
    }

    /// 控件放到标题下面、左对齐占满一行。
    pub fn stacked(self) -> Self {
        Self {
            stacked: true,
            ..self
        }
    }

    pub fn build<M>(self, ui: &mut Ui<M>, theme: &WidgetTheme) -> Result<NodeId, RenderError> {
        let colors = &theme.colors;
        let mut texts = vec![label(
            ui,
            theme,
            self.title,
            theme.font.size,
            500,
            colors.text,
        )?];
        if let Some(detail) = self.detail {
            texts.push(label(
                ui,
                theme,
                detail,
                theme.small,
                400,
                colors.secondary,
            )?);
        }
        let heading = ui.scene().node(column(theme, 2.0), Visual::Group, &texts)?;
        if self.stacked {
            let mut children = vec![heading];
            children.extend(self.control);
            children.extend(self.below);
            return ui.scene().node(
                Style {
                    padding: padding(theme, 12.0, 0.0),
                    ..column(theme, 10.0)
                },
                Visual::Group,
                &children,
            );
        }
        let mut trailing: Vec<NodeId> = self.control.into_iter().collect();
        trailing.extend(self.below);
        let trailing = ui.scene().node(
            Style {
                align_items: Some(AlignItems::FLEX_END),
                ..column(theme, 8.0)
            },
            Visual::Group,
            &trailing,
        )?;
        ui.scene().node(
            Style {
                padding: padding(theme, 10.0, 0.0),
                justify_content: Some(JustifyContent::SPACE_BETWEEN),
                min_size: taffy::Size {
                    width: taffy::LengthPercentageAuto::auto(),
                    height: taffy::LengthPercentageAuto::length(
                        theme.px(theme.control_height + 20.0),
                    ),
                },
                ..row(theme, 16.0)
            },
            Visual::Group,
            &[heading, trailing],
        )
    }
}

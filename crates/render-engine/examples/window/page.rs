//! 设置页的视图函数：左侧栏 + 右侧分组卡片（标题、说明、开关）+ 「恢复默认」按钮。
//!
//! 控件按 [`ElementState`] 挑样式（悬停、按下、键盘焦点环）；尺寸按点写，建树时乘 `scale`。

use render_engine::taffy::{
    AlignItems, Dimension, FlexDirection, JustifyContent, LengthPercentage, LengthPercentageAuto,
    NodeId, Position, Rect, Size, Style,
};
use render_engine::{
    BoxPaint, Color, Effect, EffectKind, ElementState, Fill, FontSpec, FontWeight, Interact,
    KeyCode, RenderError, TextStyle, Ui, Visual,
};

use crate::demo::{Demo, GROUPS, Message, PAGES, Row};

pub struct Palette {
    window: Color,

    sidebar: Color,

    /// 侧栏项悬停、按下时垫的底。
    hover: Color,

    pressed: Color,

    selected: Color,

    card: Color,

    border: Color,

    label: Color,

    secondary: Color,

    accent: Color,

    accent_hover: Color,

    track_off: Color,

    track_off_hover: Color,

    knob: Color,

    focus: Color,

    button: Color,

    button_hover: Color,

    button_pressed: Color,
}

impl Palette {
    pub const LIGHT: Self = Self {
        window: Color::rgb(0xF5, 0xF5, 0xF7),
        sidebar: Color::rgb(0xEB, 0xEB, 0xEE),
        hover: Color::gray(0, 12),
        pressed: Color::gray(0, 30),
        selected: Color::gray(0, 22),
        card: Color::rgb(0xFF, 0xFF, 0xFF),
        border: Color::gray(0, 18),
        label: Color::gray(0, 222),
        secondary: Color::gray(0, 128),
        accent: Color::rgb(0x0A, 0x84, 0xFF),
        accent_hover: Color::rgb(0x30, 0x97, 0xFF),
        track_off: Color::gray(0, 30),
        track_off_hover: Color::gray(0, 46),
        knob: Color::rgb(0xFF, 0xFF, 0xFF),
        focus: Color::rgba(0x0A, 0x84, 0xFF, 150),
        button: Color::rgb(0xFF, 0xFF, 0xFF),
        button_hover: Color::rgb(0xF2, 0xF2, 0xF4),
        button_pressed: Color::rgb(0xE2, 0xE2, 0xE6),
    };

    pub const DARK: Self = Self {
        window: Color::rgb(0x1E, 0x1E, 0x20),
        sidebar: Color::rgb(0x28, 0x28, 0x2B),
        hover: Color::gray(255, 14),
        pressed: Color::gray(255, 34),
        selected: Color::gray(255, 26),
        card: Color::rgb(0x2C, 0x2C, 0x2F),
        border: Color::gray(255, 20),
        label: Color::gray(255, 222),
        secondary: Color::gray(255, 140),
        accent: Color::rgb(0x0A, 0x84, 0xFF),
        accent_hover: Color::rgb(0x30, 0x97, 0xFF),
        track_off: Color::gray(255, 40),
        track_off_hover: Color::gray(255, 58),
        knob: Color::rgb(0xF2, 0xF2, 0xF2),
        focus: Color::rgba(0x40, 0x9C, 0xFF, 170),
        button: Color::rgb(0x3A, 0x3A, 0x3E),
        button_hover: Color::rgb(0x44, 0x44, 0x48),
        button_pressed: Color::rgb(0x52, 0x52, 0x57),
    };
}

/// 建整窗的场景树，根节点就是窗口大小（像素）。
pub fn view(
    ui: &mut Ui<Message>,
    demo: &Demo,
    scale: f32,
    (width, height): (u32, u32),
) -> Result<NodeId, RenderError> {
    let page = Page {
        palette: if demo.dark {
            &Palette::DARK
        } else {
            &Palette::LIGHT
        },
        scale,
        demo,
    };
    let sidebar = page.sidebar(ui)?;
    let content = page.content(ui)?;
    let root = ui.scene().node(
        Style {
            size: Size {
                width: Dimension::length(width as f32),
                height: Dimension::length(height as f32),
            },
            flex_direction: FlexDirection::Row,
            ..Style::default()
        },
        Visual::solid(page.palette.window, 0.0),
        &[sidebar, content],
    )?;
    let key = ui.key("root");
    ui.interact(
        root,
        key,
        Interact::new().on_key(|press| (press.key == KeyCode::Escape).then_some(Message::Quit)),
    );
    Ok(root)
}

struct Page<'a> {
    palette: &'a Palette,

    scale: f32,

    demo: &'a Demo,
}

impl Page<'_> {
    fn px(&self, points: f32) -> f32 {
        points * self.scale
    }

    fn padding(&self, vertical: f32, horizontal: f32) -> Rect<LengthPercentage> {
        let (v, h) = (
            LengthPercentage::length(self.px(vertical)),
            LengthPercentage::length(self.px(horizontal)),
        );
        Rect {
            left: h,
            right: h,
            top: v,
            bottom: v,
        }
    }

    fn gap(&self, points: f32) -> Size<LengthPercentage> {
        Size {
            width: LengthPercentage::length(self.px(points)),
            height: LengthPercentage::length(self.px(points)),
        }
    }

    /// 键盘焦点环：画成内侧边框。
    fn ring(&self, state: ElementState) -> Option<(f32, Color)> {
        state.focus_ring.then(|| (self.px(2.0), self.palette.focus))
    }

    fn text(
        &self,
        ui: &mut Ui<Message>,
        content: &str,
        points: f32,
        weight: u16,
        color: Color,
    ) -> Result<NodeId, RenderError> {
        let font = FontSpec {
            weight: FontWeight(weight),
            ..FontSpec::new(points, points * 1.35)
        };
        let style = TextStyle::new(font.scaled(self.scale), points, color, 1.0);
        ui.scene().node(
            Style::default(),
            Visual::Text {
                text: content.to_owned(),
                style,
            },
            &[],
        )
    }

    fn sidebar(&self, ui: &mut Ui<Message>) -> Result<NodeId, RenderError> {
        let mut items = Vec::new();
        for (index, name) in PAGES.iter().enumerate() {
            items.push(self.sidebar_item(ui, index, name)?);
        }
        ui.scene().node(
            Style {
                size: Size {
                    width: Dimension::length(self.px(180.0)),
                    height: Dimension::auto(),
                },
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                gap: self.gap(2.0),
                padding: self.padding(16.0, 10.0),
                ..Style::default()
            },
            Visual::solid(self.palette.sidebar, 0.0),
            &items,
        )
    }

    fn sidebar_item(
        &self,
        ui: &mut Ui<Message>,
        index: usize,
        name: &str,
    ) -> Result<NodeId, RenderError> {
        let key = ui.scope("sidebar", |ui| ui.key(index));
        let state = ui.state(key);
        let selected = index == self.demo.page;
        let fill = if state.pressed {
            Some(self.palette.pressed)
        } else if selected {
            Some(self.palette.selected)
        } else if state.hovered {
            Some(self.palette.hover)
        } else {
            None
        };
        let weight = if selected { 600 } else { 400 };
        let label = self.text(ui, name, 13.0, weight, self.palette.label)?;
        let node = ui.scene().node(
            Style {
                padding: self.padding(6.0, 10.0),
                ..Style::default()
            },
            Visual::Box(BoxPaint {
                fill: fill.map(Fill::Solid),
                border: self.ring(state),
                radius: self.px(6.0),
            }),
            &[label],
        )?;
        ui.interact(
            node,
            key,
            Interact::new().on_click(Message::Select(index)).focusable(),
        );
        Ok(node)
    }

    fn content(&self, ui: &mut Ui<Message>) -> Result<NodeId, RenderError> {
        let title = PAGES.get(self.demo.page).copied().unwrap_or_default();
        let mut children = vec![self.text(ui, title, 22.0, 700, self.palette.label)?];
        if self.demo.page == 1 {
            let mut index = 0;
            for (title, rows) in GROUPS {
                children.push(self.text(ui, title, 12.0, 600, self.palette.secondary)?);
                children.push(self.card(ui, rows, index)?);
                index += rows.len();
            }
            let reset = self.button(ui, "恢复默认", Message::Reset)?;
            children.push(ui.scene().node(
                Style {
                    flex_direction: FlexDirection::Row,
                    justify_content: Some(JustifyContent::FLEX_END),
                    ..Style::default()
                },
                Visual::Group,
                &[reset],
            )?);
        } else {
            children.push(self.text(ui, "这一页还没做", 13.0, 400, self.palette.secondary)?);
        }
        ui.scene().node(
            Style {
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                gap: self.gap(10.0),
                padding: self.padding(20.0, 24.0),
                ..Style::default()
            },
            Visual::Group,
            &children,
        )
    }

    /// 一组设置行；`first` 是第一行开关在 [`Demo::switches`] 里的下标。
    fn card(
        &self,
        ui: &mut Ui<Message>,
        rows: &[Row],
        first: usize,
    ) -> Result<NodeId, RenderError> {
        let mut children = Vec::new();
        for (offset, (label, detail, _)) in rows.iter().enumerate() {
            if offset > 0 {
                children.push(ui.scene().node(
                    Style {
                        size: Size {
                            width: Dimension::auto(),
                            height: Dimension::length(self.px(1.0)),
                        },
                        ..Style::default()
                    },
                    Visual::solid(self.palette.border, 0.0),
                    &[],
                )?);
            }
            children.push(self.row(ui, label, detail, first + offset)?);
        }
        ui.scene().node(
            Style {
                flex_direction: FlexDirection::Column,
                padding: self.padding(0.0, 14.0),
                ..Style::default()
            },
            Visual::Box(BoxPaint {
                fill: Some(Fill::Solid(self.palette.card)),
                border: Some((self.px(1.0), self.palette.border)),
                radius: self.px(10.0),
            }),
            &children,
        )
    }

    fn row(
        &self,
        ui: &mut Ui<Message>,
        label: &str,
        detail: &str,
        index: usize,
    ) -> Result<NodeId, RenderError> {
        let title = self.text(ui, label, 13.0, 400, self.palette.label)?;
        let description = self.text(ui, detail, 11.0, 400, self.palette.secondary)?;
        let texts = ui.scene().node(
            Style {
                flex_direction: FlexDirection::Column,
                gap: self.gap(2.0),
                ..Style::default()
            },
            Visual::Group,
            &[title, description],
        )?;
        let switch = self.switch(ui, index)?;
        ui.scene().node(
            Style {
                flex_direction: FlexDirection::Row,
                justify_content: Some(JustifyContent::SPACE_BETWEEN),
                align_items: Some(AlignItems::CENTER),
                padding: self.padding(10.0, 0.0),
                ..Style::default()
            },
            Visual::Group,
            &[texts, switch],
        )
    }

    /// 开关：悬停时轨道加深，按下时旋钮拉宽（同 macOS / iOS），键盘焦点时外圈画焦点环。
    fn switch(&self, ui: &mut Ui<Message>, index: usize) -> Result<NodeId, RenderError> {
        let key = ui.scope("switch", |ui| ui.key(index));
        let state = ui.state(key);
        let on = self.demo.switches.get(index).copied().unwrap_or(false);
        let (track_w, track_h, inset) = (self.px(38.0), self.px(22.0), self.px(2.0));
        let knob_h = track_h - inset * 2.0;
        let knob_w = if state.pressed {
            knob_h + self.px(5.0)
        } else {
            knob_h
        };
        let knob = ui.scene().node(
            Style {
                position: Position::Absolute,
                size: Size {
                    width: Dimension::length(knob_w),
                    height: Dimension::length(knob_h),
                },
                inset: Rect {
                    left: LengthPercentageAuto::length(if on {
                        track_w - inset - knob_w
                    } else {
                        inset
                    }),
                    right: LengthPercentageAuto::auto(),
                    top: LengthPercentageAuto::length(inset),
                    bottom: LengthPercentageAuto::auto(),
                },
                ..Style::default()
            },
            Visual::solid(self.palette.knob, knob_h / 2.0),
            &[],
        )?;
        ui.scene().set_effects(
            knob,
            vec![Effect {
                kind: EffectKind::DropShadow,
                x: 0.0,
                y: self.px(1.0),
                blur: self.px(2.0),
                spread: 0.0,
                color: Color::gray(0, 60),
            }],
        );
        let mut children = vec![knob];
        if let Some(border) = self.ring(state) {
            let outset = LengthPercentageAuto::length(-self.px(3.0));
            children.push(ui.scene().node(
                Style {
                    position: Position::Absolute,
                    inset: Rect {
                        left: outset,
                        right: outset,
                        top: outset,
                        bottom: outset,
                    },
                    ..Style::default()
                },
                Visual::Box(BoxPaint {
                    fill: None,
                    border: Some(border),
                    radius: track_h / 2.0 + self.px(3.0),
                }),
                &[],
            )?);
        }
        let track = match (on, state.hovered) {
            (true, false) => self.palette.accent,
            (true, true) => self.palette.accent_hover,
            (false, false) => self.palette.track_off,
            (false, true) => self.palette.track_off_hover,
        };
        let node = ui.scene().node(
            Style {
                size: Size {
                    width: Dimension::length(track_w),
                    height: Dimension::length(track_h),
                },
                flex_shrink: 0.0,
                ..Style::default()
            },
            Visual::solid(track, track_h / 2.0),
            &children,
        )?;
        ui.interact(
            node,
            key,
            Interact::new().on_click(Message::Toggle(index)).focusable(),
        );
        Ok(node)
    }

    fn button(
        &self,
        ui: &mut Ui<Message>,
        label: &str,
        message: Message,
    ) -> Result<NodeId, RenderError> {
        let key = ui.key(label);
        let state = ui.state(key);
        let fill = if state.pressed {
            self.palette.button_pressed
        } else if state.hovered {
            self.palette.button_hover
        } else {
            self.palette.button
        };
        let text = self.text(ui, label, 13.0, 400, self.palette.label)?;
        let node = ui.scene().node(
            Style {
                padding: self.padding(5.0, 14.0),
                ..Style::default()
            },
            Visual::Box(BoxPaint {
                fill: Some(Fill::Solid(fill)),
                border: Some(
                    self.ring(state)
                        .unwrap_or((self.px(1.0), self.palette.border)),
                ),
                radius: self.px(6.0),
            }),
            &[text],
        )?;
        ui.interact(node, key, Interact::new().on_click(message).focusable());
        Ok(node)
    }
}

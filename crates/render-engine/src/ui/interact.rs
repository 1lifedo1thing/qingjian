//! 一个元素怎么响应：点击产出的消息、按键处理、能不能拿焦点、是否禁用。

use super::event::KeyPress;

/// 按键处理：处理了就返回消息，没处理返回 `None` 交给上级。
type KeyHandler<M> = Box<dyn Fn(&KeyPress) -> Option<M>>;

/// 什么都不设也有用：元素会记悬停状态。
pub struct Interact<M> {
    pub(super) click: Option<M>,

    pub(super) key: Option<KeyHandler<M>>,

    pub(super) focusable: bool,

    pub(super) disabled: bool,
}

impl<M> Default for Interact<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> Interact<M> {
    pub fn new() -> Self {
        Self {
            click: None,
            key: None,
            focusable: false,
            disabled: false,
        }
    }

    /// 点击（按下抬起都在它上面），以及有焦点时按空格 / 回车。
    pub fn on_click(self, message: M) -> Self {
        Self {
            click: Some(message),
            ..self
        }
    }

    pub fn on_key(self, handler: impl Fn(&KeyPress) -> Option<M> + 'static) -> Self {
        Self {
            key: Some(Box::new(handler)),
            ..self
        }
    }

    /// 能用 Tab 走到、点击时拿焦点。
    pub fn focusable(self) -> Self {
        Self {
            focusable: true,
            ..self
        }
    }

    /// 禁用：不产出消息、不拿焦点，但仍挡住底下的元素。
    pub fn disabled(self, disabled: bool) -> Self {
        Self { disabled, ..self }
    }

    /// 能接点击：有点击消息且没禁用。
    pub(super) fn clickable(&self) -> bool {
        self.click.is_some() && !self.disabled
    }

    pub(super) fn takes_focus(&self) -> bool {
        self.focusable && !self.disabled
    }
}

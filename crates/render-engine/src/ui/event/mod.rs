//! 交给交互层的输入事件，与窗口库无关：宿主把系统事件翻成它们（坐标已是像素，原点在根节点左上角）。

mod key_code;
mod key_press;
mod modifiers;

pub use key_code::KeyCode;
pub use key_press::KeyPress;
pub use modifiers::Modifiers;

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    PointerMoved {
        x: f32,
        y: f32,
    },

    /// 指针离开窗口。
    PointerLeft,

    /// 主键按下 / 抬起；位置以最近一次 `PointerMoved` 为准。触屏的按下前宿主先补一个 `PointerMoved`。
    PointerDown,

    PointerUp,

    Key(KeyPress),
}

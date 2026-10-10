//! 按键时按住的修饰键。`command` 是 macOS 的 ⌘、其他平台的 Ctrl，快捷键按它判断。

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub shift: bool,

    pub alt: bool,

    pub command: bool,
}

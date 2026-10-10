//! 控件产出的消息：一条对应配置文件里的一项改动（或界面自己的状态）。

use qingjian_platform::{Appearance, CandidateRenderer, LayoutMode, PreeditMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeKind {
    Candidate,
    Annotation,
}

#[derive(Debug, Clone)]
pub enum Message {
    Page(usize),
    Appearance(Appearance),
    Theme(String),
    Layout(LayoutMode),
    /// 横排时 ↑ / ↓ 展开成多行（只有 macOS 用）。
    Grid,
    Preedit(PreeditMode),
    Renderer(CandidateRenderer),
    /// 打开字体下拉菜单。
    OpenFonts,
    /// 选了一个字族；`None` 是系统字体。
    PickFont(Option<String>),
    /// 关掉弹出菜单（点了菜单外面、按了 Esc）。
    CloseMenu,
    Size(SizeKind, i32),
    ResetSizes,
    Animations,
    /// 悬浮状态条（只有 Windows 有）。
    StatusBar,
    Quit,
}

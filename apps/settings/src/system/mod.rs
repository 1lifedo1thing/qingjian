//! 与操作系统打交道的部分：文件位置、本机字体、打开文件与目录的按钮动作、Windows 上只开一个窗口。

pub mod action;
pub mod fonts;
pub mod paths;
#[cfg(windows)]
pub mod single_instance;

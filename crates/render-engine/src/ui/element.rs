//! 布局之后的一个可交互元素：在根坐标里的矩形与最近的可交互上级。

use taffy::NodeId;

use super::{Interact, Key};

pub struct Element<M> {
    pub key: Key,

    pub node: NodeId,

    pub interact: Interact<M>,

    /// 左、上、宽、高（像素）。
    pub rect: (f32, f32, f32, f32),

    /// 最近的可交互上级在元素表里的下标。
    pub parent: Option<usize>,
}

impl<M> Element<M> {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        let (left, top, width, height) = self.rect;
        x >= left && x < left + width && y >= top && y < top + height
    }
}

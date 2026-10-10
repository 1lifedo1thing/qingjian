//! 元素的稳定 key：每帧重建整棵树，`NodeId` 每次都变，悬停、按下、焦点按它跨帧认回来。

use std::hash::{DefaultHasher, Hash, Hasher};

/// 由路径哈希出来：父 key 加上一段本地名（组件名、列表项的数据 id、序号）。同一进程里同一路径总是同一个值。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Key(u64);

impl Key {
    /// 根作用域。
    pub const ROOT: Self = Self(0);

    pub fn child(self, local: impl Hash) -> Self {
        let mut hasher = DefaultHasher::new();
        self.0.hash(&mut hasher);
        local.hash(&mut hasher);
        Self(hasher.finish())
    }
}

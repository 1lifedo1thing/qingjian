//! 一帧的建树入口：包着场景树，管 key 的作用域，收集可交互元素。

use std::hash::Hash;
use std::time::Duration;

use taffy::NodeId;

use super::{ElementState, Elements, Interact, Interaction, Key};
use crate::animation::Easing;
use crate::scene::Scene;

pub struct Ui<'a, M> {
    scene: Scene,

    interaction: &'a Interaction<M>,

    /// 当前作用域，[`Self::key`] 在它下面派生。
    scope: Key,

    elements: Vec<(Key, NodeId, Interact<M>)>,
}

impl<'a, M> Ui<'a, M> {
    pub fn new(interaction: &'a Interaction<M>) -> Self {
        Self {
            scene: Scene::new(),
            interaction,
            scope: Key::ROOT,
            elements: Vec::new(),
        }
    }

    pub fn scene(&mut self) -> &mut Scene {
        &mut self.scene
    }

    /// 当前作用域下的 key。同一作用域里本地名不能重复，列表项用数据的 id 当本地名，排序变了状态跟着走。
    pub fn key(&self, local: impl Hash) -> Key {
        self.scope.child(local)
    }

    /// 在子作用域里建：组件里的 key 都派生在它下面，同一个组件用几次也不会撞。
    pub fn scope<R>(&mut self, local: impl Hash, build: impl FnOnce(&mut Self) -> R) -> R {
        let outer = self.scope;
        self.scope = outer.child(local);
        let result = build(self);
        self.scope = outer;
        result
    }

    pub fn state(&self, key: Key) -> ElementState {
        self.interaction.state(key)
    }

    /// 登记 `node` 可交互；矩形取它布局后的盒子。
    pub fn interact(&mut self, node: NodeId, key: Key, interact: Interact<M>) {
        self.elements.push((key, node, interact));
    }

    /// `node` 的位置、大小、不透明度、颜色与上一帧同 key 的节点不同时，从旧值平滑过渡过去。
    /// 交给 `Stage::present` 画才有过渡，`Stage::rasterize` 直接画终点。
    pub fn transition(&mut self, node: NodeId, key: Key, duration: Duration, easing: Easing) {
        self.scene
            .set_transition(node, key.transition_name(), duration, easing);
    }

    pub fn finish(self) -> (Scene, Elements<M>) {
        (self.scene, Elements(self.elements))
    }
}

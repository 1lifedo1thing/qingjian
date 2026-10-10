//! 跨帧的交互状态与事件处理：上一帧的可交互元素（按画序）、指针位置、悬停路径、按下与焦点。

mod keyboard;
mod pointer;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use taffy::NodeId;

use super::element::Element;
use super::{ElementState, Elements, Event, Key, Response};
use crate::error::RenderError;
use crate::scene::Scene;

pub struct Interaction<M> {
    /// 上一帧的可交互元素，按画序（先父后子），后面的盖在前面的上面。
    elements: Vec<Element<M>>,

    /// key → 元素下标；同一帧 key 重复时只认第一个。
    index: HashMap<Key, usize>,

    /// 指针位置（根坐标像素）；离开窗口为 `None`。
    pointer: Option<(f32, f32)>,

    /// 指针下的元素与它的可交互上级，从里到外。
    hovered: Vec<Key>,

    /// 按下时捕获指针的元素，抬起前一直归它。
    pressed: Option<Key>,

    focused: Option<Key>,

    /// 焦点是键盘带来的，画焦点环。
    focus_visible: bool,
}

impl<M> Default for Interaction<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> Interaction<M> {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            index: HashMap::new(),
            pointer: None,
            hovered: Vec::new(),
            pressed: None,
            focused: None,
            focus_visible: false,
        }
    }

    pub fn state(&self, key: Key) -> ElementState {
        let hovered = self.hovered.contains(&key);
        let focused = self.focused == Some(key);
        ElementState {
            hovered,
            pressed: hovered && self.pressed == Some(key),
            focused,
            focus_ring: focused && self.focus_visible,
        }
    }

    /// 元素上一帧在根坐标里的矩形（左、上、宽、高，像素）：弹出层定位、测试里找点击位置用。
    pub fn rect(&self, key: Key) -> Option<(f32, f32, f32, f32)> {
        self.element(key).map(|element| element.rect)
    }

    /// 收下新一帧的元素（`scene` 已布局）。不在 `root` 下的元素丢掉；焦点、按下的元素没了（或焦点元素被禁用）就清掉，悬停按指针位置重新命中。
    /// 返回交互状态是否因此变了（布局挪动让指针换了元素），变了要再重建一次。
    pub fn attach(
        &mut self,
        scene: &Scene,
        root: NodeId,
        elements: Elements<M>,
    ) -> Result<bool, RenderError> {
        let mut pending: HashMap<NodeId, (Key, super::Interact<M>)> = elements
            .0
            .into_iter()
            .map(|(key, node, interact)| (node, (key, interact)))
            .collect();
        self.elements.clear();
        self.index.clear();
        let mut by_node: HashMap<NodeId, usize> = HashMap::new();
        for (node, rect) in scene.paint_order(root)? {
            let Some((key, interact)) = pending.remove(&node) else {
                continue;
            };
            let position = self.elements.len();
            match self.index.entry(key) {
                Entry::Vacant(entry) => {
                    entry.insert(position);
                }
                Entry::Occupied(_) => {
                    tracing::warn!(?key, "同一帧里 key 重复，命中与状态只认第一个");
                }
            }
            by_node.insert(node, position);
            self.elements.push(Element {
                key,
                node,
                interact,
                rect,
                parent: None,
            });
        }
        for element in &mut self.elements {
            let mut current = scene.parent(element.node);
            while let Some(node) = current {
                if let Some(&position) = by_node.get(&node) {
                    element.parent = Some(position);
                    break;
                }
                current = scene.parent(node);
            }
        }

        let mut changed = false;
        if self.focused.is_some_and(|key| {
            !self
                .element(key)
                .is_some_and(|element| element.interact.takes_focus())
        }) {
            self.focused = None;
            changed = true;
        }
        if self
            .pressed
            .is_some_and(|key| !self.index.contains_key(&key))
        {
            self.pressed = None;
            changed = true;
        }
        changed |= self.rehover();
        Ok(changed)
    }

    /// 处理一个输入事件。
    pub fn handle(&mut self, event: &Event) -> Response<M>
    where
        M: Clone,
    {
        match event {
            Event::PointerMoved { x, y } => self.pointer_moved(Some((*x, *y))),
            Event::PointerLeft => self.pointer_moved(None),
            Event::PointerDown => self.pointer_down(),
            Event::PointerUp => self.pointer_up(),
            Event::Key(press) => self.key(press),
        }
    }

    /// 按指针位置重新算悬停路径，返回是否变了。
    fn rehover(&mut self) -> bool {
        let hovered = match self.pointer {
            Some((x, y)) => self.hit(x, y),
            None => Vec::new(),
        };
        let changed = hovered != self.hovered;
        self.hovered = hovered;
        changed
    }

    /// 命中：画序最后（最上面）的、矩形包含这一点的元素，加上它的可交互上级，从里到外。
    fn hit(&self, x: f32, y: f32) -> Vec<Key> {
        let target = self
            .elements
            .iter()
            .rposition(|element| element.contains(x, y));
        self.chain(target)
            .map(|position| self.elements[position].key)
            .collect()
    }

    /// 从 `start` 往上的元素下标。
    fn chain(&self, start: Option<usize>) -> impl Iterator<Item = usize> + '_ {
        std::iter::successors(start, |&position| self.elements[position].parent)
    }

    fn element(&self, key: Key) -> Option<&Element<M>> {
        self.index
            .get(&key)
            .map(|&position| &self.elements[position])
    }
}

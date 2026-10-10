//! 控件产出的消息：多数是「把配置文件里的某一项改成某个值」，其余是界面自己的状态与按钮动作。

use render_engine::Edit;

use crate::action::Action;
use crate::page_id::PageId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeKind {
    Candidate,
    Annotation,
}

/// 云服务页的输入框。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldId {
    BaseUrl,
    Model,
    ApiKey,
}

impl FieldId {
    pub const ALL: [Self; 3] = [Self::BaseUrl, Self::Model, Self::ApiKey];

    /// 配置文件 `[predict]` 里的键。
    pub fn key(self) -> &'static str {
        match self {
            Self::BaseUrl => "base_url",
            Self::Model => "model",
            Self::ApiKey => "api_key",
        }
    }
}

/// 下拉框弹出的菜单。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Popup {
    Font,
    Scheme,
    LearningLanguage,
}

#[derive(Debug, Clone)]
pub enum Message {
    Page(PageId),
    /// 原地改配置文件里的 `[分节] 键 = 值`。
    Set(&'static str, &'static str, toml_edit::Value),
    /// 同上，值是字符串数组。
    SetList(&'static str, &'static str, Vec<String>),
    OpenMenu(Popup),
    /// 关掉弹出菜单（点了菜单外面、按了 Esc）。
    CloseMenu,
    Size(SizeKind, i32),
    ResetSizes,
    Run(Action),
    /// 输入框里的一次编辑（打字、挪光标、输入法组字）。
    Edit(FieldId, Edit),
    /// 回车或离开输入框：存进配置。
    Commit(FieldId),
    /// Esc：扔掉没存的改动。
    Cancel(FieldId),
    TestCloud,
    /// 展开 / 收起「关于」页的数据来源。
    ToggleSources,
    Quit,
}

impl Message {
    pub fn set(
        section: &'static str,
        key: &'static str,
        value: impl Into<toml_edit::Value>,
    ) -> Self {
        Self::Set(section, key, value.into())
    }
}

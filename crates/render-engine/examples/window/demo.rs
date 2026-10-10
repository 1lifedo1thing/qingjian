//! 样例应用的状态与消息：选中哪一页、各开关开没开。交互层产出消息，`update` 是唯一改状态的地方。

/// 一行设置：标题、说明、默认开不开。
pub type Row = (&'static str, &'static str, bool);

pub const PAGES: [&str; 8] = [
    "通用",
    "候选窗口",
    "外观",
    "拼音",
    "词库",
    "云联想",
    "快捷键",
    "关于",
];

/// 「候选窗口」页的分组。
pub const GROUPS: [(&str, &[Row]); 3] = [
    (
        "候选窗口",
        &[
            ("横排显示候选", "关掉后候选竖着排成一列", true),
            ("显示释义", "候选旁边显示英文释义与词汇等级", true),
            ("跟随光标", "候选窗贴着输入位置，而不是固定在屏幕底部", true),
            ("显示序号", "候选前面标出 1–9", false),
        ],
    ),
    (
        "动画",
        &[
            ("候选切换动画", "翻页、高亮移动时平滑过渡", true),
            ("跟随系统「减少动态效果」", "系统开了就不播动画", true),
        ],
    ),
    (
        "字体",
        &[
            ("使用系统界面字体", "关掉后可以指定候选字体", true),
            ("光学字号", "按字号挑字形，小字更清楚", false),
        ],
    ),
];

#[derive(Debug, Clone)]
pub enum Message {
    Select(usize),
    Toggle(usize),
    Reset,
    Quit,
}

pub struct Demo {
    pub page: usize,

    /// 各开关，按 [`GROUPS`] 里的先后摊平。
    pub switches: Vec<bool>,

    pub dark: bool,
}

impl Demo {
    pub fn new(dark: bool) -> Self {
        Self {
            page: 1,
            switches: defaults(),
            dark,
        }
    }

    /// 返回 `false` 表示要退出。
    pub fn update(&mut self, message: Message) -> bool {
        match message {
            Message::Select(page) => self.page = page,
            Message::Toggle(index) => {
                if let Some(on) = self.switches.get_mut(index) {
                    *on = !*on;
                }
            }
            Message::Reset => self.switches = defaults(),
            Message::Quit => return false,
        }
        true
    }
}

fn defaults() -> Vec<bool> {
    GROUPS
        .iter()
        .flat_map(|(_, rows)| rows.iter().map(|(_, _, on)| *on))
        .collect()
}

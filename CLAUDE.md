# CLAUDE.md

自绘 UI 引擎，单个 crate 在 `crates/render-engine`。公开 API 只从 `lib.rs` 重新导出，各模块本身私有。

## 边界

- 引擎里不出现任何具体应用的概念（候选、拼音、主题文件格式、帧数据）。使用方自己的数据格式与建树留在使用方，
  转成引擎的类型（`Scene` / `Visual` / `Effect` / `Keyframes` / `TextStyle`）再交进来；需要使用方定制的画法走 trait（如 `Icon`）。
- 场景树里一律是像素；点 → 像素的倍数由使用方乘。
- 改动后除了本仓库的测试，还要在青简仓库跑 `cargo test -p qingjian-render`：它的快照测试逐像素比对候选窗截图，是引擎的回归基准。

## 常用命令

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all
cargo clippy --target x86_64-pc-windows-gnu --all-targets -- -D warnings   # DirectWrite 部分只在 Windows 编
```

## 约定

- 一个类型一个文件；有子模块的模块用 `foo/mod.rs`；同词干的兄弟文件收进目录，不用文件名前缀分组。
- 单文件不超过 800 行，目标 500 行以内；导入写精确路径，不用 glob（测试模块里的 `use super::*` 除外）。
- 标识符英文，注释与文档中文；`thiserror` 的 `#[error]` 文案英文，日志中文。新文件要有 `//!` 文件头。
- 注释只写维护时用得上的：文件头说它是什么、怎么用；代码里只解释读代码看不出来的约束与原因。不写复述代码、改动经过、装饰性分隔注释。
- 依赖提到根 `[workspace.dependencies]`；错误用 `thiserror`，日志用 `tracing`。
- 提交信息用 Conventional Commits，类型与范围英文小写、说明中文，例如 `feat(text): 文字装饰支持下划线`。

交流用中文。

# render-engine

自绘 UI 引擎：场景树（taffy 布局，每个节点画什么）→ 预乘 alpha 的 RGBA 位图，各平台只负责把位图贴到窗口上。

- 栅格：tiny-skia（纯 CPU）
- 文字：cosmic-text（fontdb + rustybuzz + swash），带回退链、彩色 emoji、SF 的光学字号与 `trak` 字距
- 字体：不扫系统字体目录，按平台清单只加载界面字体、中日文与 emoji 几个文件
- 效果：圆角、边框、渐变、九宫格图片 / SVG、投影与内阴影、整棵子树的不透明度
- 动画：跨帧按配对名做位置过渡，关键帧循环动画（只重画动的那一块）

第一个使用方是[青简输入法](https://github.com/qingjian-team/qingjian)的候选窗与状态条（`crates/qingjian-render`）。

## 许可

GPL-3.0-only，见 [LICENSE](LICENSE)。

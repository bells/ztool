## Why

Zero Snap 的状态栏图标目前会在左键点击后立即开始截图，既没有给用户确认当前动作的轻量入口，也无法自然承载后续录屏、录音能力。截图编辑器虽然绘制了选区控制点和尺寸提示，但控制点不可交互，用户只能重画选区，无法像成熟截图工具一样精确缩放或用方向键微调位置。

## What Changes

- 将 macOS 状态栏中的 Zero Snap 左键动作改为打开一个锚定在 Snap 图标下方的紧凑工具面板，而不是直接启动截图。
- 面板首个且当前唯一可执行项为“截图”；点击后关闭面板并进入现有 Zero Snap 截图流程。面板条目使用可扩展的类型化描述结构，为以后增加录屏、录音预留组合位置，但本次不展示不可用占位项，也不实现媒体录制。
- 为 Snap 面板定义重复点击切换、失焦、Escape、与其他 Zero 临时工具窗口互斥、屏幕边缘钳制以及图标锚点不可用时的安全回退行为。
- 让截图选区的四个角控制点支持双轴缩放，四条边支持单轴缩放，并在拖动期间实时更新选区、尺寸提示、遮罩和工具栏位置。
- 当 Select 工具生效且用户没有编辑文字时，让方向键按原始截图像素逐像素移动完整选区；移动必须限制在截图边界内，并支持系统键盘重复。
- 保留拖拽空白区域重建选区、标注选择、裁剪提交、全局截图快捷键和非 macOS 启动路径的现有语义。
- 为状态栏激活路由、弹层窗口几何、选区八向缩放和键盘移动增加纯模型与自动化回归测试，并保留真实 macOS 状态栏、窗口焦点和 Retina/多显示器人工验收边界。

## Capabilities

### New Capabilities

- `zero-snap-status-menu`: 定义 Zero Snap 状态栏图标所打开的紧凑、可扩展、图标锚定工具面板及其截图入口和临时窗口生命周期。
- `screenshot-selection-adjustment`: 定义截图选区的八向指针缩放、方向键逐像素移动、边界约束和实时反馈行为。

### Modified Capabilities

<!-- No existing main capability requirements are changed. -->

## Impact

- **Rust/Tauri:** 调整 `src-tauri/src/services/status_bar.rs` 中 Snap 的原生 macOS 激活策略；新增或扩展宿主拥有的 Snap 临时窗口创建、精确状态栏单元格锚定、显示器工作区钳制及 `tool_windows` 互斥协调。
- **React/TypeScript:** 为截图插件增加 Snap 菜单 surface、类型化动作描述、本地化和紧凑样式；扩展 `CaptureApp` 与 `captureSelectionModel.ts`，区分新建、移动和八向缩放手势，并接入方向键移动。
- **Routing/configuration:** 增加稳定的 Snap 菜单窗口标签，并同步 `appSurface`、插件 surface 注册、`src/main.tsx` 路由和 Tauri capability allowlist。
- **Contracts:** 不新增 Rust↔TypeScript 截图 IPC 字段；状态栏单元格锚点继续由宿主内部传递，截图动作复用现有 `start_screenshot`/会话契约。
- **Platform:** 新状态栏锚定面板以 macOS 原生分组状态栏为目标；全局快捷键仍直接截图，Windows 的系统截图路径和 Linux 的现有不支持/错误路径保持不变。
- **Verification:** 需要前端纯模型、源码边界、Rust 路由/几何测试及完整构建检查，并在真实 macOS `SystemUIServer`、Retina/多显示器和 Tauri 截图覆盖窗口中人工验证。

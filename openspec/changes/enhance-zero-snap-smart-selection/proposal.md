## Why

Zero Snap 目前打开捕获窗口后会直接把整张截图设为选区，用户必须重新框选，且缺少窗口边界吸附、鼠标定位参考和可输入的精确几何控制。参考录屏中的 iShot 交互，可以把“开始截图到得到准确选区”缩短为悬停识别、单击确认、必要时再精调的一条连续路径。

## What Changes

- macOS 自定义截图在捕获窗口显示后先进入未提交的目标选择态，不再默认把整张图片作为已完成选区；指针悬停可预览窗口候选，单击候选即可提交与应用窗口边界对齐的选区。
- 保留从空白位置拖动创建任意矩形选区；一旦拖动超过阈值，手动框选优先于当前窗口候选，避免吸附和自由选择争抢同一指针手势。
- 在目标选择和手动框选期间绘制一条水平线与一条垂直线组成的全屏十字参考线，并让交点跟随鼠标；提交选区或离开可捕获图像后隐藏参考线。
- 把选区左上方的只读尺寸徽标升级为紧凑几何控制条：宽和高分别可输入源图片像素值，并通过键盘、失焦或 Escape 明确提交或回滚。
- 在几何控制条中增加圆角半径滑杆。圆角预览、外部遮罩以及 Copy、Save、Pin 导出共享同一个源像素半径；圆角外的 PNG 像素透明，确保预览与结果一致。
- 过滤 Zero 自己的捕获/菜单/钉图窗口、不可见或最小化窗口、桌面层和无效小窗；候选不可用或原生枚举失败时仍允许全屏与手动框选，不阻塞截图。
- 新增窗口候选坐标、命中优先级、手势仲裁、尺寸输入、圆角约束和透明导出的自动化测试，并保留真实 macOS 屏幕录制权限、窗口层级、Retina 和多显示器人工验收边界。
- Windows 继续使用系统截图启动器，Linux/其他平台继续保持当前错误路径；本次不声称为这些平台增加 Zero 自定义覆盖层。
- macOS 捕获 WebView 保持隐藏，直到 session 初始化、PNG 原始字节读取、图片解码和 React DOM 提交全部完成后再原子显示；首个可见帧必须已经包含冻结截图，不能暴露 WebView 的空白加载页。
- macOS 覆盖层使用完整显示器边界和高于系统菜单栏/Dock 的原生窗口层级，使用户只看到截图中冻结的系统栏；Windows 继续完全交给系统截图启动器，因此不创建可能闪白或重复系统栏的 Zero 覆盖层。
- 捕获窗口先应用目标显示器的完整物理尺寸、再应用该显示器的全局位置，避免 macOS 从 WebView 默认尺寸放大时按左下角锚定而把窗口顶边推到负坐标；垂直堆叠、负原点和 Retina 显示器都必须保持覆盖窗与冻结 PNG 对齐。

## Capabilities

### New Capabilities

- `smart-screenshot-targeting`: 定义截图启动后的窗口边界候选、悬停预览、单击吸附、自由拖拽优先级和鼠标十字参考线行为。
- `screenshot-selection-geometry-controls`: 定义选区宽高输入、圆角半径调整、实时几何反馈以及透明圆角导出语义。

### Modified Capabilities

<!-- No existing main capability requirements are changed. -->

## Impact

- **Rust/Tauri:** `src-tauri/src/services/screenshot.rs` 在 macOS 会收集并过滤顶层窗口边界，将候选转换为当前截图图片的源像素坐标；`CaptureSessionPayload` 增加类型化候选列表。优先评估 Apache-2.0 的 `xcap` 窗口枚举能力，并以 macOS target dependency/窄适配层隔离。捕获窗口新增 session-scoped reveal 生命周期，AppKit 窗口设置只在主线程执行，失败时关闭隐藏窗口、清理 session 并恢复宿主 shell；窗口几何准备固定为 size-before-position，避免 AppKit 调整尺寸改变已经放好的顶边。
- **React/TypeScript:** `captureTypes.ts` 增加与 Rust 对称的候选契约；`CaptureApp` 的 Select 交互扩展为 targeting/create/resize 三类状态；新增纯命中、几何输入和圆角模型，并让现有 Canvas 导出接受选区圆角。捕获页面只在图片解码和 DOM 提交后调用 reveal，初始化或 reveal 失败会主动取消会话。
- **Rendering/UI:** 捕获覆盖层增加候选框、十字参考线和可编辑几何控制条；所有定位继续使用源图片像素与视口映射，控件、遮罩和参考线不进入导出图片。
- **Compatibility:** 现有截图 session id、受限媒体 token、上传 lease、Copy/Save/Pin 命令及标注对象保持兼容；会新增 `CaptureSessionPayload` 响应字段，但不新增可从普通窗口调用的原生命令。
- **Verification:** 需要 TypeScript 单元/集成测试、Rust 候选过滤与 IPC 序列化测试、完整前后端 gate、OpenSpec 严格校验，以及真实 macOS 应用窗口、重叠层级、屏幕边缘、Retina/多显示器和圆角 PNG 验收。

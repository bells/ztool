## Context

当前 macOS Zero Snap 在隐藏宿主窗口后调用系统 `screencapture` 生成 PNG，把受限媒体描述写入 `ScreenshotSessionStore`，随后在主显示器上打开全屏 `capture` WebView。`CaptureApp` 读取 PNG 后立刻用 `createFullImageSelection` 建立整图选区；React 已拥有源图片像素与视口坐标互转、选区新建/八向缩放/方向键移动、标注和 Copy/Save/Pin Canvas 导出。Rust 与 TypeScript 之间的 `CaptureSessionPayload` 目前只传 session、初始动作和媒体描述。

运行时复查发现，捕获 WebView 虽以 `visible(false)` 创建，但 Rust 在页面完成 session 初始化、4-5 MB PNG IPC 读取和图片解码之前立即 `show`，因此会先暴露 WebKit 空白帧。另一个问题是 Tauri `always_on_top` 在 macOS 仍低于 `SystemUIServer`：冻结截图已包含菜单栏，而真实菜单栏继续浮在覆盖层之上，形成两行。参考 Snapzy 的“先准备背景、再以 screen-saver level 展示全显示器 panel”以及 ScreenCapture 在 Windows 上“先准备捕获画布、再显示 topmost popup”的生命周期，本设计把跨平台不变量明确为 `Preparing -> Ready -> Visible -> Closing`。

用户录制的 25.5 秒 iShot 交互显示了四个连续阶段：启动后直接进入取景；移动鼠标时窗口或应用区域边界成为候选；贯穿捕获画面的水平/垂直参考线跟随指针；提交选区后左上方出现宽高和圆角滑杆。Zero 已有后半段选区编辑能力，因此本次应扩展截图插件，而不是引入第二个编辑器或绕过现有会话安全边界。

窗口枚举属于通用原生能力。2026-08-23 调研的 `xcap` 0.9.8 使用 Apache-2.0 许可证，GitHub 约 1,000 stars、crates.io 累计约 150 万下载，提供按 z 顺序的 `Window::all()` 以及 id、pid、应用名、标题、位置、像素尺寸、最小化和焦点状态。它比已被同一作者演进替代的 `screenshots` 更活跃；本设计只复用其窗口元数据能力，不迁移现有媒体存储、提交或平台启动路径。

## Goals / Non-Goals

**Goals:**

- 让 macOS 自定义覆盖层从“整图已选中”改为“悬停识别、单击确认或拖动框选”的目标选择态。
- 让窗口候选、选区、尺寸输入和圆角始终使用原始截图像素坐标，并通过明确的 Rust/TypeScript 对称契约跨 IPC。
- 让窗口枚举失败可降级，不能阻断全屏或手动截图。
- 让宽高输入、圆角预览、遮罩、工具栏锚点和最终 PNG 使用同一份选区几何。
- 保留现有截图 session、媒体 token、上传 lease、标注、Copy/Save/Pin 和宿主窗口恢复语义。
- 用纯模型覆盖可自动化的命中、过滤、坐标、手势和导出逻辑，同时标明真实 macOS 运行时门槛。
- 保证自定义覆盖层的第一个可见帧已经包含完成解码并提交的截图，并覆盖当前显示器的实时系统栏，避免空白闪现和双菜单栏。

**Non-Goals:**

- 不实现录屏中出现但用户未要求的阴影、颜色拾取、比例快捷键、OCR、滚动截图、多窗口合并或放大镜。
- 不识别应用窗口内部的按钮、侧栏、列表行等可访问性元素；首版智能候选只承诺顶层可见窗口和整屏回退。
- 不替换现有标注工具，不把选区几何调整写入标注 undo/redo 历史。
- 不把 `xcap` 用作新的视频录制引擎，也不借本次改写截图资源安全、剪贴板或文件保存架构。
- 不为 Windows/Linux 新建 Zero 自定义截图覆盖层；Windows 系统截图启动器与 Linux/其他错误路径保持不变。
- 不在本次重构 Zero 当前只覆盖一个捕获显示器的窗口模型；只要求其它显示器候选被安全过滤且坐标不泄漏到当前图片。

## Decisions

### 1. 在 Rust 会话准备阶段快照窗口候选，并只向前端暴露源像素矩形

在 `src-tauri/src/services/screenshot/` 下抽出 macOS 专用 `capture_targets` 适配器。截图开始隐藏 Zero 自有临时 surface 后，适配器调用 `xcap::Window::all()`，按原生 z 顺序读取窗口元数据，随后立即执行现有 PNG 捕获。候选与 PNG 一起写入同一个 `ScreenshotSession`，`init_screenshot_session` 只返回这份快照，不在覆盖层显示后重新枚举。

新增对称契约：

```rust
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotTargetCandidate {
    pub id: String,
    pub kind: ScreenshotTargetKind,
    pub bounds: ScreenshotSourceBounds,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ScreenshotTargetKind {
    Window,
}
```

```ts
interface ScreenshotTargetCandidate {
  id: string;
  kind: "window";
  bounds: Bounds;
}

interface CaptureSession {
  sessionId: string;
  initialAction: "copy" | "save";
  media: ScreenshotMediaDescriptor;
  targets: ScreenshotTargetCandidate[];
}
```

数组顺序就是从前到后的命中优先级；前端不接收应用名、窗口标题、pid 或全局屏幕坐标，降低无关信息暴露并避免 UI 依赖平台字段。id 是 session 内不透明标识，只用于稳定 React key/hover 比较，不能传回原生命令。

备选方案是前端在每次 pointermove 调用 Rust 命令枚举窗口，但这会增加 IPC、权限和时序抖动，也会让已冻结 PNG 与实时移动窗口不一致。另一个方案是直接手写 `CGWindowListCopyWindowInfo` FFI；它依赖更深的 CoreGraphics 字典解析和生命周期处理，而成熟的 `xcap` 已提供受维护的安全封装。

### 2. 用窄的 macOS target dependency 和可替换适配器接入 `xcap`

`xcap = "0.9.8"` 只加入 `cfg(target_os = "macos")` 依赖，本次只调用窗口枚举 API，不接入其录屏接口。业务服务只依赖内部 `CaptureTargetProvider`/纯 `NativeWindowSnapshot` 数据，不让 `xcap::Window` 穿过服务、IPC 或测试边界。适配器过滤：Zero 当前进程的 capture/snap-menu/pin 等窗口、最小化或读取失败的窗口、低于最小命中尺寸的窗口、不与当前捕获显示器相交的窗口，以及可识别的桌面背景层；部分相交的窗口裁剪到捕获图片范围。

原生窗口矩形先放入显式 `NativeCaptureGeometry`，再通过 `image_width / monitor_physical_width` 与 `image_height / monitor_physical_height` 转为源图片像素。转换采用外侧取整并在图片边界钳制，避免 Retina 或坐标小数让窗口边缘少一列像素。候选的全局负坐标、非主显示器坐标不会直接进入 React。

枚举、单个字段读取或坐标转换失败都记录为有界诊断并跳过对应候选；整体枚举失败则返回空数组，现有截图继续。选择 `xcap` 不是把截图可用性绑定到它。

### 3. 将“目标选择阶段”和“指针手势”分开建模

`CaptureApp` 不再初始化整图选区，而以 `selection = null`、`targeting = { hoverTargetId: null, pointer: null }` 启动。新增纯函数 `resolveScreenshotTargetAtPoint(targets, point, imageSize)`，按数组顺序返回第一个包含指针的有效窗口；没有窗口命中时返回内建整图 fallback，而不是把整图伪装成原生窗口候选。

pointerdown 保存源像素起点、当时候选和 pointer id，但不立即提交。pointerup 前的视口移动未超过 4 CSS px 时视为 click：若起止仍命中同一候选则提交其矩形；超过阈值后切换为现有 `create` 手势并从 pointerdown 起点建立自由选区，当前吸附候选立即失效。阈值以 CSS px 判断触控/Retina 手感，实际选区继续以源像素计算。

现有 `SelectionPointerInteraction` 保持判别联合并扩展为 `pending-target | create | resize`。同一 pointer id 同时只能属于一种手势；pointer cancel、lost capture 或 Escape 回到手势前状态。选区一旦提交，现有调整、标注和导出流程接管，窗口候选不再在背景中跳动。用户在 Select 工具中拖动空白画面仍可重建选区。

备选方案是 pointermove 时直接把 `selection` 改成候选矩形，但这会把预览误当成可导出的已提交状态，并让工具栏、快捷键和标注提前激活。

### 4. 十字参考线属于取景反馈，不属于截图内容

新增 `CaptureCrosshair` 纯视图状态，只保存当前指针在源图片/视口中的映射点。当处于 targeting 或活动 create 手势且指针位于实际渲染图片内时，渲染一条贯穿图片可见边界的水平线和一条垂直线；交点严格跟随指针。指针离开图片、选区提交、进入尺寸输入或开始 resize/annotation 后隐藏。

十字线与候选边框位于 PNG 图片和标注 Canvas 之上、几何控制与工具栏之下，并设置 `pointer-events: none`。它们只是 DOM/SVG chrome，不进入 `renderCaptureToCanvas`。CSS 使用 1 物理像素感知的细线和 Zero 现有选区强调色，不复制 iShot 的网格放大镜。

### 5. 用 `SelectionGeometry` 统一矩形与圆角半径

前端引入：

```ts
interface SelectionGeometry {
  bounds: Bounds;
  cornerRadius: number;
}
```

`cornerRadius` 是整数源图片像素，范围为 `0..floor(min(width, height) / 2)`。现有 resize、方向键移动和自由创建仍以 `Bounds` 纯函数工作，再通过 `normalizeSelectionGeometry` 钳制半径；缩小选区不会留下非法圆角。圆角默认 0，因此旧行为与旧 PNG 保持矩形。

本次不把圆角写进 Rust session 或持久化设置，因为它只影响本次前端编辑和最终上传的 PNG。Copy/Save/Pin 继续上传已渲染的同一份 PNG，不需要扩展命令输入。

### 6. 把尺寸徽标升级为受控、可回滚的几何控制条

`SelectionGeometryControls` 首选左边与选区对齐、显示在选区上方；空间不足时落到选区内侧，并始终按捕获视口钳制。它包含两个带明确 accessible label 的整数输入和一个圆角 slider。宽/高输入使用独立字符串 draft，避免用户输入过程中因空字符串或半个数字破坏选区。

Enter/Tab/失焦提交当前字段，Escape 恢复编辑前值。提交时把值解析为整数并限制到最小选区尺寸以及从当前左上角到图片右/下边界的最大值；左上角固定，改变右边或下边。无效文本不改变几何并显示紧凑错误状态。输入或 slider 获得焦点时，CaptureApp 的方向键 nudge、Delete、工具快捷键和 canvas pointerdown 必须让位给控件原生行为。

slider input 事件实时更新半径，范围随当前宽高重算；键盘方向键可以逐级调整 slider。控制条和工具栏都从同一个 active geometry 定位，拖动 resize 时显示实时尺寸，但输入只在指针手势结束后可交互。

备选方案是把尺寸编辑放入底部工具栏，但录屏的关键价值是尺寸与被调整选区在空间上直接关联；独立控制条也能避免把标注工具栏塞得更宽。

### 7. 用 SVG 偶奇遮罩和 Canvas alpha mask 保证所见即所得

当前 `.capture-selection-frame` 通过巨大 box-shadow 制造矩形遮罩，无法正确挖出圆角孔。改为独立 selection chrome：SVG 偶奇 path 绘制视口外框减去 rounded rect 的遮罩，另一个 rounded rect 绘制边界；八个现有 resize handle 仍锚定在 bounds 上。候选预览同样复用只读矩形 chrome，但不显示 handle/控制条。

导出时先按现有流程把底图和标注绘制到选区尺寸 Canvas，再在 `cornerRadius > 0` 时使用 rounded rect 与 `destination-in` 合成裁掉四角。这样底图和越过圆角边缘的标注一起被裁剪，圆角外保持真正 alpha=0；Copy、Save、Pin 都消费这份结果。半径 0 跳过额外合成，保持当前快路径。

控制条、十字线、候选框、遮罩、handle 和工具栏从不绘入导出 Canvas。录屏中的“阴影”开关不在范围内，圆角 PNG 周围也不自动添加阴影或画布留白。

### 8. 保持平台和故障边界诚实

只有 `cfg(target_os = "macos")` 的 custom-overlay session 返回窗口候选。非 macOS 不创建 `CaptureSessionPayload`，因此不新增伪候选或未使用依赖。macOS 没有屏幕录制权限时仍走现有截图错误与宿主恢复；只有候选枚举失败时才无提示降级为空候选，让整图 fallback 和手动拖拽可用。

自动测试证明纯坐标、序列化、手势和像素 alpha，不代表 Screen Recording 权限、真实 z-order、窗口阴影边界、Retina 缩放或多显示器运行时已正确。后者保留为明确任务。

### 9. 把捕获覆盖层改为 session-scoped 原子 reveal

`open_capture_window` 只创建隐藏、无边框、完整显示器尺寸的 WebView，安装销毁清理并完成位置/尺寸准备，不再负责 `show` 或聚焦。`CaptureApp` 完成 `init_screenshot_session`、受限 PNG 读取、Blob URL 建立和图片解码后一次性提交 session/image 状态；随后在 `useLayoutEffect` 中调用新的 `reveal_screenshot_capture({ sessionId })`，确保调用发生在图片元素已经写入 DOM 之后。ref 防止 React StrictMode 或重复渲染再次 reveal。

reveal 命令只接受 label 为 `capture` 的调用者，并校验请求 session id 等于活动 session；store 对同一 session 的 reveal claim 是幂等的。命令在 blocking worker 中等待一个有界主线程任务，避免 IPC/main-thread 重入死锁。macOS 主线程任务设置 `NSWindow.level = NSScreenSaverWindowLevel`、`sharingType = None`、`animationBehavior = None`，以及 `CanJoinAllSpaces | FullScreenAuxiliary | Stationary` collection behavior，然后才 show/focus。这里不使用 `simple_fullscreen`，因为它会改变应用级菜单栏/Dock presentation state，增加取消、异常和多屏恢复风险。

初始化、调度、原生配置或 reveal 失败时，前端尽力调用已有 cancel；Rust 端也关闭捕获窗口、清理活动 session/lease/media 并恢复原 shell。Windows 保持 `ms-screenclip:`/Snipping Tool 系统启动路径：它不会创建 `capture` WebView，因此同一产品保证通过“系统原生 launcher 独占显示”满足，而不是复制一套尚未运行时验证的自定义覆盖层。

### 10. 把目标显示器几何作为一个不可拆分的窗口帧应用

2026-08-23 的新录屏显示，冻结 PNG 内容正确，但覆盖窗只出现在屏幕上半部分。运行时窗口边界为 `(0, -600, 1440, 900)`；这恰好对应捕获 WebView 默认约 `800×600` 物理像素（Retina 下约 `400×300` 点）先定位到屏幕顶边，随后扩大到 `2880×1800` 物理像素时，AppKit 保持左下角不动，使顶边向上移动 600 点。问题不是 React 遮罩或截图裁剪，而是 `set_position` 后 `set_size` 把一个完整帧拆成了顺序敏感的两次变更。

`open_capture_window` 因此必须先 `set_size(monitor_size)`，再 `set_position(monitor_position)`。最后一次操作明确恢复目标显示器的全局顶左原点，适用于 `(0, 0)`、负 X/Y 和垂直堆叠显示器；这里继续使用显示器完整 bounds，而非排除菜单栏/Dock 的 work area。捕获窗保持隐藏直至这两个操作都成功，任一失败沿用现有关闭窗口、清理 session 和恢复 shell 的路径。

Flameshot 的实现提供了可复用的不变量而不是可直接复制的 Qt 代码：macOS 先用光标所在 `QScreen` 捕获该屏幕，窗口也移动并缩放到同一个 `QScreen::geometry()`；Windows 对选中显示器额外把 pixmap 物理尺寸按 DPR 还原为窗口逻辑尺寸，并把窗口绑定到选中 `QScreen`。Zero 本次先修复已证实的帧应用顺序；后续若从“主显示器单屏捕获”扩展为“点击/光标显示器捕获”，必须让显示器选择、PNG 捕获、候选转换和覆盖窗共同消费同一份显式显示器描述，不能分别重新查询 primary/current screen。

## Risks / Trade-offs

- [Risk] 窗口在候选枚举与 PNG 捕获之间移动，矩形与冻结图像短暂错位 → 隐藏 Zero surface 后连续同步执行枚举和捕获，不在覆盖层出现后刷新；真实拖窗竞态作为烟测记录。
- [Risk] `xcap` 的坐标/尺寸在 Retina 或负坐标布局上与 Tauri/screencapture 不同 → 经 `NativeCaptureGeometry` 显式缩放、外侧取整和裁剪，并覆盖 1x/2x、负原点与跨屏窗口纯测试。
- [Risk] 引入 `xcap` 增加 macOS 构建体积或传递依赖 → 仅放入 macOS target dependency，检查 `Cargo.lock`、重复 `image` 版本、release bundle 增量和许可证；业务层通过窄适配器保持可替换。
- [Risk] 某些系统/无标题窗口被错误展示或过滤 → 过滤规则保守、候选失败不阻塞，使用 z 顺序与整图 fallback，并在 Finder、浏览器、Tauri、全屏窗口上实测。
- [Risk] click 与 drag 阈值造成误吸附或延迟 → 只延迟到 pointerup，使用 4 CSS px 纯模型和真实 Retina 手感验证，pointer capture 保证快速拖动完整。
- [Risk] 圆角预览与导出像素不一致 → SVG 和 Canvas 共用同一源像素 radius/normalizer，并用四角 alpha 采样测试 Copy/Save/Pin 的共同 PNG 路径。
- [Risk] 小选区放不下控制条或工具栏 → 两者分别使用纯定位模型在上/下/内侧回退并钳制，不允许覆盖导致输入不可达。
- [Risk] 输入控件抢占全局截图快捷键 → 基于 editable target、focus、composition 和 pointer state 的现有 hotkey guard 扩展测试，控件事件不冒泡到 Canvas。
- [Risk] 旧 `CaptureSession` 测试/fixture 因新增必填字段失败 → Rust/TS 同步增加 `targets`，测试 fixture 显式使用空数组；不使用前端可选字段掩盖契约漂移。
- [Risk] hidden WebView 的渲染提交与原生窗口 show 存在竞态 → 图片先解码，React layout effect 在 DOM mutation 后发出 ready，Rust 再把 reveal 排入 AppKit 主线程；窗口在此之前始终不可见。
- [Risk] 高窗口层级或失败路径让系统栏/宿主窗口无法恢复 → 只修改单个 capture `NSWindow`，不改变 `NSApplication` presentation options；所有 reveal 错误都走 session 清理和 shell 恢复。
- [Risk] macOS 在窗口已定位后改变尺寸会保留左下角并移动顶边，Retina 下位移会被放大为明显的半屏裁切 → 隐藏准备期固定使用 size-before-position，并用源契约测试锁定调用顺序；真实垂直堆叠/负坐标仍做运行时烟测。

## Migration Plan

1. 先添加 Rust/TypeScript 对称契约、候选过滤/坐标转换与序列化失败测试；完成 `xcap` macOS target dependency 的许可证、锁文件和构建验证。
2. 让 session 在不改变截图输出的前提下携带候选，前端先读取但不展示；确认候选为空时完整现有流程仍可运行。
3. 引入 targeting/pending-target/create 状态与十字线，取消整图自动提交，再接入单击吸附和拖动仲裁。
4. 引入 `SelectionGeometry`、几何控制条、圆角 SVG 遮罩和 Canvas alpha mask；逐项验证现有 resize、nudge、标注、Copy/Save/Pin。
5. 将 capture window 拆成 hidden preparation 与 session-scoped reveal，补齐 macOS 原生层级、前端解码后 ready 以及失败清理测试。
6. 固定捕获窗口 size-before-position 的完整帧应用顺序，运行聚焦与完整 gate、严格 OpenSpec 校验，再执行真实 macOS 权限、首帧、窗口层级、Retina/多显示器和 PNG alpha 验收。

回滚时移除候选字段/适配器与 `xcap` macOS 依赖，恢复 `createFullImageSelection` 初始化，保留既有 selection resize/nudge 和矩形导出。没有持久化数据迁移，旧截图 session 在进程重启后自然失效。

## Open Questions

无阻塞问题。首版窗口候选只承诺顶层窗口；是否进一步识别应用内部可访问性区域、增加阴影或将自定义覆盖层扩展到 Windows，应分别通过后续 OpenSpec 评估权限、性能和跨平台坐标模型。

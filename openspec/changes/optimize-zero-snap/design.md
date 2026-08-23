## Context

Zero 在 macOS 上把主 Ø 与各工具图标组合为一个原生状态栏 item，并已能通过同一套单元格命中计算区分 Launch、Snap、Awake、Paper 和主图标。当前 `native_status_bar_activation` 仅为 Launch 与 Paper 提供专用 surface，Snap 仍落回 `StatusBarActionType::StartScreenshot`，所以左键命中 Snap 后会立刻调用截图服务。Paper 已提供一套可复用的先例：宿主创建无边框临时窗口、使用被点击单元格的物理矩形定位、按目标显示器工作区钳制，并与其他临时工具窗口互斥。

截图覆盖窗口中的真实选区保存在原始图片像素坐标中。`CaptureApp` 已能用 Select 拖动重建选区，并将选区映射到视口，绘制尺寸徽标、遮罩和八个控制点；但是 `.capture-selection-frame` 为 `pointer-events: none`，控制点目前只是装饰。当前 Select 指针状态只区分“重建选区”和“绘制标注”，方向键也没有选区移动语义。

本次同时触及原生状态栏路由、Tauri 窗口生命周期、插件 surface 注册、React 交互状态和纯几何模型。设计必须保留现有截图会话、裁剪提交和平台边界，并且不能把未来录屏、录音的具体权限、编码或存储假设提前固化。

## Goals / Non-Goals

**Goals:**

- 让 macOS Snap 图标打开一个精确锚定、可切换、可键盘操作的紧凑面板。
- 让当前唯一的“截图”入口安全地交接到现有 macOS 自定义截图流程，并保持全局快捷键直接截图。
- 让面板的动作描述可新增录屏、录音条目，而不复制窗口生命周期或状态栏路由。
- 让四个圆形角点和四条边成为真实的选区缩放入口，并保持固定对边、最小尺寸和图片边界约束。
- 让方向键按原始截图像素移动完整选区，保持选区大小并支持按键重复。
- 用纯 TypeScript/Rust 模型覆盖几何和路由边界，同时诚实保留真实 macOS 状态栏、焦点、Retina 与多显示器验收。

**Non-Goals:**

- 本次不实现、展示或承诺录屏、录音、OCR、翻译、延时截图等条目。
- 不把主 Zero 面板或完整 `ScreenshotPanel` 塞进状态栏小面板，也不重做 Zero Snap 的主窗口详情页。
- 不改变 `CommandOrControl+Shift+A` 的直接截图语义，不修改 Copy/Save/Pin 的图片上传契约。
- 不增加选区数值输入、比例锁定、窗口智能识别、Shift 加速移动、键盘缩放或拖动选区内部移动。
- 不改变标注对象的移动/缩放行为，也不把选区调整写入标注撤销重做历史。
- 不以 macOS 面板实现替换 Windows 系统截图启动器或扩大 Linux 支持范围。

## Decisions

### 1. 将 Snap 纳入宿主拥有的 surface-aware 状态栏激活策略

`NativeStatusBarActivation` 增加 Snap 目标。macOS 分组状态栏在命中 Snap 单元格后，不再执行清单中的 `start-screenshot` action，而是把该单元格物理矩形传给 `toggle_snap_menu_window`。全局快捷键继续直接调用 `start_screenshot_session(..., "copy")`；非 macOS fallback action row 继续执行现有清单 action，不依赖 macOS 几何。

该选择把原生命中、窗口位置和平台策略留在 Rust 宿主中，也不会把 `show-snap-menu` 之类的窗口细节扩散到插件 manifest 公共 action union。备选方案是把 Snap 清单 action 改成 `open-plugin`，但那会打开完整 Zero shell，无法满足参考图的轻量入口。

### 2. 使用一个宿主窗口和一个插件自有 React surface

新增稳定窗口标签 `snap-menu`，并把它同步到 `AppSurface`、`BundledPluginSurface`、`src/main.tsx` 的插件 surface 路由、截图插件 `surfaces` 注册和 Tauri capability allowlist。Rust 以隐藏、无边框、透明、不可调整大小、always-on-top、skip-taskbar 的配置惰性创建单例窗口；窗口尺寸使用紧凑 token，足以容纳当前单行入口，并允许以后通过同一列表纵向增加条目。

`SnapMenuApp` 只渲染插件自有的类型化 `SnapMenuItem` 描述。首版数组仅包含稳定 id `screenshot`、本地化 label、现有 Snap 图标语义和启动 handler；不渲染禁用的录屏/录音占位。第一个可用项在面板出现时获得键盘焦点，Enter/Space 与鼠标点击等价，Escape 调用现有 `hide_current_surface`。

这保持插件的展示、i18n 与动作描述独立可插拔，同时由宿主统一掌握原生窗口。备选方案是构建原生 `Menu`；原生菜单不便复用插件本地化/视觉体系，也难以为未来媒体动作展示一致的状态与错误反馈。

### 3. 抽取共享的锚定几何并扩展临时工具窗口协调器

Snap 使用与 Paper 相同的“被点击虚拟单元格”作为锚点：优先在该单元格下方居中，随后按锚点中心解析目标显示器，并在其物理 work area 内同时钳制水平和垂直位置。现有 Paper 专用 anchor/position 计算应提取为宿主内部的通用物理几何 helper，由 Paper 和 Snap 分别提供窗口尺寸与间距；当单元格或显示器几何不可用时，回退到安全的 `TrayCenter`，不能因定位失败静默声称已显示。

`ToolWindowKind` 增加 Snap，`prepare_tool_window` 在打开 Snap 时隐藏 tray、Launch 和 Paper；打开 Launch 或 Paper 时也隐藏 Snap。重复点击已经可见的 Snap 图标只隐藏现有窗口。右键菜单、工具折叠、可见性设置和 Ø 图标行为不进入这个协调器。

抽取共享 helper 比复制 Paper 定位函数更能避免负坐标、混合 DPI 和屏幕边缘规则逐渐漂移；但原生 `SystemUIServer` 几何仍必须实机验证。

### 4. 用一个受限 Rust command 原子完成“隐藏菜单后截图”交接

直接让 React 先隐藏窗口再调用现有 `start_screenshot` 会产生竞态：窗口可能被截进图片，且启动失败后错误面板已经消失。新增窄命令 `start_snap_menu_screenshot`，自动注入调用窗口并要求 label 为 `snap-menu`；命令先隐藏 Snap surface，再委托同一个 `start_screenshot_session(app, "copy")` 服务。启动准备失败时，它重新显示并聚焦原 Snap 窗口，让 React 能显示本地化错误；成功后面板保持隐藏，截图取消或完成也不重新打开它。

前端调用保持显式类型：`invoke<ScreenshotStartResult>("start_snap_menu_screenshot")`。返回结构复用现有 `ScreenshotStartResult`，不新增图片数据或会话字段。全局快捷键与主 Snap 面板继续走各自现有入口。

### 5. 用判别联合统一选区新建与八向缩放手势

在 `captureSelectionModel.ts` 增加 `SelectionResizeHandle` 八值 union 和纯函数 `resolveSelectionResize(initial, handle, delta, imageSize, minimumSize)`。角点同时移动两条相邻边；上/下只修改 y/height，左/右只修改 x/width；相对边保持固定。活动边到达固定边的最小距离后停止，不穿越或隐式翻转 handle；所有结果钳制到原始截图边界。

`CaptureApp` 用一个 `SelectionPointerInteraction` 判别联合替代彼此松散的多个选区 ref，至少区分 `create` 与 `resize`，并保存开始点、原选区、handle 与 pointer id。控制点接收 pointer down 并优先于 canvas；pointer capture 保证快速拖到选区或窗口之外时仍收到结束事件。move 只写入 `selectionDraft`，pointer up 提交，pointer cancel、lost capture 或 Escape 回滚到开始前选区并清理交互。

选区 frame 本身保持不拦截普通 canvas 操作，仅八个放大的透明 hit target 可交互。四个角点显示为圆形，四边中点显示为紧凑边控件，并设置对应 `nwse`、`nesw`、`ns`、`ew` cursor。视觉大小与至少 16px 的命中区域分离，避免为了易点选而制造过重装饰。

### 6. 方向键移动使用图片像素模型而不是 CSS 位移

新增纯函数 `moveSelectionBy(selection, delta, imageSize)`：保持 width/height 不变，并把 x/y 钳制到 `[0, imageSize - selectionSize]`。`CaptureApp` 的 keydown 仅在 Select 工具激活、会话与有效选区存在、没有文字输入/IME 编辑、没有指针选区手势且未按 Command/Control/Option/Alt 时消费 Arrow 键。每次 keydown 使用 `(±1, 0)` 或 `(0, ±1)` 原始图片像素；不屏蔽 `event.repeat`，因此系统重复自然生效。

移动直接提交到 `selection`，不进入标注 history，也不改变 width/height。到达边界后的继续按键保持状态不变。使用图片坐标而非 CSS 像素，才能保证 Retina/缩放显示下的尺寸徽标与最终裁剪完全一致。

### 7. 所有调整中的视觉与导出继续共享唯一选区来源

`activeSelection = selectionDraft ?? selection` 继续驱动选区边框、外部遮罩、尺寸徽标和工具栏锚点，因此缩放拖动期间四者实时同步。Copy/Save/Pin 的现有裁剪链路仍只消费最终有效选区；控制点、尺寸提示和遮罩绝不进入导出 canvas。

调整控制点不创建 annotation、不改变当前标注，也不触发 undo/redo。点击已有标注仍优先选择标注；点击没有命中控制点或标注的空白区域仍按现有规则重建选区。这样选区边界与标注编辑保持清晰分层。

## Risks / Trade-offs

- [Risk] Snap 面板在捕获前未完全隐藏而被截入图片 → 由 Rust command 同步隐藏后再启动截图，并用真实截图像素验收首帧。
- [Risk] 菜单失焦回调与第二次状态栏点击竞争，导致隐藏后又重开 → 沿用临时窗口的短延迟失焦复核与可见性 toggle，并覆盖重复点击测试。
- [Risk] 从 Paper 抽取共享定位 helper 引入回归 → 先迁移现有 Paper 几何测试，再增加 Snap 尺寸、负坐标、左右边缘、纵向多屏和混合 DPI 用例。
- [Risk] 小型控制点在 Retina 屏幕难以命中 → 把视觉点与透明命中区域分离，并在真实 Tauri 窗口检查八个方向。
- [Risk] 选区 resize 与 canvas 重建选区同时开始 → handle pointer down 阻止冒泡并由判别联合保证同一时刻只有一个选区手势。
- [Risk] 方向键与文字输入、输入法或未来标注快捷键冲突 → 只在 Select 工具和无编辑上下文时消费无修饰 Arrow，保留输入控件默认行为。
- [Risk] 连续键盘移动造成不必要的 React 工作 → 每次只更新一个小型 Bounds 对象，依赖现有 memoized 映射；实测持续按键期间的响应与工具栏跟随。
- [Risk] 自动测试误代表状态栏/多显示器运行时正确 → 将 `SystemUIServer` 精确命中、窗口焦点、负坐标、Retina、边缘与截图不含菜单保留为明确人工门槛。

## Migration Plan

1. 先为共享锚定几何、Snap 激活目标、Snap 临时窗口选项、八向 resize 和逐像素 move 添加失败测试。
2. 抽取 Paper/Snap 共用的宿主定位 helper，扩展 `ToolWindowKind`，再接入 `snap-menu` 窗口与插件 surface 路由。
3. 增加 Snap 菜单 UI 和原子截图交接命令，验证全局快捷键与 fallback action 不变。
4. 将选区指针状态改为判别联合，接入八个 handle、pointer cancel 和实时 draft；随后接入方向键移动。
5. 运行聚焦测试、完整前后端 gate 和 OpenSpec 严格校验，再执行真实 macOS 单屏/多屏截图验收。

回滚时恢复 Snap 的 `ExistingAction` 路由、移除 `snap-menu` label/surface/窗口与新增命令，并移除控制点事件及新几何函数。没有持久化数据或用户设置迁移，现有截图会话与图片格式无需回滚。

## Open Questions

无阻塞问题。Snap 面板的最终宽高、圆角和行间距可在真实菜单栏烟测时按 Zero 的紧凑工具语言微调，但不得改变单一“截图”入口、锚定、键盘与生命周期契约。

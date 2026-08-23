import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useReducer,
  useRef,
  useState,
  type CSSProperties,
  type PointerEvent as ReactPointerEvent,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Check,
  Circle,
  Download,
  Grid3X3,
  MousePointer2,
  MoveUpRight,
  Pencil,
  Pin,
  Redo2,
  Square,
  Trash2,
  Type,
  Undo2,
  X,
  type LucideIcon,
} from "lucide-react";
import type { Bounds } from "./captureCanvas";
import {
  drawAnnotations,
  isAnnotationLargeEnough,
  normalizeRect,
} from "./captureCanvas";
import {
  cropCanvasToPngBytes,
  loadImageFromObjectUrl,
  releaseCanvas,
  releaseDecodedImage,
  releaseObjectUrl,
  renderFinalCanvas,
} from "./captureExport";
import { resolveCaptureHotkey } from "./captureHotkeys";
import { captureReducer, createId, initialHistoryState } from "./captureReducer";
import {
  CAPTURE_SELECTION_HANDLES,
  imageBoundsToViewportBounds,
  isPointInBounds,
  moveSelectionBy,
  resolveSelectPointerTarget,
  resolveSelectionDrag,
  resolveSelectionNudge,
  resolveSelectionResize,
  viewportPointToImagePoint,
  type SelectionResizeHandle,
  type Size,
} from "./captureSelectionModel";
import {
  MINIMUM_SELECTION_DIMENSION,
  normalizeSelectionGeometry,
  selectionGeometryFromBounds,
  type SelectionGeometry,
} from "./captureGeometryModel";
import {
  hasExceededTargetDragThreshold,
  normalizeScreenshotTargets,
  resolveScreenshotTargetAtPoint,
  resolveStableTargetClick,
  type ResolvedScreenshotTarget,
} from "./captureTargetModel";
import { SelectionGeometryControls } from "./SelectionGeometryControls";
import {
  buildPrepareScreenshotCommitPayload,
  buildScreenshotUploadOptions,
} from "./captureSerialize";
import {
  resolveCaptureToolbarPosition,
  type CaptureToolbarPosition,
} from "./captureToolbarModel";
import type {
  AnnotationObject,
  CaptureSession,
  CaptureTool,
  Point,
  ScreenshotCommitResult,
  ScreenshotError,
  ScreenshotUploadLease,
} from "./captureTypes";
import { resolveLanguage } from "../../../core/preferences/i18n";
import { normalizePreferences } from "../../../core/preferences/preferencesModel";
import { readStoredPreferences } from "../../../core/preferences/preferencesStorage";
import {
  createScreenshotTranslator,
  type TranslationKey,
} from "../i18n";

type DraftAnnotation = AnnotationObject | null;

interface TextDraft {
  x: number;
  y: number;
  screenX: number;
  screenY: number;
  value: string;
}

interface CaptureToolDescriptor {
  tool: CaptureTool;
  labelKey: TranslationKey;
  Icon: LucideIcon;
}

type SelectionPointerInteraction =
  | {
      kind: "pending-target";
      pointerId: number;
      start: Point;
      startViewport: Point;
      target: ResolvedScreenshotTarget | null;
    }
  | {
      kind: "create";
      pointerId: number;
      start: Point;
      initial: SelectionGeometry;
    }
  | {
      kind: "resize";
      pointerId: number;
      start: Point;
      initial: SelectionGeometry;
      handle: SelectionResizeHandle;
    };

const tools: CaptureToolDescriptor[] = [
  { tool: "select", labelKey: "screenshot.toolbar.select", Icon: MousePointer2 },
  { tool: "rectangle", labelKey: "screenshot.toolbar.rectangle", Icon: Square },
  { tool: "ellipse", labelKey: "screenshot.toolbar.ellipse", Icon: Circle },
  { tool: "arrow", labelKey: "screenshot.toolbar.arrow", Icon: MoveUpRight },
  { tool: "pen", labelKey: "screenshot.toolbar.pen", Icon: Pencil },
  { tool: "text", labelKey: "screenshot.toolbar.text", Icon: Type },
  { tool: "mosaic", labelKey: "screenshot.toolbar.mosaic", Icon: Grid3X3 },
  { tool: "pin", labelKey: "screenshot.toolbar.pin", Icon: Pin },
];

const TEXT_INPUT_WIDTH = 220;
const TEXT_INPUT_HEIGHT = 96;
const TEXT_FONT_SIZE = 24;

function roundedRectanglePath(bounds: Bounds, radius: number): string {
  const { x, y, width, height } = bounds;
  const safeRadius = Math.max(0, Math.min(radius, width / 2, height / 2));
  const right = x + width;
  const bottom = y + height;
  return [
    `M ${x + safeRadius} ${y}`,
    `H ${right - safeRadius}`,
    `Q ${right} ${y} ${right} ${y + safeRadius}`,
    `V ${bottom - safeRadius}`,
    `Q ${right} ${bottom} ${right - safeRadius} ${bottom}`,
    `H ${x + safeRadius}`,
    `Q ${x} ${bottom} ${x} ${bottom - safeRadius}`,
    `V ${y + safeRadius}`,
    `Q ${x} ${y} ${x + safeRadius} ${y}`,
    "Z",
  ].join(" ");
}

function screenshotErrorMessage(error: unknown): string {
  if (typeof error === "object" && error !== null && "message" in error) {
    const message = (error as ScreenshotError).message;
    if (typeof message === "string") {
      return message;
    }
  }
  return String(error);
}

export function CaptureApp() {
  const preferences = normalizePreferences(readStoredPreferences(window.localStorage), []);
  const t = createScreenshotTranslator(
    resolveLanguage(preferences.language, navigator.language),
  );
  const [session, setSession] = useState<CaptureSession | null>(null);
  const [imageSrc, setImageSrc] = useState<string | null>(null);
  const [baseImage, setBaseImage] = useState<HTMLImageElement | null>(null);
  const [tool, setTool] = useState<CaptureTool>("select");
  const [selection, setSelection] = useState<SelectionGeometry | null>(null);
  const [selectionDraft, setSelectionDraft] = useState<SelectionGeometry | null>(null);
  const [hoverTarget, setHoverTarget] = useState<ResolvedScreenshotTarget | null>(null);
  const [crosshair, setCrosshair] = useState<Point | null>(null);
  const [viewportSize, setViewportSize] = useState<Size>(() => ({
    width: window.innerWidth,
    height: window.innerHeight,
  }));
  const [toolbarSize, setToolbarSize] = useState<Size | null>(null);
  const [history, dispatch] = useReducer(captureReducer, initialHistoryState);
  const [draft, setDraft] = useState<DraftAnnotation>(null);
  const [textDraft, setTextDraft] = useState<TextDraft | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isCommitting, setIsCommitting] = useState(false);
  const dragStartRef = useRef<Point | null>(null);
  const selectionInteractionRef = useRef<SelectionPointerInteraction | null>(null);
  const draftIdRef = useRef<string | null>(null);
  const draftRef = useRef<DraftAnnotation>(null);
  const textDraftRef = useRef<TextDraft | null>(null);
  const imageRef = useRef<HTMLImageElement | null>(null);
  const overlayRef = useRef<HTMLCanvasElement | null>(null);
  const textInputRef = useRef<HTMLTextAreaElement | null>(null);
  const toolbarRef = useRef<HTMLElement | null>(null);
  const revealedSessionIdRef = useRef<string | null>(null);

  useEffect(() => {
    let disposed = false;
    let objectUrl: string | null = null;
    let decodedImage: HTMLImageElement | null = null;
    let receivedBytes: Uint8Array | null = null;
    let initializedSessionId: string | null = null;

    void (async () => {
      try {
        const payload = await invoke<CaptureSession>("init_screenshot_session", {});
        initializedSessionId = payload.sessionId;
        const value = await invoke<ArrayBuffer>("read_screenshot_media", {
          input: { token: payload.media.token },
        });
        receivedBytes = new Uint8Array(value);
        if (
          receivedBytes.byteLength === 0 ||
          receivedBytes.byteLength !== payload.media.byteLength
        ) {
          throw new Error("The screenshot resource size does not match its descriptor");
        }
        objectUrl = URL.createObjectURL(new Blob([value], { type: payload.media.mimeType }));
        decodedImage = await loadImageFromObjectUrl(objectUrl);
        if (disposed) {
          releaseDecodedImage(decodedImage);
          decodedImage = null;
          objectUrl = releaseObjectUrl(objectUrl);
          return;
        }
        setSession(payload);
        setSelection(null);
        setImageSrc(objectUrl);
        setBaseImage(decodedImage);
      } catch (error) {
        releaseDecodedImage(decodedImage);
        decodedImage = null;
        objectUrl = releaseObjectUrl(objectUrl);
        if (!disposed) {
          setError(screenshotErrorMessage(error));
          if (initializedSessionId) {
            await invoke("cancel_screenshot_session", {
              sessionId: initializedSessionId,
            }).catch(() => undefined);
          }
        }
      } finally {
        receivedBytes?.fill(0);
        receivedBytes = null;
      }
    })();

    return () => {
      disposed = true;
      releaseDecodedImage(decodedImage);
      objectUrl = releaseObjectUrl(objectUrl);
    };
  }, []);

  useLayoutEffect(() => {
    if (!session || !imageSrc || !baseImage) {
      return;
    }
    if (revealedSessionIdRef.current === session.sessionId) {
      return;
    }
    const imageElement = imageRef.current;
    if (!imageElement) {
      return;
    }
    let cancelled = false;

    void (async () => {
      try {
        await imageElement.decode();
        if (
          cancelled ||
          revealedSessionIdRef.current === session.sessionId
        ) {
          return;
        }
        revealedSessionIdRef.current = session.sessionId;

        // Force layout after the displayed image itself has decoded. Native reveal is
        // queued onto AppKit's main thread, so no empty WebView frame is exposed.
        void document.documentElement.offsetWidth;
        await invoke("reveal_screenshot_capture", { sessionId: session.sessionId });
      } catch (error) {
        if (cancelled) {
          return;
        }
        setError(screenshotErrorMessage(error));
        await invoke("cancel_screenshot_session", {
          sessionId: session.sessionId,
        }).catch(() => undefined);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [baseImage, imageSrc, session]);

  useLayoutEffect(() => {
    const input = textInputRef.current;
    if (!input) {
      return;
    }

    const focusFrame = requestAnimationFrame(() => {
      input.focus({ preventScroll: true });
      input.setSelectionRange(input.value.length, input.value.length);
    });
    return () => cancelAnimationFrame(focusFrame);
  }, [textDraft?.screenX, textDraft?.screenY]);

  useLayoutEffect(() => {
    const toolbar = toolbarRef.current;
    if (!toolbar) {
      return;
    }

    const updateToolbarSize = () => {
      const rect = toolbar.getBoundingClientRect();
      const next = { width: rect.width, height: rect.height };
      setToolbarSize((current) =>
        current?.width === next.width && current.height === next.height ? current : next,
      );
    };
    const updateViewportSize = () => {
      setViewportSize({ width: window.innerWidth, height: window.innerHeight });
    };

    const observer = new ResizeObserver(updateToolbarSize);
    observer.observe(toolbar);
    window.addEventListener("resize", updateViewportSize);
    updateToolbarSize();
    updateViewportSize();

    return () => {
      observer.disconnect();
      window.removeEventListener("resize", updateViewportSize);
    };
  }, [selection]);

  const imageSize = useMemo<Size>(
    () => ({ width: session?.media.width ?? 0, height: session?.media.height ?? 0 }),
    [session?.media.height, session?.media.width],
  );
  const targets = useMemo(
    () => normalizeScreenshotTargets(session?.targets ?? [], imageSize),
    [imageSize, session?.targets],
  );
  const activeGeometry = selectionDraft ?? selection;
  const activeSelection = activeGeometry?.bounds ?? null;
  const selectionViewportBounds = useMemo(() => {
    if (!session || !activeSelection) {
      return null;
    }
    return imageBoundsToViewportBounds(
      activeSelection,
      { width: session.media.width, height: session.media.height },
      viewportSize,
    );
  }, [activeSelection, session, viewportSize]);

  const imageViewportBounds = useMemo(() => {
    if (!session) {
      return null;
    }
    return imageBoundsToViewportBounds(
      { x: 0, y: 0, width: session.media.width, height: session.media.height },
      imageSize,
      viewportSize,
    );
  }, [imageSize, session, viewportSize]);

  const hoverViewportBounds = useMemo(() => {
    if (!hoverTarget || !session) {
      return null;
    }
    return imageBoundsToViewportBounds(hoverTarget.bounds, imageSize, viewportSize);
  }, [hoverTarget, imageSize, session, viewportSize]);

  const toolbarPosition: CaptureToolbarPosition | null = useMemo(() => {
    if (!selectionViewportBounds || !toolbarSize) {
      return null;
    }
    return resolveCaptureToolbarPosition(selectionViewportBounds, toolbarSize, viewportSize);
  }, [selectionViewportBounds, toolbarSize, viewportSize]);

  useEffect(() => {
    if (!session || !overlayRef.current) {
      return;
    }

    const overlay = overlayRef.current;
    overlay.width = session.media.width;
    overlay.height = session.media.height;
    const ctx = overlay.getContext("2d");
    if (!ctx) {
      return;
    }

    ctx.clearRect(0, 0, session.media.width, session.media.height);
    if (baseImage) {
      ctx.drawImage(baseImage, 0, 0, session.media.width, session.media.height);
    }
    drawAnnotations(ctx, draft ? [...history.annotations, draft] : history.annotations);
  }, [baseImage, draft, history.annotations, session]);

  const pointerToImagePoint = useCallback(
    (event: ReactPointerEvent, clampToImage = false): Point | null => {
      if (!session || !imageRef.current) {
        return null;
      }

      const rect = imageRef.current.getBoundingClientRect();
      return viewportPointToImagePoint(
        { x: event.clientX - rect.left, y: event.clientY - rect.top },
        { width: session.media.width, height: session.media.height },
        { width: rect.width, height: rect.height },
        clampToImage,
      );
    },
    [session],
  );

  const updateDraft = useCallback((nextDraft: DraftAnnotation) => {
    draftRef.current = nextDraft;
    setDraft(nextDraft);
  }, []);

  const updateTextDraft = useCallback((nextDraft: TextDraft | null) => {
    textDraftRef.current = nextDraft;
    setTextDraft(nextDraft);
  }, []);

  const renderCurrentFinalCanvas = useCallback((): HTMLCanvasElement => {
    if (!session || !baseImage) {
      throw new Error("Capture session is not ready");
    }

    return renderFinalCanvas(
      baseImage,
      session.media.width,
      session.media.height,
      history.annotations,
    );
  }, [baseImage, history.annotations, session]);

  const uploadSelection = useCallback(
    async (
      action: "copy" | "save" | "pin",
      geometry: SelectionGeometry,
    ): Promise<ScreenshotCommitResult> => {
      if (!session) {
        throw new Error("Capture session is not ready");
      }
      const lease = await invoke<ScreenshotUploadLease>(
        "prepare_screenshot_commit",
        buildPrepareScreenshotCommitPayload(session.sessionId, action),
      );
      let sourceCanvas: HTMLCanvasElement | null = null;
      let pngBytes: Uint8Array | null = null;
      try {
        sourceCanvas = renderCurrentFinalCanvas();
        pngBytes = await cropCanvasToPngBytes(
          sourceCanvas,
          geometry.bounds,
          geometry.cornerRadius,
        );
        if (!pngBytes) {
          throw new Error("Screenshot selection is outside the captured image");
        }
        if (pngBytes.byteLength === 0 || pngBytes.byteLength > lease.maxBytes) {
          throw new Error("Screenshot PNG exceeds the approved upload size");
        }
        return await invoke<ScreenshotCommitResult>(
          "upload_screenshot_commit",
          pngBytes,
          buildScreenshotUploadOptions(lease),
        );
      } finally {
        pngBytes?.fill(0);
        releaseCanvas(sourceCanvas);
      }
    },
    [renderCurrentFinalCanvas, session],
  );

  const commitPin = useCallback(
    async (geometry: SelectionGeometry) => {
      setError(null);
      try {
        await uploadSelection("pin", geometry);
      } catch (error) {
        setError(screenshotErrorMessage(error));
      }
    },
    [uploadSelection],
  );

  const commit = useCallback(
    async (action: "copy" | "save") => {
      if (!session || !selection) {
        return;
      }

      setIsCommitting(true);
      setError(null);
      try {
        await uploadSelection(action, selection);
      } catch (error) {
        setError(screenshotErrorMessage(error));
      } finally {
        setIsCommitting(false);
      }
    },
    [selection, session, uploadSelection],
  );

  const cancel = useCallback(() => {
    if (draft || textDraft || selectionDraft || selectionInteractionRef.current) {
      setDraft(null);
      draftRef.current = null;
      updateTextDraft(null);
      setSelectionDraft(null);
      dragStartRef.current = null;
      selectionInteractionRef.current = null;
      draftIdRef.current = null;
      setCrosshair(null);
      setHoverTarget(null);
      return;
    }

    if (!session) {
      return;
    }

    invoke("cancel_screenshot_session", { sessionId: session.sessionId }).catch((error) =>
      setError(screenshotErrorMessage(error)),
    );
  }, [draft, selectionDraft, session, textDraft, updateTextDraft]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const target = event.target;
      const editableTarget =
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        (target instanceof HTMLElement && target.isContentEditable);
      const selectionDelta = resolveSelectionNudge({
        key: event.key,
        selectToolActive: tool === "select",
        editableTarget,
        composing: event.isComposing || Boolean(textDraftRef.current),
        pointerActive: Boolean(selectionInteractionRef.current),
        metaKey: event.metaKey,
        ctrlKey: event.ctrlKey,
        altKey: event.altKey,
        shiftKey: event.shiftKey,
        repeat: event.repeat,
      });
      if (selectionDelta && session) {
        event.preventDefault();
        setSelection((current) =>
          current
            ? normalizeSelectionGeometry(
                {
                  ...current,
                  bounds: moveSelectionBy(current.bounds, selectionDelta, imageSize),
                },
                imageSize,
              )
            : current,
        );
        return;
      }

      if (editableTarget) {
        return;
      }

      const action = resolveCaptureHotkey(event);
      if (!action) {
        return;
      }

      event.preventDefault();
      if (action === "undo") {
        dispatch({ type: "undo" });
      } else if (action === "redo") {
        dispatch({ type: "redo" });
      } else if (action === "removeSelected") {
        dispatch({ type: "removeSelected" });
      } else if (action === "cancel") {
        cancel();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [cancel, imageSize, session, tool]);

  function createDraft(start: Point, point: Point): AnnotationObject {
    const id = draftIdRef.current ?? createId(tool);
    draftIdRef.current = id;

    if (tool === "arrow") {
      return {
        id,
        type: "arrow",
        from: start,
        to: point,
        color: "#55f280",
        strokeWidth: 5,
      };
    }

    if (tool === "pen") {
      const previousDraft = draftRef.current;
      const previousPoints = previousDraft?.type === "pen" ? previousDraft.points : [start];
      return {
        id,
        type: "pen",
        points: [...previousPoints, point],
        color: "#55f280",
        strokeWidth: 5,
      };
    }

    const bounds = normalizeRect(start, point);
    if (tool === "ellipse") {
      return {
        id,
        type: "ellipse",
        ...bounds,
        color: "#55f280",
        strokeWidth: 4,
      };
    }

    if (tool === "mosaic") {
      return {
        id,
        type: "mosaic",
        ...bounds,
        color: "#55f280",
        strokeWidth: 2,
        pixelSize: 14,
      };
    }

    if (tool === "pin") {
      return {
        id,
        type: "pin",
        ...bounds,
        color: "#55f280",
        strokeWidth: 2,
      };
    }

    return {
      id,
      type: "rectangle",
      ...bounds,
      color: "#55f280",
      strokeWidth: 4,
    };
  }

  const handlePointerDown = (event: ReactPointerEvent<HTMLCanvasElement>) => {
    const point = pointerToImagePoint(event);
    if (!point) {
      return;
    }

    if (textDraftRef.current) {
      event.preventDefault();
      commitTextDraft();
      return;
    }

    if (tool !== "select" && activeSelection && !isPointInBounds(point, activeSelection)) {
      return;
    }

    setError(null);

    if (tool === "select") {
      if (!selection) {
        const target = resolveScreenshotTargetAtPoint(targets, point, imageSize);
        event.currentTarget.setPointerCapture(event.pointerId);
        selectionInteractionRef.current = {
          kind: "pending-target",
          pointerId: event.pointerId,
          start: point,
          startViewport: { x: event.clientX, y: event.clientY },
          target,
        };
        setHoverTarget(target);
        setCrosshair(point);
        return;
      }
      const target = resolveSelectPointerTarget(history.annotations, point);
      if (target.kind === "annotation") {
        dispatch({ type: "select", id: target.annotationId });
        return;
      }
      event.currentTarget.setPointerCapture(event.pointerId);
      dispatch({ type: "select", id: null });
      selectionInteractionRef.current = {
        kind: "create",
        pointerId: event.pointerId,
        start: point,
        initial: selection,
      };
      setSelectionDraft(null);
      setCrosshair(point);
      return;
    }

    if (tool === "text") {
      updateTextDraft({
        x: point.x,
        y: point.y,
        screenX: Math.min(event.clientX, Math.max(0, window.innerWidth - TEXT_INPUT_WIDTH)),
        screenY: Math.min(event.clientY, Math.max(0, window.innerHeight - TEXT_INPUT_HEIGHT)),
        value: "",
      });
      return;
    }

    event.currentTarget.setPointerCapture(event.pointerId);
    dragStartRef.current = point;
    draftIdRef.current = createId(tool);
    updateDraft(createDraft(point, point));
  };

  const handlePointerMove = (event: ReactPointerEvent<HTMLCanvasElement>) => {
    const point = pointerToImagePoint(event);
    if (!point) {
      if (!selectionInteractionRef.current) {
        setHoverTarget(null);
        setCrosshair(null);
      }
      return;
    }

    const interaction = selectionInteractionRef.current;
    if (
      interaction?.kind === "pending-target" &&
      interaction.pointerId === event.pointerId &&
      session
    ) {
      setCrosshair(point);
      const currentTarget = resolveScreenshotTargetAtPoint(targets, point, imageSize);
      setHoverTarget(currentTarget);
      if (
        hasExceededTargetDragThreshold(interaction.startViewport, {
          x: event.clientX,
          y: event.clientY,
        })
      ) {
        const initial = selectionGeometryFromBounds({
          x: interaction.start.x,
          y: interaction.start.y,
          width: 0,
          height: 0,
        });
        selectionInteractionRef.current = {
          kind: "create",
          pointerId: event.pointerId,
          start: interaction.start,
          initial,
        };
        setHoverTarget(null);
        const nextBounds = resolveSelectionDrag(
          initial.bounds,
          interaction.start,
          point,
          imageSize,
        );
        setSelectionDraft(normalizeSelectionGeometry({ ...initial, bounds: nextBounds }, imageSize));
      }
      return;
    }
    if (
      interaction?.kind === "create" &&
      interaction.pointerId === event.pointerId &&
      session
    ) {
      const nextSelection = resolveSelectionDrag(
        interaction.initial.bounds,
        interaction.start,
        point,
        { width: session.media.width, height: session.media.height },
      );
      setSelectionDraft(
        nextSelection === interaction.initial.bounds
          ? null
          : normalizeSelectionGeometry(
              { ...interaction.initial, bounds: nextSelection },
              imageSize,
            ),
      );
      setCrosshair(point);
      return;
    }

    if (!selection && tool === "select") {
      setHoverTarget(resolveScreenshotTargetAtPoint(targets, point, imageSize));
      setCrosshair(point);
    }

    const start = dragStartRef.current;
    if (!start) {
      return;
    }

    updateDraft(createDraft(start, point));
  };

  const handlePointerUp = (event: ReactPointerEvent<HTMLCanvasElement>) => {
    const point = pointerToImagePoint(event);
    const interaction = selectionInteractionRef.current;
    if (
      interaction?.kind === "pending-target" &&
      interaction.pointerId === event.pointerId &&
      session
    ) {
      const endTarget = point
        ? resolveScreenshotTargetAtPoint(targets, point, imageSize)
        : null;
      const bounds = resolveStableTargetClick(interaction.target, endTarget, false);
      if (bounds) {
        setSelection(normalizeSelectionGeometry(selectionGeometryFromBounds(bounds), imageSize));
        setHoverTarget(null);
        setCrosshair(null);
      }
      selectionInteractionRef.current = null;
      if (event.currentTarget.hasPointerCapture(event.pointerId)) {
        event.currentTarget.releasePointerCapture(event.pointerId);
      }
      return;
    }
    if (
      interaction?.kind === "create" &&
      interaction.pointerId === event.pointerId &&
      session
    ) {
      const nextSelection = point
        ? resolveSelectionDrag(
            interaction.initial.bounds,
            interaction.start,
            point,
            { width: session.media.width, height: session.media.height },
          )
        : interaction.initial.bounds;
      const nextGeometry = normalizeSelectionGeometry(
        { ...interaction.initial, bounds: nextSelection },
        imageSize,
      );
      if (
        nextGeometry.bounds.width >= MINIMUM_SELECTION_DIMENSION &&
        nextGeometry.bounds.height >= MINIMUM_SELECTION_DIMENSION
      ) {
        setSelection(nextGeometry);
      }
      setSelectionDraft(null);
      selectionInteractionRef.current = null;
      setHoverTarget(null);
      setCrosshair(null);
      if (event.currentTarget.hasPointerCapture(event.pointerId)) {
        event.currentTarget.releasePointerCapture(event.pointerId);
      }
      return;
    }

    const start = dragStartRef.current;
    const currentDraft = start && point ? createDraft(start, point) : draftRef.current;
    dragStartRef.current = null;
    draftIdRef.current = null;
    draftRef.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }

    if (!currentDraft || !isAnnotationLargeEnough(currentDraft)) {
      setDraft(null);
      return;
    }

    updateDraft(null);
    if (currentDraft.type === "pin") {
      void commitPin(selectionGeometryFromBounds(currentDraft));
      return;
    }

    dispatch({ type: "add", annotation: currentDraft });
  };

  const rollbackSelectionPointerInteraction = (event: ReactPointerEvent) => {
    const interaction = selectionInteractionRef.current;
    if (!interaction || interaction.pointerId !== event.pointerId) {
      return;
    }

    selectionInteractionRef.current = null;
    setSelectionDraft(null);
    setCrosshair(null);
    setHoverTarget(null);
  };

  const handleResizePointerDown = (
    event: ReactPointerEvent<HTMLSpanElement>,
    handle: SelectionResizeHandle,
  ) => {
    if (tool !== "select" || !session || !selection) {
      return;
    }

    const point = pointerToImagePoint(event, true);
    if (!point) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    dispatch({ type: "select", id: null });
    selectionInteractionRef.current = {
      kind: "resize",
      pointerId: event.pointerId,
      start: point,
      initial: selection,
      handle,
    };
    setSelectionDraft(null);
  };

  const handleResizePointerMove = (event: ReactPointerEvent<HTMLSpanElement>) => {
    const interaction = selectionInteractionRef.current;
    if (
      interaction?.kind !== "resize" ||
      interaction.pointerId !== event.pointerId ||
      !session
    ) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    const point = pointerToImagePoint(event, true);
    if (!point) {
      return;
    }

    const bounds = resolveSelectionResize(
      interaction.initial.bounds,
      interaction.handle,
      {
        x: point.x - interaction.start.x,
        y: point.y - interaction.start.y,
      },
      imageSize,
    );
    setSelectionDraft(
      normalizeSelectionGeometry({ ...interaction.initial, bounds }, imageSize),
    );
  };

  const handleResizePointerUp = (event: ReactPointerEvent<HTMLSpanElement>) => {
    const interaction = selectionInteractionRef.current;
    if (
      interaction?.kind !== "resize" ||
      interaction.pointerId !== event.pointerId ||
      !session
    ) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    const point = pointerToImagePoint(event, true);
    const nextSelection = point
      ? resolveSelectionResize(
          interaction.initial.bounds,
          interaction.handle,
          {
            x: point.x - interaction.start.x,
            y: point.y - interaction.start.y,
          },
          imageSize,
        )
      : interaction.initial.bounds;
    setSelection(
      normalizeSelectionGeometry(
        { ...interaction.initial, bounds: nextSelection },
        imageSize,
      ),
    );
    setSelectionDraft(null);
    selectionInteractionRef.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  };

  const handleResizePointerCancel = (event: ReactPointerEvent<HTMLSpanElement>) => {
    event.preventDefault();
    event.stopPropagation();
    rollbackSelectionPointerInteraction(event);
  };

  function commitTextDraft() {
    const currentDraft = textDraftRef.current;
    if (!currentDraft) {
      return;
    }
    updateTextDraft(null);

    const text = currentDraft.value;
    if (text.trim().length === 0) {
      return;
    }

    dispatch({
      type: "add",
      annotation: {
        id: createId("text"),
        type: "text",
        x: currentDraft.x,
        y: currentDraft.y + TEXT_FONT_SIZE,
        text,
        fontSize: TEXT_FONT_SIZE,
        color: "#55f280",
        strokeWidth: 2,
      },
    });
  }

  const selectionViewportRadius =
    activeGeometry && activeSelection && selectionViewportBounds && activeSelection.width > 0
      ? activeGeometry.cornerRadius * (selectionViewportBounds.width / activeSelection.width)
      : 0;
  const selectionMaskPath =
    selectionViewportBounds && imageViewportBounds
      ? [
          `M ${imageViewportBounds.x} ${imageViewportBounds.y}`,
          `H ${imageViewportBounds.x + imageViewportBounds.width}`,
          `V ${imageViewportBounds.y + imageViewportBounds.height}`,
          `H ${imageViewportBounds.x}`,
          "Z",
          roundedRectanglePath(selectionViewportBounds, selectionViewportRadius),
        ].join(" ")
      : null;

  return (
    <main className="capture-shell">
      {imageSrc ? (
        <img ref={imageRef} className="capture-image" src={imageSrc} alt="" draggable={false} />
      ) : null}
      <canvas
        ref={overlayRef}
        className="capture-overlay"
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerCancel={rollbackSelectionPointerInteraction}
        onLostPointerCapture={rollbackSelectionPointerInteraction}
        onPointerLeave={() => {
          if (!selectionInteractionRef.current) {
            setHoverTarget(null);
            setCrosshair(null);
          }
        }}
      />
      {!selection && hoverViewportBounds ? (
        <div
          className="capture-target-preview"
          role="img"
          aria-label={t("screenshot.target.preview")}
          style={{
            left: hoverViewportBounds.x,
            top: hoverViewportBounds.y,
            width: hoverViewportBounds.width,
            height: hoverViewportBounds.height,
          }}
        />
      ) : null}
      {crosshair && imageViewportBounds ? (
        <div
          className="capture-crosshair"
          role="img"
          aria-label={t("screenshot.target.guides")}
          style={{
            "--capture-guide-x": `${imageViewportBounds.x + (crosshair.x / imageSize.width) * imageViewportBounds.width}px`,
            "--capture-guide-y": `${imageViewportBounds.y + (crosshair.y / imageSize.height) * imageViewportBounds.height}px`,
            "--capture-image-left": `${imageViewportBounds.x}px`,
            "--capture-image-top": `${imageViewportBounds.y}px`,
            "--capture-image-width": `${imageViewportBounds.width}px`,
            "--capture-image-height": `${imageViewportBounds.height}px`,
          } as CSSProperties}
        >
          <span className="capture-guide-horizontal" />
          <span className="capture-guide-vertical" />
        </div>
      ) : null}
      {!selection && imageSrc ? (
        <p className="capture-targeting-hint" role="status">
          {t("screenshot.target.hint")}
        </p>
      ) : null}
      {selectionMaskPath && selectionViewportBounds ? (
        <svg className="capture-selection-chrome" aria-hidden="true">
          <path className="capture-selection-dimming" d={selectionMaskPath} fillRule="evenodd" />
          <path
            className={selectionDraft ? "capture-selection-border dragging" : "capture-selection-border"}
            d={roundedRectanglePath(selectionViewportBounds, selectionViewportRadius)}
          />
        </svg>
      ) : null}
      {activeSelection && selectionViewportBounds ? (
        <div
          className={`capture-selection-frame${selectionDraft ? " dragging" : ""}${tool === "select" ? " adjustable" : ""}`}
          style={{
            left: selectionViewportBounds.x,
            top: selectionViewportBounds.y,
            width: selectionViewportBounds.width,
            height: selectionViewportBounds.height,
            borderRadius: selectionViewportRadius,
          }}
          aria-hidden="true"
        >
          {CAPTURE_SELECTION_HANDLES.map(({ handle, cursor }) => (
            <span
              className={`capture-selection-handle ${handle}`}
              data-handle={handle}
              key={handle}
              style={{ cursor }}
              onPointerDown={(event) => handleResizePointerDown(event, handle)}
              onPointerMove={handleResizePointerMove}
              onPointerUp={handleResizePointerUp}
              onPointerCancel={handleResizePointerCancel}
              onLostPointerCapture={handleResizePointerCancel}
            />
          ))}
        </div>
      ) : null}
      {selection && activeGeometry && selectionViewportBounds ? (
        <SelectionGeometryControls
          geometry={activeGeometry}
          imageSize={imageSize}
          viewportBounds={selectionViewportBounds}
          viewportSize={viewportSize}
          disabled={Boolean(selectionDraft)}
          labels={{
            group: t("screenshot.geometry.group"),
            width: t("screenshot.geometry.width"),
            height: t("screenshot.geometry.height"),
            radius: t("screenshot.geometry.radius"),
            invalid: t("screenshot.geometry.invalid"),
          }}
          onChange={(geometry) => setSelection(normalizeSelectionGeometry(geometry, imageSize))}
        />
      ) : null}
      {textDraft ? (
        <textarea
          ref={textInputRef}
          className="capture-text-input"
          style={{ left: textDraft.screenX, top: textDraft.screenY }}
          value={textDraft.value}
          aria-label={t("screenshot.toolbar.text")}
          onChange={(event) => updateTextDraft({ ...textDraft, value: event.target.value })}
          onBlur={commitTextDraft}
          onKeyDown={(event) => {
            if (event.key === "Escape") {
              event.preventDefault();
              updateTextDraft(null);
            }
          }}
        />
      ) : null}
      {error ? <p className="capture-error">{error}</p> : null}
      {selection ? <nav
        ref={toolbarRef}
        className="capture-toolbar-live"
        aria-label={t("screenshot.toolbar.label")}
        data-placement={toolbarPosition?.placement}
        data-ready={toolbarPosition ? "true" : "false"}
        style={{ left: toolbarPosition?.left ?? 0, top: toolbarPosition?.top ?? 0 }}
      >
        <span
          className="capture-tool-group"
          role="group"
          aria-label={t("screenshot.toolbar.tools")}
        >
          {tools.map(({ tool: entryTool, labelKey, Icon }) => {
            const label = t(labelKey);
            const selected = entryTool === tool;
            return (
              <button
                type="button"
                className={selected ? "capture-tool-live selected" : "capture-tool-live"}
                key={entryTool}
                title={label}
                aria-label={label}
                aria-pressed={selected}
                onClick={() => setTool(entryTool)}
              >
                <Icon aria-hidden="true" />
              </button>
            );
          })}
        </span>
        <span className="capture-divider-live" aria-hidden="true" />
        <span
          className="capture-tool-group"
          role="group"
          aria-label={t("screenshot.toolbar.history")}
        >
          <button
            type="button"
            className="capture-tool-live"
            title={t("screenshot.toolbar.undo")}
            aria-label={t("screenshot.toolbar.undo")}
            disabled={history.undoStack.length === 0}
            onClick={() => dispatch({ type: "undo" })}
          >
            <Undo2 aria-hidden="true" />
          </button>
          <button
            type="button"
            className="capture-tool-live"
            title={t("screenshot.toolbar.redo")}
            aria-label={t("screenshot.toolbar.redo")}
            disabled={history.redoStack.length === 0}
            onClick={() => dispatch({ type: "redo" })}
          >
            <Redo2 aria-hidden="true" />
          </button>
          <button
            type="button"
            className="capture-tool-live danger"
            title={t("screenshot.toolbar.delete")}
            aria-label={t("screenshot.toolbar.delete")}
            disabled={!history.selectedId}
            onClick={() => dispatch({ type: "removeSelected" })}
          >
            <Trash2 aria-hidden="true" />
          </button>
        </span>
        <span className="capture-divider-live" aria-hidden="true" />
        <span
          className="capture-tool-group"
          role="group"
          aria-label={t("screenshot.toolbar.finish")}
        >
          <button
            type="button"
            className="capture-tool-live danger"
            title={t("screenshot.toolbar.cancel")}
            aria-label={t("screenshot.toolbar.cancel")}
            onClick={cancel}
          >
            <X aria-hidden="true" />
          </button>
          <button
            type="button"
            className="capture-tool-live"
            title={t("screenshot.toolbar.save")}
            aria-label={t("screenshot.toolbar.save")}
            disabled={isCommitting || !selection}
            onClick={() => commit("save")}
          >
            <Download aria-hidden="true" />
          </button>
          <button
            type="button"
            className="capture-tool-live confirm"
            title={t("screenshot.toolbar.copy")}
            aria-label={t("screenshot.toolbar.copy")}
            disabled={isCommitting || !selection}
            onClick={() => commit("copy")}
          >
            <Check aria-hidden="true" />
          </button>
        </span>
      </nav> : null}
    </main>
  );
}

export default CaptureApp;

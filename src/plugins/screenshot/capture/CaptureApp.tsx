import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useReducer,
  useRef,
  useState,
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
  cropCanvasToPngDataUrl,
  loadImageFromBase64,
  renderFinalCanvas,
} from "./captureExport";
import { resolveCaptureHotkey } from "./captureHotkeys";
import { captureReducer, createId, initialHistoryState } from "./captureReducer";
import {
  createFullImageSelection,
  imageBoundsToViewportBounds,
  isPointInBounds,
  resolveSelectPointerTarget,
  resolveSelectionDrag,
  viewportPointToImagePoint,
  type Size,
} from "./captureSelectionModel";
import {
  buildCommitScreenshotPayload,
  buildPinScreenshotPayload,
} from "./captureSerialize";
import {
  resolveCaptureToolbarPosition,
  type CaptureToolbarPosition,
} from "./captureToolbarModel";
import type { AnnotationObject, CaptureSession, CaptureTool, Point } from "./captureTypes";
import { createTranslator, resolveLanguage, type TranslationKey } from "../../preferences/i18n";
import { normalizePreferences } from "../../preferences/preferencesModel";
import { readStoredPreferences } from "../../preferences/preferencesStorage";

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

function toImageSrc(imageBase64: string): string {
  return imageBase64.startsWith("data:")
    ? imageBase64
    : `data:image/png;base64,${imageBase64}`;
}

export function CaptureApp() {
  const preferences = normalizePreferences(readStoredPreferences(window.localStorage), []);
  const t = createTranslator(resolveLanguage(preferences.language, navigator.language));
  const [session, setSession] = useState<CaptureSession | null>(null);
  const [imageSrc, setImageSrc] = useState<string | null>(null);
  const [baseImage, setBaseImage] = useState<HTMLImageElement | null>(null);
  const [tool, setTool] = useState<CaptureTool>("select");
  const [selection, setSelection] = useState<Bounds | null>(null);
  const [selectionDraft, setSelectionDraft] = useState<Bounds | null>(null);
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
  const selectionDragStartRef = useRef<Point | null>(null);
  const selectionBeforeDragRef = useRef<Bounds | null>(null);
  const draftIdRef = useRef<string | null>(null);
  const draftRef = useRef<DraftAnnotation>(null);
  const textDraftRef = useRef<TextDraft | null>(null);
  const imageRef = useRef<HTMLImageElement | null>(null);
  const overlayRef = useRef<HTMLCanvasElement | null>(null);
  const textInputRef = useRef<HTMLTextAreaElement | null>(null);
  const toolbarRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    invoke<CaptureSession>("init_screenshot_session", {})
      .then(async (payload) => {
        const src = toImageSrc(payload.image_base64);
        setSession(payload);
        setSelection(createFullImageSelection({ width: payload.width, height: payload.height }));
        setImageSrc(src);
        setBaseImage(await loadImageFromBase64(src));
      })
      .catch((err) => setError(String(err)));
  }, []);

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
  }, []);

  const activeSelection = selectionDraft ?? selection;
  const selectionViewportBounds = useMemo(() => {
    if (!session || !activeSelection) {
      return null;
    }
    return imageBoundsToViewportBounds(
      activeSelection,
      { width: session.width, height: session.height },
      viewportSize,
    );
  }, [activeSelection, session, viewportSize]);

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
    overlay.width = session.width;
    overlay.height = session.height;
    const ctx = overlay.getContext("2d");
    if (!ctx) {
      return;
    }

    ctx.clearRect(0, 0, session.width, session.height);
    if (baseImage) {
      ctx.drawImage(baseImage, 0, 0, session.width, session.height);
    }
    drawAnnotations(ctx, draft ? [...history.annotations, draft] : history.annotations);
  }, [baseImage, draft, history.annotations, session]);

  const pointerToImagePoint = useCallback(
    (event: ReactPointerEvent): Point | null => {
      if (!session || !imageRef.current) {
        return null;
      }

      const rect = imageRef.current.getBoundingClientRect();
      return viewportPointToImagePoint(
        { x: event.clientX - rect.left, y: event.clientY - rect.top },
        { width: session.width, height: session.height },
        { width: rect.width, height: rect.height },
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

    return renderFinalCanvas(baseImage, session.width, session.height, history.annotations);
  }, [baseImage, history.annotations, session]);

  const commitPin = useCallback(
    async (bounds: Bounds) => {
      if (!session) {
        return;
      }

      setError(null);
      try {
        const pngBase64 = cropCanvasToPngDataUrl(renderCurrentFinalCanvas(), bounds);
        if (!pngBase64) {
          return;
        }

        await invoke("pin_screenshot", buildPinScreenshotPayload(session.session_id, pngBase64));
      } catch (err) {
        setError(String(err));
      }
    },
    [renderCurrentFinalCanvas, session],
  );

  const commit = useCallback(
    async (action: "copy" | "save") => {
      if (!session || !selection) {
        return;
      }

      setIsCommitting(true);
      setError(null);
      try {
        const pngBase64 = cropCanvasToPngDataUrl(renderCurrentFinalCanvas(), selection);
        if (!pngBase64) {
          throw new Error("Screenshot selection is outside the captured image");
        }
        await invoke(
          "commit_screenshot",
          buildCommitScreenshotPayload({
            sessionId: session.session_id,
            action,
            pngBase64,
          }),
        );
      } catch (err) {
        setError(String(err));
      } finally {
        setIsCommitting(false);
      }
    },
    [renderCurrentFinalCanvas, selection, session],
  );

  const cancel = useCallback(() => {
    if (draft || textDraft || selectionDraft) {
      setDraft(null);
      draftRef.current = null;
      updateTextDraft(null);
      setSelectionDraft(null);
      dragStartRef.current = null;
      selectionDragStartRef.current = null;
      selectionBeforeDragRef.current = null;
      draftIdRef.current = null;
      return;
    }

    if (!session) {
      return;
    }

    invoke("cancel_screenshot_session", { sessionId: session.session_id }).catch((err) =>
      setError(String(err)),
    );
  }, [draft, selectionDraft, session, textDraft, updateTextDraft]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const target = event.target;
      if (
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        (target instanceof HTMLElement && target.isContentEditable)
      ) {
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
  }, [cancel]);

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
      const target = resolveSelectPointerTarget(history.annotations, point);
      if (target.kind === "annotation") {
        dispatch({ type: "select", id: target.annotationId });
        return;
      }
      if (!selection) {
        return;
      }
      event.currentTarget.setPointerCapture(event.pointerId);
      dispatch({ type: "select", id: null });
      selectionDragStartRef.current = point;
      selectionBeforeDragRef.current = selection;
      setSelectionDraft(null);
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
      return;
    }

    const selectionStart = selectionDragStartRef.current;
    const previousSelection = selectionBeforeDragRef.current;
    if (selectionStart && previousSelection && session) {
      const nextSelection = resolveSelectionDrag(
        previousSelection,
        selectionStart,
        point,
        { width: session.width, height: session.height },
      );
      setSelectionDraft(nextSelection === previousSelection ? null : nextSelection);
      return;
    }

    const start = dragStartRef.current;
    if (!start) {
      return;
    }

    updateDraft(createDraft(start, point));
  };

  const handlePointerUp = (event: ReactPointerEvent<HTMLCanvasElement>) => {
    const point = pointerToImagePoint(event);
    const selectionStart = selectionDragStartRef.current;
    const previousSelection = selectionBeforeDragRef.current;
    if (selectionStart && previousSelection && session) {
      const nextSelection = point
        ? resolveSelectionDrag(
            previousSelection,
            selectionStart,
            point,
            { width: session.width, height: session.height },
          )
        : previousSelection;
      setSelection(nextSelection);
      setSelectionDraft(null);
      selectionDragStartRef.current = null;
      selectionBeforeDragRef.current = null;
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
      void commitPin(currentDraft);
      return;
    }

    dispatch({ type: "add", annotation: currentDraft });
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
      />
      {activeSelection && selectionViewportBounds ? (
        <div
          className={selectionDraft ? "capture-selection-frame dragging" : "capture-selection-frame"}
          style={{
            left: selectionViewportBounds.x,
            top: selectionViewportBounds.y,
            width: selectionViewportBounds.width,
            height: selectionViewportBounds.height,
          }}
          aria-hidden="true"
        >
          <span className="capture-selection-size">
            {Math.round(activeSelection.width)} × {Math.round(activeSelection.height)}
          </span>
          {[
            "top-left",
            "top",
            "top-right",
            "right",
            "bottom-right",
            "bottom",
            "bottom-left",
            "left",
          ].map((position) => (
            <span className={`capture-selection-handle ${position}`} key={position} />
          ))}
        </div>
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
      <nav
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
      </nav>
    </main>
  );
}

export default CaptureApp;

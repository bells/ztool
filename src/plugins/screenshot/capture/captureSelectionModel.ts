import {
  clampBounds,
  hitTestAnnotation,
  normalizeRect,
  type Bounds,
} from "./captureCanvas.js";
import type { AnnotationObject, Point } from "./captureTypes.js";

export interface Size {
  width: number;
  height: number;
}

export type SelectionResizeHandle =
  | "top-left"
  | "top"
  | "top-right"
  | "right"
  | "bottom-right"
  | "bottom"
  | "bottom-left"
  | "left";

export interface CaptureSelectionHandleDescriptor {
  handle: SelectionResizeHandle;
  cursor: "nwse-resize" | "nesw-resize" | "ns-resize" | "ew-resize";
  horizontalEdge: "left" | "right" | null;
  verticalEdge: "top" | "bottom" | null;
}

export const CAPTURE_SELECTION_HANDLES = [
  { handle: "top-left", cursor: "nwse-resize", horizontalEdge: "left", verticalEdge: "top" },
  { handle: "top", cursor: "ns-resize", horizontalEdge: null, verticalEdge: "top" },
  { handle: "top-right", cursor: "nesw-resize", horizontalEdge: "right", verticalEdge: "top" },
  { handle: "right", cursor: "ew-resize", horizontalEdge: "right", verticalEdge: null },
  { handle: "bottom-right", cursor: "nwse-resize", horizontalEdge: "right", verticalEdge: "bottom" },
  { handle: "bottom", cursor: "ns-resize", horizontalEdge: null, verticalEdge: "bottom" },
  { handle: "bottom-left", cursor: "nesw-resize", horizontalEdge: "left", verticalEdge: "bottom" },
  { handle: "left", cursor: "ew-resize", horizontalEdge: "left", verticalEdge: null },
] as const satisfies readonly CaptureSelectionHandleDescriptor[];

export interface SelectionNudgeInput {
  key: string;
  selectToolActive: boolean;
  editableTarget: boolean;
  composing: boolean;
  pointerActive: boolean;
  metaKey?: boolean;
  ctrlKey?: boolean;
  altKey?: boolean;
  shiftKey?: boolean;
  repeat?: boolean;
}

export type SelectPointerTarget =
  | { kind: "annotation"; annotationId: string }
  | { kind: "selection" };

interface ContainGeometry {
  scale: number;
  offsetX: number;
  offsetY: number;
  renderedWidth: number;
  renderedHeight: number;
}

const MINIMUM_SELECTION_SIZE = 4;

function containGeometry(source: Size, viewport: Size): ContainGeometry | null {
  if (
    source.width <= 0 ||
    source.height <= 0 ||
    viewport.width <= 0 ||
    viewport.height <= 0
  ) {
    return null;
  }

  const scale = Math.min(viewport.width / source.width, viewport.height / source.height);
  const renderedWidth = source.width * scale;
  const renderedHeight = source.height * scale;
  return {
    scale,
    renderedWidth,
    renderedHeight,
    offsetX: (viewport.width - renderedWidth) / 2,
    offsetY: (viewport.height - renderedHeight) / 2,
  };
}

export function createFullImageSelection(imageSize: Size): Bounds {
  return {
    x: 0,
    y: 0,
    width: Math.max(0, imageSize.width),
    height: Math.max(0, imageSize.height),
  };
}

export function resolveSelectionDrag(
  previous: Bounds,
  from: Point,
  to: Point,
  imageSize: Size,
  minimumSize = MINIMUM_SELECTION_SIZE,
): Bounds {
  const next = clampBounds(normalizeRect(from, to), imageSize.width, imageSize.height);
  if (!next || next.width < minimumSize || next.height < minimumSize) {
    return previous;
  }
  return next;
}

export function resolveSelectionResize(
  previous: Bounds,
  handle: SelectionResizeHandle,
  delta: Point,
  imageSize: Size,
  minimumSize = MINIMUM_SELECTION_SIZE,
): Bounds {
  const safe = clampBounds(previous, imageSize.width, imageSize.height);
  const descriptor = CAPTURE_SELECTION_HANDLES.find((entry) => entry.handle === handle);
  if (
    !safe ||
    !descriptor ||
    minimumSize <= 0 ||
    safe.width < minimumSize ||
    safe.height < minimumSize
  ) {
    return previous;
  }

  let left = safe.x;
  let top = safe.y;
  let right = safe.x + safe.width;
  let bottom = safe.y + safe.height;

  if (descriptor.horizontalEdge === "left") {
    left = Math.min(Math.max(0, left + delta.x), right - minimumSize);
  } else if (descriptor.horizontalEdge === "right") {
    right = Math.max(Math.min(imageSize.width, right + delta.x), left + minimumSize);
  }

  if (descriptor.verticalEdge === "top") {
    top = Math.min(Math.max(0, top + delta.y), bottom - minimumSize);
  } else if (descriptor.verticalEdge === "bottom") {
    bottom = Math.max(Math.min(imageSize.height, bottom + delta.y), top + minimumSize);
  }

  return { x: left, y: top, width: right - left, height: bottom - top };
}

export function moveSelectionBy(
  previous: Bounds,
  delta: Point,
  imageSize: Size,
): Bounds {
  const safe = clampBounds(previous, imageSize.width, imageSize.height);
  if (!safe) {
    return previous;
  }

  return {
    ...safe,
    x: Math.min(Math.max(0, safe.x + delta.x), imageSize.width - safe.width),
    y: Math.min(Math.max(0, safe.y + delta.y), imageSize.height - safe.height),
  };
}

export function resolveSelectionNudge(input: SelectionNudgeInput): Point | null {
  if (
    !input.selectToolActive ||
    input.editableTarget ||
    input.composing ||
    input.pointerActive ||
    input.metaKey ||
    input.ctrlKey ||
    input.altKey ||
    input.shiftKey
  ) {
    return null;
  }

  switch (input.key) {
    case "ArrowLeft":
      return { x: -1, y: 0 };
    case "ArrowRight":
      return { x: 1, y: 0 };
    case "ArrowUp":
      return { x: 0, y: -1 };
    case "ArrowDown":
      return { x: 0, y: 1 };
    default:
      return null;
  }
}

export function imageBoundsToViewportBounds(
  bounds: Bounds,
  source: Size,
  viewport: Size,
): Bounds | null {
  const geometry = containGeometry(source, viewport);
  const safeBounds = clampBounds(bounds, source.width, source.height);
  if (!geometry || !safeBounds) {
    return null;
  }

  return {
    x: geometry.offsetX + safeBounds.x * geometry.scale,
    y: geometry.offsetY + safeBounds.y * geometry.scale,
    width: safeBounds.width * geometry.scale,
    height: safeBounds.height * geometry.scale,
  };
}

export function viewportPointToImagePoint(
  point: Point,
  source: Size,
  viewport: Size,
  clampToImage = false,
): Point | null {
  const geometry = containGeometry(source, viewport);
  if (!geometry) {
    return null;
  }

  const x = (point.x - geometry.offsetX) / geometry.scale;
  const y = (point.y - geometry.offsetY) / geometry.scale;
  if (!clampToImage && (x < 0 || y < 0 || x > source.width || y > source.height)) {
    return null;
  }
  return {
    x: Math.min(Math.max(0, x), source.width),
    y: Math.min(Math.max(0, y), source.height),
  };
}

export function isPointInBounds(point: Point, bounds: Bounds): boolean {
  return (
    point.x >= bounds.x &&
    point.x <= bounds.x + bounds.width &&
    point.y >= bounds.y &&
    point.y <= bounds.y + bounds.height
  );
}

export function resolveSelectPointerTarget(
  annotations: AnnotationObject[],
  point: Point,
): SelectPointerTarget {
  const annotation = hitTestAnnotation(annotations, point);
  return annotation
    ? { kind: "annotation", annotationId: annotation.id }
    : { kind: "selection" };
}

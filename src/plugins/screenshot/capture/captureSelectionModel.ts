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
): Point | null {
  const geometry = containGeometry(source, viewport);
  if (!geometry) {
    return null;
  }

  const x = (point.x - geometry.offsetX) / geometry.scale;
  const y = (point.y - geometry.offsetY) / geometry.scale;
  if (x < 0 || y < 0 || x > source.width || y > source.height) {
    return null;
  }
  return { x, y };
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

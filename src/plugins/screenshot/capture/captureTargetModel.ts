import { clampBounds, type Bounds } from "./captureCanvas.js";
import type { Point, ScreenshotTargetCandidate } from "./captureTypes.js";
import type { Size } from "./captureSelectionModel.js";

export const TARGET_DRAG_THRESHOLD_CSS_PX = 4;

export interface ResolvedScreenshotTarget {
  id: string;
  kind: "window" | "screen";
  bounds: Bounds;
}

export function normalizeScreenshotTargets(
  targets: readonly ScreenshotTargetCandidate[],
  imageSize: Size,
): ScreenshotTargetCandidate[] {
  const seen = new Set<string>();
  const normalized: ScreenshotTargetCandidate[] = [];
  for (const target of targets) {
    if (target.kind !== "window" || seen.has(target.id)) {
      continue;
    }
    const bounds = clampBounds(target.bounds, imageSize.width, imageSize.height);
    if (!bounds) {
      continue;
    }
    seen.add(target.id);
    normalized.push({ id: target.id, kind: "window", bounds });
  }
  return normalized;
}

export function resolveScreenshotTargetAtPoint(
  targets: readonly ScreenshotTargetCandidate[],
  point: Point,
  imageSize: Size,
): ResolvedScreenshotTarget | null {
  if (
    point.x < 0 ||
    point.y < 0 ||
    point.x > imageSize.width ||
    point.y > imageSize.height
  ) {
    return null;
  }

  const windowTarget = targets.find(
    ({ bounds }) =>
      point.x >= bounds.x &&
      point.x <= bounds.x + bounds.width &&
      point.y >= bounds.y &&
      point.y <= bounds.y + bounds.height,
  );
  if (windowTarget) {
    return windowTarget;
  }
  return {
    id: "screen",
    kind: "screen",
    bounds: { x: 0, y: 0, width: imageSize.width, height: imageSize.height },
  };
}

export function hasExceededTargetDragThreshold(
  start: Point,
  current: Point,
  threshold = TARGET_DRAG_THRESHOLD_CSS_PX,
): boolean {
  return Math.hypot(current.x - start.x, current.y - start.y) > threshold;
}

export function resolveStableTargetClick(
  start: ResolvedScreenshotTarget | null,
  end: ResolvedScreenshotTarget | null,
  dragged: boolean,
): Bounds | null {
  if (!start || !end || dragged || start.id !== end.id || start.kind !== end.kind) {
    return null;
  }
  return { ...end.bounds };
}

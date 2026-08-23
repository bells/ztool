import { clampBounds, type Bounds } from "./captureCanvas.js";
import type { Size } from "./captureSelectionModel.js";

export const MINIMUM_SELECTION_DIMENSION = 4;
export const GEOMETRY_CONTROL_SAFE_MARGIN = 8;
export const GEOMETRY_CONTROL_GAP = 8;
export const GEOMETRY_CONTROL_INSET = 8;

export interface SelectionGeometry {
  bounds: Bounds;
  cornerRadius: number;
}

export type SelectionDimension = "width" | "height";

export interface DimensionCommitResult {
  geometry: SelectionGeometry;
  valid: boolean;
}

export type GeometryControlPlacement = "outside-top" | "inside-top" | "viewport-clamped";

export interface GeometryControlPosition {
  left: number;
  top: number;
  placement: GeometryControlPlacement;
}

function clamp(value: number, minimum: number, maximum: number): number {
  if (maximum < minimum) {
    return minimum;
  }
  return Math.max(minimum, Math.min(value, maximum));
}

export function maximumSelectionCornerRadius(bounds: Bounds): number {
  return Math.max(0, Math.floor(Math.min(bounds.width, bounds.height) / 2));
}

export function selectionGeometryFromBounds(
  bounds: Bounds,
  cornerRadius = 0,
): SelectionGeometry {
  return { bounds: { ...bounds }, cornerRadius };
}

export function normalizeSelectionGeometry(
  geometry: SelectionGeometry,
  imageSize: Size,
): SelectionGeometry {
  const bounds = clampBounds(geometry.bounds, imageSize.width, imageSize.height) ?? {
    x: 0,
    y: 0,
    width: 0,
    height: 0,
  };
  return {
    bounds,
    cornerRadius: clamp(
      Number.isFinite(geometry.cornerRadius) ? Math.round(geometry.cornerRadius) : 0,
      0,
      maximumSelectionCornerRadius(bounds),
    ),
  };
}

export function commitSelectionDimension(
  current: SelectionGeometry,
  dimension: SelectionDimension,
  draft: string,
  imageSize: Size,
  minimum = MINIMUM_SELECTION_DIMENSION,
): DimensionCommitResult {
  const value = /^\d+$/.test(draft.trim()) ? Number(draft.trim()) : Number.NaN;
  if (!Number.isInteger(value) || value < minimum) {
    return { geometry: current, valid: false };
  }

  const maximum =
    dimension === "width"
      ? Math.max(minimum, imageSize.width - current.bounds.x)
      : Math.max(minimum, imageSize.height - current.bounds.y);
  const bounds = {
    ...current.bounds,
    [dimension]: Math.min(value, maximum),
  };
  return {
    geometry: normalizeSelectionGeometry({ ...current, bounds }, imageSize),
    valid: true,
  };
}

export function resolveGeometryControlPosition(
  selection: Bounds,
  control: Size,
  viewport: Size,
): GeometryControlPosition {
  const maximumLeft = viewport.width - control.width - GEOMETRY_CONTROL_SAFE_MARGIN;
  const left = clamp(selection.x, GEOMETRY_CONTROL_SAFE_MARGIN, maximumLeft);
  const outsideTop = selection.y - control.height - GEOMETRY_CONTROL_GAP;
  if (outsideTop >= GEOMETRY_CONTROL_SAFE_MARGIN) {
    return { left, top: outsideTop, placement: "outside-top" };
  }

  const insideTop = selection.y + GEOMETRY_CONTROL_INSET;
  if (selection.height >= control.height + GEOMETRY_CONTROL_INSET * 2) {
    return { left, top: insideTop, placement: "inside-top" };
  }

  const maximumTop = viewport.height - control.height - GEOMETRY_CONTROL_SAFE_MARGIN;
  return {
    left,
    top: clamp(insideTop, GEOMETRY_CONTROL_SAFE_MARGIN, maximumTop),
    placement: "viewport-clamped",
  };
}

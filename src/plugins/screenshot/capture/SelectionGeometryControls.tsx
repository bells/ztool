import {
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import {
  commitSelectionDimension,
  maximumSelectionCornerRadius,
  normalizeSelectionGeometry,
  resolveGeometryControlPosition,
  type SelectionDimension,
  type SelectionGeometry,
} from "./captureGeometryModel";
import type { Bounds } from "./captureCanvas";
import type { Size } from "./captureSelectionModel";

interface SelectionGeometryControlLabels {
  group: string;
  width: string;
  height: string;
  radius: string;
  invalid: string;
}

interface SelectionGeometryControlsProps {
  geometry: SelectionGeometry;
  imageSize: Size;
  viewportBounds: Bounds;
  viewportSize: Size;
  labels: SelectionGeometryControlLabels;
  disabled?: boolean;
  onChange: (geometry: SelectionGeometry) => void;
}

interface DimensionDrafts {
  width: string;
  height: string;
}

export function SelectionGeometryControls({
  geometry,
  imageSize,
  viewportBounds,
  viewportSize,
  labels,
  disabled = false,
  onChange,
}: SelectionGeometryControlsProps) {
  const controlRef = useRef<HTMLDivElement | null>(null);
  const skipBlurCommitRef = useRef<SelectionDimension | null>(null);
  const editStartRef = useRef<DimensionDrafts>({
    width: String(Math.round(geometry.bounds.width)),
    height: String(Math.round(geometry.bounds.height)),
  });
  const [controlSize, setControlSize] = useState<Size>({ width: 292, height: 46 });
  const [drafts, setDrafts] = useState<DimensionDrafts>(editStartRef.current);
  const [invalid, setInvalid] = useState<SelectionDimension | null>(null);

  useLayoutEffect(() => {
    const next = {
      width: String(Math.round(geometry.bounds.width)),
      height: String(Math.round(geometry.bounds.height)),
    };
    editStartRef.current = next;
    setDrafts(next);
    setInvalid(null);
  }, [geometry.bounds.height, geometry.bounds.width]);

  useLayoutEffect(() => {
    const control = controlRef.current;
    if (!control) {
      return;
    }
    const measure = () => {
      const rect = control.getBoundingClientRect();
      setControlSize({ width: rect.width, height: rect.height });
    };
    const observer = new ResizeObserver(measure);
    observer.observe(control);
    measure();
    return () => observer.disconnect();
  }, []);

  const position = useMemo(
    () => resolveGeometryControlPosition(viewportBounds, controlSize, viewportSize),
    [controlSize, viewportBounds, viewportSize],
  );

  const commitDraft = (dimension: SelectionDimension) => {
    const result = commitSelectionDimension(geometry, dimension, drafts[dimension], imageSize);
    if (!result.valid) {
      setInvalid(dimension);
      return;
    }
    setInvalid(null);
    onChange(result.geometry);
  };

  const rollbackDraft = (dimension: SelectionDimension) => {
    setDrafts((current) => ({ ...current, [dimension]: editStartRef.current[dimension] }));
    setInvalid(null);
  };

  const handleDimensionKeyDown = (
    event: KeyboardEvent<HTMLInputElement>,
    dimension: SelectionDimension,
  ) => {
    event.stopPropagation();
    if (event.nativeEvent.isComposing) {
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      skipBlurCommitRef.current = dimension;
      rollbackDraft(dimension);
      event.currentTarget.blur();
    } else if (event.key === "Enter" || event.key === "Tab") {
      commitDraft(dimension);
    }
  };

  const maximumRadius = maximumSelectionCornerRadius(geometry.bounds);

  return (
    <div
      ref={controlRef}
      className="capture-geometry-controls"
      role="group"
      aria-label={labels.group}
      data-placement={position.placement}
      style={{ left: position.left, top: position.top }}
      onPointerDown={(event) => event.stopPropagation()}
      onPointerMove={(event) => event.stopPropagation()}
      onPointerUp={(event) => event.stopPropagation()}
    >
      <label className="capture-geometry-field">
        <span>{labels.width}</span>
        <input
          type="text"
          inputMode="numeric"
          aria-label={labels.width}
          aria-invalid={invalid === "width"}
          disabled={disabled}
          value={drafts.width}
          onFocus={() => {
            editStartRef.current.width = String(Math.round(geometry.bounds.width));
          }}
          onChange={(event) => {
            setDrafts((current) => ({ ...current, width: event.target.value }));
            setInvalid(null);
          }}
          onBlur={() => {
            if (skipBlurCommitRef.current === "width") {
              skipBlurCommitRef.current = null;
            } else {
              commitDraft("width");
            }
          }}
          onKeyDown={(event) => handleDimensionKeyDown(event, "width")}
          onCompositionStart={(event) => event.stopPropagation()}
          onCompositionEnd={(event) => event.stopPropagation()}
        />
      </label>
      <span className="capture-geometry-times" aria-hidden="true">×</span>
      <label className="capture-geometry-field">
        <span>{labels.height}</span>
        <input
          type="text"
          inputMode="numeric"
          aria-label={labels.height}
          aria-invalid={invalid === "height"}
          disabled={disabled}
          value={drafts.height}
          onFocus={() => {
            editStartRef.current.height = String(Math.round(geometry.bounds.height));
          }}
          onChange={(event) => {
            setDrafts((current) => ({ ...current, height: event.target.value }));
            setInvalid(null);
          }}
          onBlur={() => {
            if (skipBlurCommitRef.current === "height") {
              skipBlurCommitRef.current = null;
            } else {
              commitDraft("height");
            }
          }}
          onKeyDown={(event) => handleDimensionKeyDown(event, "height")}
          onCompositionStart={(event) => event.stopPropagation()}
          onCompositionEnd={(event) => event.stopPropagation()}
        />
      </label>
      <label className="capture-radius-field">
        <span>{labels.radius}</span>
        <input
          type="range"
          min="0"
          max={maximumRadius}
          step="1"
          aria-label={labels.radius}
          disabled={disabled}
          value={geometry.cornerRadius}
          onKeyDown={(event) => event.stopPropagation()}
          onChange={(event) =>
            onChange(
              normalizeSelectionGeometry(
                { ...geometry, cornerRadius: Number(event.target.value) },
                imageSize,
              ),
            )
          }
        />
        <output>{geometry.cornerRadius}</output>
      </label>
      {invalid ? (
        <span className="capture-geometry-invalid" role="status">
          {labels.invalid}
        </span>
      ) : null}
    </div>
  );
}

import type { AnnotationObject, ArrowAnnotation, MosaicAnnotation, PenAnnotation, Point } from "./captureTypes";

export interface Bounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

function distance(from: Point, to: Point): number {
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  return Math.sqrt(dx * dx + dy * dy);
}

export function normalizeRect(from: Point, to: Point): Bounds {
  const x = Math.min(from.x, to.x);
  const y = Math.min(from.y, to.y);
  const width = Math.abs(to.x - from.x);
  const height = Math.abs(to.y - from.y);
  return { x, y, width, height };
}

export function annotationBounds(annotation: AnnotationObject): Bounds | null {
  switch (annotation.type) {
    case "rectangle":
    case "ellipse":
    case "mosaic":
    case "pin":
      return {
        x: annotation.x,
        y: annotation.y,
        width: annotation.width,
        height: annotation.height,
      };
    case "arrow": {
      const fromX = Math.min(annotation.from.x, annotation.to.x);
      const fromY = Math.min(annotation.from.y, annotation.to.y);
      const width = Math.abs(annotation.from.x - annotation.to.x);
      const height = Math.abs(annotation.from.y - annotation.to.y);
      return {
        x: fromX,
        y: fromY,
        width,
        height,
      };
    }
    case "pen": {
      if (annotation.points.length === 0) {
        return null;
      }
      const xs = annotation.points.map((point) => point.x);
      const ys = annotation.points.map((point) => point.y);
      const minX = Math.min(...xs);
      const maxX = Math.max(...xs);
      const minY = Math.min(...ys);
      const maxY = Math.max(...ys);
      return {
        x: minX,
        y: minY,
        width: maxX - minX,
        height: maxY - minY,
      };
    }
    case "text": {
      const lines = annotation.text.split(/\r?\n/);
      const longestLine = Math.max(...lines.map((line) => line.length), 1);
      const width = Math.max(24, Math.round(longestLine * annotation.fontSize * 0.56));
      return {
        x: annotation.x,
        y: annotation.y - annotation.fontSize,
        width,
        height: lines.length * annotation.fontSize * 1.35,
      };
    }
    default:
      return null;
  }
}

export function isAnnotationLargeEnough(annotation: AnnotationObject, minimumSize = 4): boolean {
  if (annotation.type === "arrow") {
    return distance(annotation.from, annotation.to) >= minimumSize;
  }

  if (annotation.type === "pen") {
    return annotation.points.length >= 2;
  }

  if (annotation.type === "text") {
    return annotation.text.trim().length > 0;
  }

  const bounds = annotationBounds(annotation);
  if (!bounds) {
    return false;
  }

  return bounds.width >= minimumSize && bounds.height >= minimumSize;
}

export function clampBounds(bounds: Bounds, maxWidth: number, maxHeight: number): Bounds | null {
  const x = Math.max(0, Math.min(bounds.x, maxWidth));
  const y = Math.max(0, Math.min(bounds.y, maxHeight));
  const right = Math.max(0, Math.min(bounds.x + bounds.width, maxWidth));
  const bottom = Math.max(0, Math.min(bounds.y + bounds.height, maxHeight));
  const width = right - x;
  const height = bottom - y;
  if (width <= 0 || height <= 0) {
    return null;
  }

  return { x, y, width, height };
}

export function hitTestAnnotation(
  annotations: AnnotationObject[],
  point: Point,
): AnnotationObject | null {
  for (let index = annotations.length - 1; index >= 0; index -= 1) {
    const bounds = annotationBounds(annotations[index]);
    if (!bounds) {
      continue;
    }

    const padding = Math.max(6, annotations[index].strokeWidth);
    const insideX = point.x >= bounds.x - padding && point.x <= bounds.x + bounds.width + padding;
    const insideY = point.y >= bounds.y - padding && point.y <= bounds.y + bounds.height + padding;
    if (insideX && insideY) {
      return annotations[index];
    }
  }

  return null;
}

function drawArrow(ctx: CanvasRenderingContext2D, arrow: ArrowAnnotation) {
  const { from, to, strokeWidth, color } = arrow;
  const length = distance(from, to);
  if (length < 2) {
    return;
  }

  const angle = Math.atan2(to.y - from.y, to.x - from.x);
  const headLength = Math.max(10, strokeWidth * 3.6);
  const left = {
    x: to.x - headLength * Math.cos(angle - Math.PI / 7),
    y: to.y - headLength * Math.sin(angle - Math.PI / 7),
  };
  const right = {
    x: to.x - headLength * Math.cos(angle + Math.PI / 7),
    y: to.y - headLength * Math.sin(angle + Math.PI / 7),
  };

  ctx.save();
  ctx.lineWidth = strokeWidth;
  ctx.strokeStyle = color;
  ctx.fillStyle = color;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";

  ctx.beginPath();
  ctx.moveTo(from.x, from.y);
  ctx.lineTo(to.x, to.y);
  ctx.stroke();

  ctx.beginPath();
  ctx.moveTo(to.x, to.y);
  ctx.lineTo(left.x, left.y);
  ctx.lineTo(right.x, right.y);
  ctx.closePath();
  ctx.fill();

  ctx.restore();
}

function drawPen(ctx: CanvasRenderingContext2D, pen: PenAnnotation) {
  const { points, color, strokeWidth } = pen;
  if (points.length === 0) {
    return;
  }

  ctx.save();
  ctx.lineWidth = strokeWidth;
  ctx.strokeStyle = color;
  ctx.lineCap = "round";
  ctx.lineJoin = "round";

  ctx.beginPath();
  ctx.moveTo(points[0].x, points[0].y);
  for (let index = 1; index < points.length; index += 1) {
    const point = points[index];
    ctx.lineTo(point.x, point.y);
  }
  ctx.stroke();
  ctx.restore();
}

function drawMosaic(ctx: CanvasRenderingContext2D, mosaic: MosaicAnnotation) {
  const { x, y, width, height, pixelSize } = mosaic;
  const safePixel = Math.max(2, Math.floor(pixelSize));
  const sourceX = Math.max(0, Math.floor(x));
  const sourceY = Math.max(0, Math.floor(y));
  const sourceWidth = Math.max(1, Math.floor(width));
  const sourceHeight = Math.max(1, Math.floor(height));

  const tinyWidth = Math.max(1, Math.floor(sourceWidth / safePixel));
  const tinyHeight = Math.max(1, Math.floor(sourceHeight / safePixel));

  const tinyCanvas = document.createElement("canvas");
  tinyCanvas.width = tinyWidth;
  tinyCanvas.height = tinyHeight;
  const tinyContext = tinyCanvas.getContext("2d");
  if (!tinyContext) {
    return;
  }

  tinyContext.imageSmoothingEnabled = false;
  tinyContext.drawImage(
    ctx.canvas,
    sourceX,
    sourceY,
    sourceWidth,
    sourceHeight,
    0,
    0,
    tinyWidth,
    tinyHeight,
  );

  ctx.save();
  ctx.imageSmoothingEnabled = false;
  ctx.drawImage(
    tinyCanvas,
    0,
    0,
    tinyWidth,
    tinyHeight,
    sourceX,
    sourceY,
    sourceWidth,
    sourceHeight,
  );
  ctx.strokeStyle = "rgba(0, 0, 0, 0.22)";
  ctx.lineWidth = 1;
  ctx.strokeRect(sourceX, sourceY, sourceWidth, sourceHeight);
  ctx.restore();
}

export function drawAnnotations(
  ctx: CanvasRenderingContext2D,
  annotations: AnnotationObject[],
): void {
  annotations.forEach((annotation) => {
    switch (annotation.type) {
      case "rectangle":
        ctx.save();
        ctx.lineWidth = annotation.strokeWidth;
        ctx.strokeStyle = annotation.color;
        ctx.strokeRect(annotation.x, annotation.y, annotation.width, annotation.height);
        ctx.restore();
        break;

      case "ellipse":
        ctx.save();
        ctx.lineWidth = annotation.strokeWidth;
        ctx.strokeStyle = annotation.color;
        ctx.beginPath();
        ctx.ellipse(
          annotation.x + annotation.width / 2,
          annotation.y + annotation.height / 2,
          Math.max(0.5, annotation.width / 2),
          Math.max(0.5, annotation.height / 2),
          0,
          0,
          Math.PI * 2,
        );
        ctx.stroke();
        ctx.restore();
        break;

      case "arrow":
        drawArrow(ctx, annotation);
        break;

      case "pen":
        drawPen(ctx, annotation);
        break;

      case "text":
        ctx.save();
        ctx.fillStyle = annotation.color;
        ctx.font = `${annotation.fontSize}px "Avenir Next", "PingFang SC", sans-serif`;
        ctx.textBaseline = "alphabetic";
        annotation.text.split(/\r?\n/).forEach((line, index) => {
          ctx.fillText(line, annotation.x, annotation.y + index * annotation.fontSize * 1.35);
        });
        ctx.restore();
        break;

      case "mosaic":
        drawMosaic(ctx, annotation);
        break;

      case "pin":
        break;

      default:
        break;
    }
  });
}

import type { Bounds } from "./captureCanvas";
import { clampBounds, drawAnnotations } from "./captureCanvas.js";
import type { AnnotationObject } from "./captureTypes";

export async function loadImageFromObjectUrl(objectUrl: string): Promise<HTMLImageElement> {
  const image = new Image();
  image.src = objectUrl;
  try {
    await image.decode();
    return image;
  } catch (error) {
    image.src = "";
    throw error;
  }
}

export function releaseDecodedImage(image: HTMLImageElement | null): void {
  if (image) {
    image.src = "";
  }
}

export function releaseObjectUrl(
  objectUrl: string | null,
  revoke: (url: string) => void = URL.revokeObjectURL.bind(URL),
): null {
  if (objectUrl) {
    revoke(objectUrl);
  }
  return null;
}

export function createSizedCanvas(width: number, height: number): HTMLCanvasElement {
  const canvas = document.createElement("canvas");
  canvas.width = Math.max(1, Math.floor(width));
  canvas.height = Math.max(1, Math.floor(height));
  return canvas;
}

export function releaseCanvas(canvas: HTMLCanvasElement | null): void {
  if (!canvas) {
    return;
  }
  canvas.width = 1;
  canvas.height = 1;
}

export function renderFinalCanvas(
  baseImage: CanvasImageSource,
  width: number,
  height: number,
  annotations: AnnotationObject[],
): HTMLCanvasElement {
  const canvas = createSizedCanvas(width, height);
  const ctx = canvas.getContext("2d");
  if (!ctx) {
    releaseCanvas(canvas);
    throw new Error("Canvas context is unavailable");
  }
  ctx.drawImage(baseImage, 0, 0, width, height);
  drawAnnotations(ctx, annotations.filter((annotation) => annotation.type !== "pin"));
  return canvas;
}

export function canvasToPngBytes(canvas: HTMLCanvasElement): Promise<Uint8Array> {
  return new Promise((resolve, reject) => {
    canvas.toBlob((blob) => {
      if (!blob) {
        reject(new Error("The screenshot canvas could not be encoded as PNG"));
        return;
      }
      blob
        .arrayBuffer()
        .then((buffer) => resolve(new Uint8Array(buffer)), reject);
    }, "image/png");
  });
}

export async function cropCanvasToPngBytes(
  source: HTMLCanvasElement,
  bounds: Bounds,
): Promise<Uint8Array | null> {
  const safeBounds = clampBounds(bounds, source.width, source.height);
  if (!safeBounds) {
    return null;
  }
  const target = createSizedCanvas(safeBounds.width, safeBounds.height);
  try {
    const ctx = target.getContext("2d");
    if (!ctx) {
      throw new Error("Canvas context is unavailable");
    }
    ctx.drawImage(
      source,
      safeBounds.x,
      safeBounds.y,
      safeBounds.width,
      safeBounds.height,
      0,
      0,
      safeBounds.width,
      safeBounds.height,
    );
    return await canvasToPngBytes(target);
  } finally {
    releaseCanvas(target);
  }
}

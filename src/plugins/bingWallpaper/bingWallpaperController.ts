import type { BingWallpaperService } from "./bingWallpaperServiceCore.js";
import type { BingWallpaperSnapshot } from "./contracts.js";

export interface RequestGate {
  isCurrent(): boolean;
  dispose(): void;
}

export interface ActionGate {
  tryStart(action: "apply" | "save"): boolean;
  finish(action: "apply" | "save"): void;
  current(): "apply" | "save" | null;
}

export function createRequestGate(): RequestGate {
  let current = true;
  return {
    isCurrent: () => current,
    dispose: () => {
      current = false;
    },
  };
}

export function createActionGate(): ActionGate {
  let active: "apply" | "save" | null = null;
  return {
    tryStart(action) {
      if (active !== null) {
        return false;
      }
      active = action;
      return true;
    },
    finish(action) {
      if (active === action) {
        active = null;
      }
    },
    current: () => active,
  };
}

export async function loadBingWallpaperCacheFirst(
  service: Pick<BingWallpaperService, "snapshot" | "refresh">,
  gate: RequestGate,
  onSnapshot: (snapshot: BingWallpaperSnapshot) => void,
  onError: (message: string) => void,
) {
  try {
    const cached = await service.snapshot();
    if (!gate.isCurrent()) {
      return;
    }
    onSnapshot(cached);

    const refreshed = await service.refresh();
    if (!gate.isCurrent()) {
      return;
    }
    onSnapshot(refreshed);
    if (refreshed.error) {
      onError(refreshed.error.message);
    }
  } catch (loadError) {
    if (gate.isCurrent()) {
      onError(errorMessage(loadError));
    }
  }
}

export function errorMessage(value: unknown) {
  if (value instanceof Error) {
    return value.message;
  }
  if (isRecord(value) && typeof value.message === "string") {
    return value.message;
  }
  return String(value);
}

export function nextBingWallpaperReloadVersion(current: number) {
  return current + 1;
}

export function shouldStartBingWallpaperPresentation(
  activity: "active" | "hidden" | "disposed",
) {
  return activity === "active";
}

export function previewBytesMatchDescriptor(
  actualByteLength: number,
  expectedByteLength: number,
) {
  return actualByteLength > 0 && actualByteLength === expectedByteLength;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

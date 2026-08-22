export const SURFACE_ACTIVITY_EVENT = "zero://surface-activity";

export type SurfaceActivityState = "active" | "hidden" | "disposed";

export interface SurfaceActivityPayload {
  label: string;
  state: SurfaceActivityState;
}

export interface SurfaceActivityModel {
  acceptNative(payload: SurfaceActivityPayload): void;
  setDocumentVisibility(visibility: DocumentVisibilityState): void;
  dispose(): void;
  snapshot(): SurfaceActivityState;
}

export function combineSurfaceActivity(
  nativeState: SurfaceActivityState,
  documentVisibility: DocumentVisibilityState,
): SurfaceActivityState {
  if (nativeState === "disposed") return "disposed";
  return nativeState === "active" && documentVisibility === "visible"
    ? "active"
    : "hidden";
}

export function createSurfaceActivityModel(
  label: string,
  initialDocumentVisibility: DocumentVisibilityState,
  onChange: (state: SurfaceActivityState) => void,
): SurfaceActivityModel {
  let nativeState: SurfaceActivityState = "hidden";
  let documentVisibility = initialDocumentVisibility;
  let currentState = combineSurfaceActivity(nativeState, documentVisibility);
  let disposed = false;

  const publish = () => {
    const next = combineSurfaceActivity(nativeState, documentVisibility);
    if (next === currentState) return;
    currentState = next;
    onChange(next);
  };

  return {
    acceptNative(payload) {
      if (disposed || payload.label !== label) return;
      nativeState = payload.state;
      if (payload.state === "disposed") disposed = true;
      publish();
    },
    setDocumentVisibility(visibility) {
      if (disposed) return;
      documentVisibility = visibility;
      publish();
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      nativeState = "disposed";
      publish();
    },
    snapshot() {
      return currentState;
    },
  };
}

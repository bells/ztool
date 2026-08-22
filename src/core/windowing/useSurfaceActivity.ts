import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  createSurfaceActivityModel,
  SURFACE_ACTIVITY_EVENT,
  type SurfaceActivityPayload,
  type SurfaceActivityState,
} from "./surfaceActivityCore";

export function useSurfaceActivity(): SurfaceActivityState {
  const [activity, setActivity] = useState<SurfaceActivityState>("hidden");

  useEffect(() => {
    const label = getCurrentWindow().label;
    let live = true;
    let eventRevision = 0;
    let unlisten: UnlistenFn | undefined;
    const model = createSurfaceActivityModel(
      label,
      document.visibilityState,
      (next) => {
        if (live) setActivity(next);
      },
    );

    const refreshNativeSnapshot = async () => {
      const revision = eventRevision;
      const payload = await invoke<SurfaceActivityPayload>("get_surface_activity");
      if (live && revision === eventRevision) model.acceptNative(payload);
    };

    const onVisibilityChange = () => {
      model.setDocumentVisibility(document.visibilityState);
      if (document.visibilityState === "visible") {
        void refreshNativeSnapshot().catch(() => undefined);
      }
    };
    const onBeforeUnload = () => model.dispose();

    document.addEventListener("visibilitychange", onVisibilityChange);
    window.addEventListener("beforeunload", onBeforeUnload);

    void listen<SurfaceActivityPayload>(SURFACE_ACTIVITY_EVENT, (event) => {
      eventRevision += 1;
      model.acceptNative(event.payload);
    }).then((stop) => {
      if (!live) {
        stop();
        return;
      }
      unlisten = stop;
      void refreshNativeSnapshot().catch(() => undefined);
    });

    return () => {
      live = false;
      model.dispose();
      unlisten?.();
      document.removeEventListener("visibilitychange", onVisibilityChange);
      window.removeEventListener("beforeunload", onBeforeUnload);
    };
  }, []);

  return activity;
}

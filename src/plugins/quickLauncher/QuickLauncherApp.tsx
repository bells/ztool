import { useCallback, useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { resolveLanguage } from "../../core/preferences/i18n";
import { useSurfaceActivity } from "../../core/windowing/useSurfaceActivity";
import { createQuickLauncherTranslator } from "./i18n";
import { QuickLauncherView } from "./QuickLauncherView";
import { shouldDismissLauncher } from "./quickLauncherModel";
import { quickLauncherService } from "./quickLauncherService";
import { useQuickLauncher } from "./useQuickLauncher";

export default function QuickLauncherApp() {
  const controller = useQuickLauncher();
  const surfaceActivity = useSurfaceActivity();
  const [focusEpoch, setFocusEpoch] = useState(0);
  const t = createQuickLauncherTranslator(
    resolveLanguage("system", navigator.language),
  );
  const hide = useCallback(() => {
    void quickLauncherService.hideWindow();
  }, []);

  useEffect(() => {
    if (surfaceActivity !== "active") return;
    controller.resetTransient();
    setFocusEpoch((value) => value + 1);
  }, [controller.resetTransient, surfaceActivity]);

  useEffect(() => {
    if (surfaceActivity !== "active") return;

    let unlistenFocus: (() => void) | null = null;
    let blurDismissTimer: ReturnType<typeof setTimeout> | null = null;
    let disposed = false;
    const launcherWindow = getCurrentWindow();
    launcherWindow.onFocusChanged(({ payload: focused }) => {
      if (focused && blurDismissTimer !== null) {
        clearTimeout(blurDismissTimer);
        blurDismissTimer = null;
      } else if (!focused && shouldDismissLauncher("floating", "blur", controller.activatingId !== null)) {
        blurDismissTimer = setTimeout(() => {
          blurDismissTimer = null;
          void launcherWindow.isFocused()
            .then((stillFocused) => {
              if (!stillFocused) hide();
            })
            .catch(hide);
        }, 120);
      }
    }).then((unlisten) => {
      if (disposed) unlisten(); else unlistenFocus = unlisten;
    });
    return () => {
      disposed = true;
      if (blurDismissTimer !== null) clearTimeout(blurDismissTimer);
      unlistenFocus?.();
    };
  }, [controller.activatingId, hide, surfaceActivity]);

  return (
    <main className="quick-launcher-window">
      <QuickLauncherView
        controller={controller}
        surface="floating"
        focusEpoch={focusEpoch}
        t={t}
        onDismiss={hide}
        onActivationSuccess={hide}
      />
    </main>
  );
}

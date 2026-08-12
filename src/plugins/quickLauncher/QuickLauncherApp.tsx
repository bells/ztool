import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { createTranslator, resolveLanguage } from "../preferences/i18n";
import { QuickLauncherView } from "./QuickLauncherView";
import { shouldDismissLauncher } from "./quickLauncherModel";
import { quickLauncherService } from "./quickLauncherService";
import { useQuickLauncher } from "./useQuickLauncher";

export default function QuickLauncherApp() {
  const controller = useQuickLauncher();
  const [focusEpoch, setFocusEpoch] = useState(0);
  const t = createTranslator(resolveLanguage("system", navigator.language));
  const hide = useCallback(() => {
    void quickLauncherService.hideWindow();
  }, []);

  useEffect(() => {
    let unlistenShown: (() => void) | null = null;
    let unlistenFocus: (() => void) | null = null;
    let blurDismissTimer: ReturnType<typeof setTimeout> | null = null;
    let disposed = false;
    const launcherWindow = getCurrentWindow();
    listen("quick-launcher-shown", () => {
      controller.resetTransient();
      setFocusEpoch((value) => value + 1);
    }).then((unlisten) => {
      if (disposed) unlisten(); else unlistenShown = unlisten;
    });
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
      unlistenShown?.();
      unlistenFocus?.();
    };
  }, [controller.activatingId, controller.resetTransient, hide]);

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

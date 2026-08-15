import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { TranslationKey } from "./i18n";

export type ScreenshotAction = "copy" | "save";

interface ScreenshotCapabilities {
  platform: string;
  selection_visual: boolean;
  custom_overlay: boolean;
  system_launcher: boolean;
  active_actions: string[];
  pending_tools: string[];
}

interface ScreenshotStartResult {
  mode: string;
  action: ScreenshotAction;
  message: string;
}

const defaultCapabilities: ScreenshotCapabilities = {
  platform: "Unknown",
  selection_visual: true,
  custom_overlay: false,
  system_launcher: false,
  active_actions: [],
  pending_tools: [],
};

export function useScreenshotPlugin() {
  const [capabilities, setCapabilities] =
    useState<ScreenshotCapabilities>(defaultCapabilities);
  const [messageKey, setMessageKey] =
    useState<TranslationKey>("screenshot.status.initializing");
  const [messageDetail, setMessageDetail] = useState<string | null>(null);
  const [isBusy, setIsBusy] = useState(false);

  const refresh = useCallback(async () => {
    const next = await invoke<ScreenshotCapabilities>("get_screenshot_capabilities");
    setCapabilities(next);
    setMessageKey(
      next.selection_visual
        ? "screenshot.status.ready"
        : "screenshot.status.fallback",
    );
    setMessageDetail(null);
  }, []);

  const start = useCallback(async (action: ScreenshotAction) => {
    setIsBusy(true);
    try {
      const result = await invoke<ScreenshotStartResult>("start_screenshot", { action });
      setMessageKey(
        result.action === "save"
          ? "screenshot.status.saveStarted"
          : "screenshot.status.copyStarted",
      );
      setMessageDetail(null);
    } catch (err) {
      setMessageKey("screenshot.status.startError");
      setMessageDetail(String(err));
    } finally {
      setIsBusy(false);
    }
  }, []);

  useEffect(() => {
    refresh().catch((err) => {
      setMessageKey("screenshot.status.initError");
      setMessageDetail(String(err));
    });
  }, [refresh]);

  return {
    capabilities,
    messageKey,
    messageDetail,
    isBusy,
    start,
  };
}

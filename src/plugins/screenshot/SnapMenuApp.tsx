import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ScanLine } from "lucide-react";
import { resolveLanguage } from "../../core/preferences/i18n";
import { normalizePreferences } from "../../core/preferences/preferencesModel";
import { readStoredPreferences } from "../../core/preferences/preferencesStorage";
import { useSurfaceActivity } from "../../core/windowing/useSurfaceActivity";
import { createScreenshotTranslator } from "./i18n";
import { SNAP_MENU_ITEMS } from "./snapMenuModel";

interface ScreenshotStartResult {
  mode: string;
  platform: string;
  action: string;
  message: string;
  session_id: string | null;
  capture_window_label: string | null;
}

function errorMessage(error: unknown): string {
  if (typeof error === "object" && error !== null && "message" in error) {
    const message = (error as { message?: unknown }).message;
    if (typeof message === "string") return message;
  }
  return String(error);
}

export default function SnapMenuApp() {
  const preferences = normalizePreferences(readStoredPreferences(window.localStorage), []);
  const t = createScreenshotTranslator(
    resolveLanguage(preferences.language, navigator.language),
  );
  const activity = useSurfaceActivity();
  const firstActionRef = useRef<HTMLButtonElement | null>(null);
  const [isStarting, setIsStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const dismiss = useCallback(() => {
    void invoke("hide_current_surface");
  }, []);

  const startScreenshot = useCallback(async () => {
    if (isStarting) return;
    setIsStarting(true);
    setError(null);
    try {
      await invoke<ScreenshotStartResult>("start_snap_menu_screenshot");
    } catch (reason) {
      setError(errorMessage(reason));
    } finally {
      setIsStarting(false);
    }
  }, [isStarting]);

  useEffect(() => {
    if (activity !== "active") return;
    const focusFrame = requestAnimationFrame(() => firstActionRef.current?.focus());
    return () => cancelAnimationFrame(focusFrame);
  }, [activity]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        dismiss();
      }
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [dismiss]);

  return (
    <main className="snap-menu-window">
      <nav className="snap-menu-card" aria-label={t("screenshot.menu.label")}>
        {SNAP_MENU_ITEMS.map((item, index) => (
          <button
            ref={index === 0 ? firstActionRef : undefined}
            type="button"
            className="snap-menu-action"
            key={item.id}
            disabled={isStarting}
            onClick={startScreenshot}
          >
            <span className="snap-menu-icon" aria-hidden="true">
              <ScanLine />
            </span>
            <span className="snap-menu-copy">
              <strong>{t(item.labelKey)}</strong>
              <small title={error ?? undefined} data-error={error ? "true" : "false"}>
                {error ?? t("screenshot.menu.hint")}
              </small>
            </span>
            <kbd aria-label={t("screenshot.menu.shortcut")}>⌘⇧A</kbd>
          </button>
        ))}
      </nav>
    </main>
  );
}

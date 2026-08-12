import { useCallback, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { BingWallpaperPanel } from "./BingWallpaperPanel";
import { createTranslator, resolveLanguage } from "../preferences/i18n";
import { normalizePreferences } from "../preferences/preferencesModel";
import { readStoredPreferences } from "../preferences/preferencesStorage";

export default function PaperApp() {
  const preferences = normalizePreferences(
    readStoredPreferences(window.localStorage),
    [],
  );
  const t = createTranslator(
    resolveLanguage(preferences.language, navigator.language),
  );
  const dismiss = useCallback(() => {
    void getCurrentWindow().hide();
  }, []);

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
    <main className="paper-window">
      <BingWallpaperPanel t={t} />
    </main>
  );
}

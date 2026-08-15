import { useCallback, useEffect, useMemo, useState } from "react";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import type { PluginId } from "../pluginHost/pluginTypes";
import type { TranslationKey } from "./i18n";
import {
  AppPreferences,
  LanguagePreference,
  getVisiblePluginIds,
  normalizePreferences,
  setLanguagePreference,
  setToolVisibility,
} from "./preferencesModel";
import {
  readStoredPreferences,
  writeCanonicalPreferences,
} from "./preferencesStorage";

export function usePreferences(pluginIds: PluginId[]) {
  const pluginIdsKey = pluginIds.join("\u0000");
  const [preferences, setPreferences] = useState<AppPreferences>(() =>
    normalizePreferences(readStoredPreferences(window.localStorage), pluginIds),
  );
  const [isAutostartBusy, setIsAutostartBusy] = useState(false);
  const [messageKey, setMessageKey] = useState<TranslationKey>("prefs.message.ready");
  const [messageDetail, setMessageDetail] = useState<string | null>(null);

  useEffect(() => {
    writeCanonicalPreferences(window.localStorage, preferences);
  }, [preferences]);

  useEffect(() => {
    setPreferences((current) => normalizePreferences(current, pluginIds));
  }, [pluginIdsKey]);

  useEffect(() => {
    isEnabled()
      .then((enabled) => {
        setPreferences((current) => ({
          ...current,
          launchAtLogin: enabled,
        }));
      })
      .catch((error) => {
        setMessageKey("prefs.message.autostartReadError");
        setMessageDetail(String(error));
      });
  }, []);

  const visiblePluginIds = useMemo(
    () => getVisiblePluginIds(pluginIds, preferences),
    [pluginIds, preferences],
  );

  const setLaunchAtLogin = useCallback(async (enabled: boolean) => {
    setIsAutostartBusy(true);
    try {
      if (enabled) {
        await enable();
      } else {
        await disable();
      }
      setPreferences((current) => ({
        ...current,
        launchAtLogin: enabled,
      }));
      setMessageKey(enabled ? "prefs.message.autostartOn" : "prefs.message.autostartOff");
      setMessageDetail(null);
    } catch (error) {
      setMessageKey("prefs.message.autostartWriteError");
      setMessageDetail(String(error));
    } finally {
      setIsAutostartBusy(false);
    }
  }, []);

  const setToolVisible = useCallback(
    (pluginId: PluginId, visible: boolean) => {
      setPreferences((current) => setToolVisibility(current, pluginId, visible, pluginIds));
      setMessageKey("prefs.message.toolsSaved");
      setMessageDetail(null);
    },
    [pluginIds],
  );

  const setLanguage = useCallback((language: LanguagePreference) => {
    setPreferences((current) => setLanguagePreference(current, language));
    setMessageKey("prefs.message.languageSaved");
    setMessageDetail(null);
  }, []);

  return {
    preferences,
    visiblePluginIds,
    isAutostartBusy,
    messageKey,
    messageDetail,
    setLaunchAtLogin,
    setToolVisible,
    setLanguage,
  };
}

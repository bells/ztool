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
import { updateLaunchAtLoginPreference } from "./preferencesActions";

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
      const next = await updateLaunchAtLoginPreference(preferences, enabled, {
        enable,
        disable,
      });
      setPreferences(next);
      setMessageKey(enabled ? "prefs.message.autostartOn" : "prefs.message.autostartOff");
      setMessageDetail(null);
    } catch (error) {
      setMessageKey("prefs.message.autostartWriteError");
      setMessageDetail(String(error));
      throw error;
    } finally {
      setIsAutostartBusy(false);
    }
  }, [preferences]);

  const setToolVisible = useCallback(
    (pluginId: PluginId, visible: boolean) => {
      if (
        !visible &&
        preferences.visibleTools[pluginId] &&
        visiblePluginIds.length <= 1
      ) {
        return false;
      }
      setPreferences((current) => setToolVisibility(current, pluginId, visible, pluginIds));
      setMessageKey("prefs.message.toolsSaved");
      setMessageDetail(null);
      return true;
    },
    [pluginIds, preferences.visibleTools, visiblePluginIds.length],
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

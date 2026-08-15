import {
  DEFAULT_PREFERENCES,
  type AppPreferences,
} from "./preferencesModel.js";

export const PREFERENCES_STORAGE_KEY = "zero.preferences.v1";
export const LEGACY_PREFERENCES_STORAGE_KEY = "ztool.preferences.v1";

export interface PreferencesStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

export function readStoredPreferences(storage: Pick<PreferencesStorage, "getItem">) {
  try {
    const stored =
      storage.getItem(PREFERENCES_STORAGE_KEY) ??
      storage.getItem(LEGACY_PREFERENCES_STORAGE_KEY);
    return stored ? JSON.parse(stored) : DEFAULT_PREFERENCES;
  } catch {
    return DEFAULT_PREFERENCES;
  }
}

export function writeCanonicalPreferences(
  storage: Pick<PreferencesStorage, "setItem">,
  preferences: AppPreferences,
) {
  storage.setItem(PREFERENCES_STORAGE_KEY, JSON.stringify(preferences));
}

import type { AppPreferences } from "./preferencesModel";

export interface AutostartApi {
  disable(): Promise<void>;
  enable(): Promise<void>;
}

export async function updateLaunchAtLoginPreference(
  preferences: AppPreferences,
  enabled: boolean,
  api: AutostartApi,
): Promise<AppPreferences> {
  if (enabled) {
    await api.enable();
  } else {
    await api.disable();
  }

  return {
    ...preferences,
    launchAtLogin: enabled,
  };
}

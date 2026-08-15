import type { PluginId } from "../pluginHost/pluginTypes";
import {
  FIRST_PARTY_PLUGIN_IDS,
  canonicalFirstPartyPluginId,
  legacyFirstPartyPluginIds,
} from "../../brand/identity.js";

export type LanguagePreference = "system" | "zh-CN" | "en-US";

export interface AppPreferences {
  launchAtLogin: boolean;
  language: LanguagePreference;
  visibleTools: Record<PluginId, boolean>;
}

export const DEFAULT_PREFERENCES: AppPreferences = {
  launchAtLogin: false,
  language: "system",
  visibleTools: {
    [FIRST_PARTY_PLUGIN_IDS.snap]: true,
    [FIRST_PARTY_PLUGIN_IDS.awake]: true,
  },
};

export function normalizePreferences(
  value: unknown,
  pluginIds: PluginId[],
): AppPreferences {
  const input = isPreferenceShape(value) ?? {};
  const visibleTools = pluginIds.reduce(
    (result, pluginId) => ({
      ...result,
      [pluginId]: readPluginVisibility(input.visibleTools, pluginId) ?? true,
    }),
    {} as Record<PluginId, boolean>,
  );

  return {
    launchAtLogin: input.launchAtLogin ?? false,
    language: normalizeLanguage(input.language),
    visibleTools: ensureAtLeastOneVisible(visibleTools, pluginIds),
  };
}

export function getVisiblePluginIds(
  pluginIds: PluginId[],
  preferences: AppPreferences,
): PluginId[] {
  return pluginIds.filter((pluginId) => preferences.visibleTools[pluginId]);
}

export function setToolVisibility(
  preferences: AppPreferences,
  pluginId: PluginId,
  visible: boolean,
  pluginIds: PluginId[],
): AppPreferences {
  const visibleTools = ensureAtLeastOneVisible(
    {
      ...preferences.visibleTools,
      [pluginId]: visible,
    },
    pluginIds,
  );

  return {
    ...preferences,
    visibleTools,
  };
}

export function setLanguagePreference(
  preferences: AppPreferences,
  language: LanguagePreference,
): AppPreferences {
  return {
    ...preferences,
    language,
  };
}

function normalizeLanguage(language: unknown): LanguagePreference {
  if (language === "zh-CN" || language === "en-US" || language === "system") {
    return language;
  }

  return "system";
}

function ensureAtLeastOneVisible(
  visibleTools: Record<PluginId, boolean>,
  pluginIds: PluginId[],
) {
  if (pluginIds.some((pluginId) => visibleTools[pluginId])) {
    return visibleTools;
  }

  const firstPluginId = pluginIds[0];
  if (!firstPluginId) {
    return visibleTools;
  }

  return {
    ...visibleTools,
    [firstPluginId]: true,
  };
}

function isPreferenceShape(value: unknown): Partial<AppPreferences> | null {
  if (!value || typeof value !== "object") {
    return null;
  }

  return value as Partial<AppPreferences>;
}

function readPluginVisibility(
  visibleTools: Record<string, boolean> | undefined,
  pluginId: PluginId,
) {
  if (!visibleTools) {
    return undefined;
  }

  if (typeof visibleTools[pluginId] === "boolean") {
    return visibleTools[pluginId];
  }

  for (const legacyId of legacyPluginIds(pluginId)) {
    if (typeof visibleTools[legacyId] === "boolean") {
      return visibleTools[legacyId];
    }
  }

  return undefined;
}

function legacyPluginIds(pluginId: PluginId) {
  const canonicalId = canonicalFirstPartyPluginId(pluginId);
  const legacyIds = [...legacyFirstPartyPluginIds(canonicalId)];

  if (canonicalId === FIRST_PARTY_PLUGIN_IDS.snap) {
    legacyIds.push("screenshot");
  }

  if (canonicalId === FIRST_PARTY_PLUGIN_IDS.awake) {
    legacyIds.push("caffeine");
  }

  if (canonicalId === FIRST_PARTY_PLUGIN_IDS.paper) {
    legacyIds.push("bing-wallpaper");
  }

  if (canonicalId === FIRST_PARTY_PLUGIN_IDS.launch) {
    legacyIds.push("quick-launcher");
  }

  return legacyIds;
}

export const DEFAULT_PREFERENCES_DESTINATION = "general" as const;

export type StaticPreferencesDestinationId =
  | "general"
  | "status-bar"
  | "shortcuts"
  | "tools"
  | "extensions";

export type ToolPreferencesDestinationId = `tool:${string}`;
export type PreferencesDestinationId =
  | StaticPreferencesDestinationId
  | ToolPreferencesDestinationId;

export type PreferencesNavigationGroup = "zero" | "tools" | "extensions";

export interface PreferencesToolDescriptor {
  id: string;
  title: string;
  subtitle?: string;
}

export interface PreferencesDestination {
  id: PreferencesDestinationId;
  group: PreferencesNavigationGroup;
  title: string;
  description: string;
  toolId?: string;
}

export interface PreferencesSettingDescriptor {
  id: string;
  destinationId: PreferencesDestinationId;
  title: string;
  description: string;
  path: string;
  keywords: string[];
  focusTargetId: string;
}

export type PreferencesTranslate = (key: string) => string;

interface PreferencesSettingIndexInput {
  destinations: PreferencesDestination[];
  tools: PreferencesToolDescriptor[];
  t: PreferencesTranslate;
}

const STATIC_PREFERENCES_SETTING_DEFINITIONS = [
  setting("general.open-at-login", "general", "prefs.launchAtLogin.title", "prefs.launchAtLogin.description"),
  setting("general.language", "general", "prefs.language.title", "prefs.language.description"),
  setting("status-bar.enabled", "status-bar", "statusBar.enabled.title", "statusBar.enabled.description"),
  setting("status-bar.launch", "status-bar", "statusBar.launch.title", "statusBar.launch.description"),
  setting("status-bar.collapsed", "status-bar", "statusBar.collapsed.title", "statusBar.collapsed.description"),
  setting("status-bar.items", "status-bar", "statusBar.items.title", "statusBar.items.description"),
  setting("shortcuts.snap", "shortcuts", "prefs.shortcuts.snap", "prefs.shortcuts.readOnly"),
  setting("shortcuts.launch", "shortcuts", "prefs.shortcuts.launch", "prefs.shortcuts.readOnly"),
  setting("tools.overview", "tools", "prefs.tools.overview", "prefs.tools.description"),
  setting("extensions.market", "extensions", "extensions.market.title", "extensions.market.description"),
  setting("extensions.local", "extensions", "extensions.local.title", "extensions.local.description"),
  setting("extensions.installed", "extensions", "extensions.installed.title", "extensions.installed.description"),
  setting("extensions.restore", "extensions", "extensions.restore.title", "extensions.restore.description"),
  setting("extensions.diagnostics", "extensions", "extensions.diagnostics.title", "extensions.diagnostics.description"),
] as const;

export function toolPreferencesDestinationId(
  pluginId: string,
): ToolPreferencesDestinationId {
  return `tool:${pluginId}`;
}

export function pluginIdFromPreferencesDestination(
  destinationId: PreferencesDestinationId,
): string | null {
  return destinationId.startsWith("tool:")
    ? destinationId.slice("tool:".length)
    : null;
}

export function createPreferencesDestinations(
  tools: PreferencesToolDescriptor[],
  t: PreferencesTranslate,
): PreferencesDestination[] {
  return [
    destination("general", "zero", t("prefs.nav.general"), t("prefs.general.description")),
    destination(
      "status-bar",
      "zero",
      t("prefs.nav.statusBar"),
      t("prefs.statusBar.description"),
    ),
    destination(
      "shortcuts",
      "zero",
      t("prefs.nav.shortcuts"),
      t("prefs.shortcuts.description"),
    ),
    destination("tools", "tools", t("prefs.nav.tools"), t("prefs.tools.description")),
    ...tools.map<PreferencesDestination>((tool) => ({
      id: toolPreferencesDestinationId(tool.id),
      group: "tools",
      title: tool.title,
      description: tool.subtitle ?? t("prefs.tool.description"),
      toolId: tool.id,
    })),
    destination(
      "extensions",
      "extensions",
      t("prefs.nav.extensions"),
      t("prefs.extensions.description"),
    ),
  ];
}

export function resolvePreferencesDestination(
  requested: PreferencesDestinationId | null | undefined,
  destinations: PreferencesDestination[],
): PreferencesDestinationId {
  if (requested && destinations.some((candidate) => candidate.id === requested)) {
    return requested;
  }

  return destinations.find((candidate) => candidate.id === DEFAULT_PREFERENCES_DESTINATION)
    ?.id ?? destinations[0]?.id ?? DEFAULT_PREFERENCES_DESTINATION;
}

export function createPreferencesSettingIndex({
  destinations,
  tools,
  t,
}: PreferencesSettingIndexInput): PreferencesSettingDescriptor[] {
  const staticSettings = STATIC_PREFERENCES_SETTING_DEFINITIONS.map((entry) =>
    localizeSetting(entry, destinations, t),
  );

  const toolSettings = tools.flatMap((tool) => {
    const destinationId = toolPreferencesDestinationId(tool.id);
    return [
      setting(`tool.${tool.id}.enabled`, destinationId, "prefs.tool.enabled.title", "prefs.tool.enabled.description"),
      setting(`tool.${tool.id}.navigation`, destinationId, "prefs.tool.navigation.title", "prefs.tool.navigation.description"),
      setting(`tool.${tool.id}.status-bar`, destinationId, "prefs.tool.statusBar.title", "prefs.tool.statusBar.description"),
      setting(`tool.${tool.id}.shortcut`, destinationId, "prefs.tool.shortcut.title", "prefs.tool.shortcut.description"),
    ].map((entry) =>
      localizeSetting(entry, destinations, t, [tool.title, tool.subtitle ?? ""]),
    );
  });

  return [...staticSettings, ...toolSettings];
}

export function createRenderedPreferencesSettingIds(
  tools: PreferencesToolDescriptor[],
): string[] {
  return [
    ...STATIC_PREFERENCES_SETTING_DEFINITIONS.map((entry) => entry.id),
    ...tools.flatMap((tool) => [
      `tool.${tool.id}.enabled`,
      `tool.${tool.id}.navigation`,
      `tool.${tool.id}.status-bar`,
      `tool.${tool.id}.shortcut`,
    ]),
  ];
}

export function filterPreferencesSettings(
  settings: PreferencesSettingDescriptor[],
  query: string,
): PreferencesSettingDescriptor[] {
  const normalized = normalizeSearchText(query);
  if (!normalized) {
    return [];
  }

  return settings.filter((settingDescriptor) =>
    [
      settingDescriptor.title,
      settingDescriptor.description,
      settingDescriptor.path,
      ...settingDescriptor.keywords,
    ].some((value) => normalizeSearchText(value).includes(normalized)),
  );
}

export function preferencesSettingFocusTargetId(settingId: string): string {
  return `preference-setting-${settingId.replace(/[^a-zA-Z0-9_-]+/g, "-")}`;
}

export function shouldClearPreferencesSearch(key: string): boolean {
  return key === "Escape";
}

function destination(
  id: StaticPreferencesDestinationId,
  group: PreferencesNavigationGroup,
  title: string,
  description: string,
): PreferencesDestination {
  return { id, group, title, description };
}

interface RawSettingDescriptor {
  id: string;
  destinationId: PreferencesDestinationId;
  titleKey: string;
  descriptionKey: string;
}

function setting(
  id: string,
  destinationId: PreferencesDestinationId,
  titleKey: string,
  descriptionKey: string,
): RawSettingDescriptor {
  return { id, destinationId, titleKey, descriptionKey };
}

function localizeSetting(
  raw: RawSettingDescriptor,
  destinations: PreferencesDestination[],
  t: PreferencesTranslate,
  extraKeywords: string[] = [],
): PreferencesSettingDescriptor {
  const destinationTitle =
    destinations.find((candidate) => candidate.id === raw.destinationId)?.title ??
    raw.destinationId;
  const title = t(raw.titleKey);
  const description = t(raw.descriptionKey);

  return {
    id: raw.id,
    destinationId: raw.destinationId,
    title,
    description,
    path: destinationTitle,
    keywords: [destinationTitle, title, description, ...extraKeywords].filter(Boolean),
    focusTargetId: preferencesSettingFocusTargetId(raw.id),
  };
}

function normalizeSearchText(value: string): string {
  return value.trim().toLocaleLowerCase();
}

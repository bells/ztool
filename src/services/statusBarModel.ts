import type {
  PluginContributionStatusBarItem,
  PluginRecord,
  PluginSource,
  StatusBarAction,
  StatusBarIconId,
} from "../core/pluginHost/contracts";
import {
  FIRST_PARTY_PLUGIN_IDS,
  PRODUCT_NAME,
  canonicalFirstPartyPluginId,
} from "../brand/identity.js";

export interface StatusBarSettingsSnapshot {
  enabled: boolean;
  showPluginItemsOnLaunch: boolean;
  pluginItemsCollapsed: boolean;
  visiblePluginItems: Record<string, boolean>;
}

export interface UpdateStatusBarSettingsInput {
  enabled?: boolean;
  showPluginItemsOnLaunch?: boolean;
  pluginItemsCollapsed?: boolean;
  visiblePluginItems?: Record<string, boolean>;
}

export interface StatusBarItemSnapshot {
  id: string;
  pluginName: string | null;
  title: string;
  icon: StatusBarIconId;
  baseIcon: StatusBarIconId;
  activeIcon?: StatusBarIconId;
  action: StatusBarAction;
  order: number;
  nativeVisible: boolean;
  source?: PluginSource;
}

export interface StatusBarPreferenceItem {
  id: string;
  pluginName: string;
  title: string;
  icon: StatusBarIconId;
  baseIcon: StatusBarIconId;
  activeIcon?: StatusBarIconId;
  action: StatusBarAction;
  order: number;
  visible: boolean;
  disabled: boolean;
  source?: PluginSource;
}

export interface ResolveStatusBarItemsInput {
  records: PluginRecord[];
  settings: StatusBarSettingsSnapshot;
  caffeineEnabled: boolean;
  platformSupportsNativeMultiItem: boolean;
}

export interface ResolveStatusBarPreferenceItemsInput {
  records: PluginRecord[];
  settings: StatusBarSettingsSnapshot;
  caffeineEnabled?: boolean;
}

export const DEFAULT_STATUS_BAR_SETTINGS: StatusBarSettingsSnapshot = {
  enabled: true,
  showPluginItemsOnLaunch: true,
  pluginItemsCollapsed: false,
  visiblePluginItems: {},
};

const PRIMARY_STATUS_BAR_ITEM: StatusBarItemSnapshot = {
  id: "zero.primary",
  pluginName: null,
  title: PRODUCT_NAME,
  icon: "zero",
  baseIcon: "zero",
  action: { type: "toggle-tray" },
  order: 0,
  nativeVisible: true,
};

export function normalizeStatusBarSettings(
  value: unknown,
  records: PluginRecord[],
): StatusBarSettingsSnapshot {
  const input = isStatusBarSettingsShape(value) ?? {};
  const visiblePluginItems = records.reduce<Record<string, boolean>>(
    (result, record) => ({
      ...result,
      [record.name]: readPluginItemVisibility(input.visiblePluginItems, record) ?? true,
    }),
    {},
  );

  return {
    enabled: input.enabled ?? DEFAULT_STATUS_BAR_SETTINGS.enabled,
    showPluginItemsOnLaunch:
      input.showPluginItemsOnLaunch ??
      DEFAULT_STATUS_BAR_SETTINGS.showPluginItemsOnLaunch,
    pluginItemsCollapsed:
      input.pluginItemsCollapsed ?? DEFAULT_STATUS_BAR_SETTINGS.pluginItemsCollapsed,
    visiblePluginItems,
  };
}

export function resolveStatusBarItems({
  records,
  settings,
  caffeineEnabled,
  platformSupportsNativeMultiItem,
}: ResolveStatusBarItemsInput): StatusBarItemSnapshot[] {
  if (!settings.enabled || !settings.showPluginItemsOnLaunch) {
    return [PRIMARY_STATUS_BAR_ITEM];
  }

  const pluginItems = records
    .map((record, pluginIndex) =>
      resolvePluginStatusBarItems({
        record,
        pluginIndex,
        settings,
        caffeineEnabled,
        platformSupportsNativeMultiItem,
      }),
    )
    .flat()
    .sort((left, right) => left.order - right.order);

  return [PRIMARY_STATUS_BAR_ITEM, ...pluginItems];
}

export function createStatusBarPreview(
  items: StatusBarItemSnapshot[],
): StatusBarItemSnapshot[] {
  return items;
}

export function getStatusBarFallbackItems(
  items: StatusBarItemSnapshot[],
): StatusBarItemSnapshot[] {
  return items.filter((item) => item.pluginName !== null && !item.nativeVisible);
}

export function resolveStatusBarPreferenceItems({
  records,
  settings,
  caffeineEnabled = false,
}: ResolveStatusBarPreferenceItemsInput): StatusBarPreferenceItem[] {
  return records
    .map((record, pluginIndex) =>
      resolvePluginStatusBarPreferenceItems({
        record,
        pluginIndex,
        settings,
        caffeineEnabled,
      }),
    )
    .flat()
    .sort((left, right) => left.order - right.order);
}

function resolvePluginStatusBarItems({
  record,
  pluginIndex,
  settings,
  caffeineEnabled,
  platformSupportsNativeMultiItem,
}: {
  record: PluginRecord;
  pluginIndex: number;
  settings: StatusBarSettingsSnapshot;
  caffeineEnabled: boolean;
  platformSupportsNativeMultiItem: boolean;
}): StatusBarItemSnapshot[] {
  if (!isStatusBarPluginAvailable(record) || !settings.visiblePluginItems[record.name]) {
    return [];
  }

  return statusBarContributions(record)
    .filter(isSupportedStatusBarContribution)
    .filter((item) => item.visibleByDefault !== false)
    .map((item) => {
      const icon = resolveStatusBarIcon(item, record, caffeineEnabled);

      return {
        id: item.id,
        pluginName: record.name,
        title: item.title,
        icon,
        baseIcon: item.icon,
        activeIcon: item.activeIcon,
        action: item.action,
        order: normalizeOrder(item.order, pluginIndex),
        nativeVisible: platformSupportsNativeMultiItem,
        source: record.source,
      };
    });
}

function resolvePluginStatusBarPreferenceItems({
  record,
  pluginIndex,
  settings,
  caffeineEnabled,
}: {
  record: PluginRecord;
  pluginIndex: number;
  settings: StatusBarSettingsSnapshot;
  caffeineEnabled: boolean;
}): StatusBarPreferenceItem[] {
  if (!isStatusBarPluginAvailable(record)) {
    return [];
  }

  const visible = settings.visiblePluginItems[record.name] ?? true;
  const disabled = !settings.enabled || !settings.showPluginItemsOnLaunch;

  return statusBarContributions(record)
    .filter(isSupportedStatusBarContribution)
    .filter((item) => item.visibleByDefault !== false)
    .map((item) => ({
      id: item.id,
      pluginName: record.name,
      title: item.title,
      icon: resolveStatusBarIcon(item, record, caffeineEnabled),
      baseIcon: item.icon,
      activeIcon: item.activeIcon,
      action: item.action,
      order: normalizeOrder(item.order, pluginIndex),
      visible,
      disabled,
      source: record.source,
    }));
}

function isStatusBarPluginAvailable(record: PluginRecord) {
  return (
    record.enabled &&
    record.health !== "disabled" &&
    record.health !== "incompatible"
  );
}

function statusBarContributions(record: PluginRecord): PluginContributionStatusBarItem[] {
  return record.manifest.contributes?.statusBarItems ?? [
    defaultPluginStatusBarItem(record),
  ];
}

function resolveStatusBarIcon(
  item: PluginContributionStatusBarItem,
  record: PluginRecord,
  caffeineEnabled: boolean,
): StatusBarIconId {
  if (
    canonicalFirstPartyPluginId(record.name) === FIRST_PARTY_PLUGIN_IDS.awake &&
    caffeineEnabled &&
    item.activeIcon
  ) {
    return item.activeIcon;
  }

  return item.icon;
}

function defaultPluginStatusBarItem(record: PluginRecord): PluginContributionStatusBarItem {
  return {
    id: `${record.name}.status`,
    title: record.manifest.displayName ?? record.name,
    icon: "extension",
    action: { type: "open-plugin" },
    order: 1000,
    visibleByDefault: true,
  };
}

function normalizeOrder(order: number | undefined, pluginIndex: number) {
  return (order ?? 1000) * 1000 + pluginIndex;
}

function isSupportedStatusBarContribution(item: PluginContributionStatusBarItem) {
  return (
    item.action.type === "toggle-tray" ||
    item.action.type === "toggle-caffeine" ||
    item.action.type === "start-screenshot" ||
    item.action.type === "open-plugin"
  );
}

function isStatusBarSettingsShape(
  value: unknown,
): Partial<StatusBarSettingsSnapshot> | null {
  if (!value || typeof value !== "object") {
    return null;
  }

  return value as Partial<StatusBarSettingsSnapshot>;
}

function readPluginItemVisibility(
  visiblePluginItems: Record<string, boolean> | undefined,
  record: PluginRecord,
) {
  if (!visiblePluginItems) {
    return undefined;
  }

  if (typeof visiblePluginItems[record.name] === "boolean") {
    return visiblePluginItems[record.name];
  }

  const manifestId = record.manifest.id;
  if (manifestId && typeof visiblePluginItems[manifestId] === "boolean") {
    return visiblePluginItems[manifestId];
  }

  for (const [storedPluginId, visible] of Object.entries(visiblePluginItems)) {
    if (
      canonicalFirstPartyPluginId(storedPluginId) ===
      canonicalFirstPartyPluginId(record.name)
    ) {
      return visible;
    }
  }

  return undefined;
}

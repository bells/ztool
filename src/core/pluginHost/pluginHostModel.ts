import type {
  PluginHealth,
  PluginRecord,
  PluginSource,
} from "./contracts";
import { canonicalFirstPartyPluginId } from "../../brand/identity.js";

export interface PluginNavigationItem {
  id: string;
  title: string;
  subtitle: string;
  health: PluginHealth;
  enabled: boolean;
  source: PluginSource;
}

export interface PluginHostSummary {
  total: number;
  bundled: number;
  market: number;
  local: number;
  development: number;
  enabled: number;
  disabled: number;
  failed: number;
  incompatible: number;
}

export interface PluginHostState {
  records: PluginRecord[];
  selectedPluginName: string | null;
  activePlugin: PluginRecord | undefined;
  navigationItems: PluginNavigationItem[];
  summary: PluginHostSummary;
  isLoading: boolean;
  isBusy: boolean;
  error: string | null;
}

export function createPluginHostState(
  records: PluginRecord[],
  selectedPluginName?: string | null,
): PluginHostState {
  const activePlugin = selectActivePlugin(records, selectedPluginName);

  return {
    records,
    selectedPluginName: activePlugin?.name ?? selectedPluginName ?? null,
    activePlugin,
    navigationItems: toPluginNavigationItems(records),
    summary: summarizePluginRecords(records),
    isLoading: false,
    isBusy: false,
    error: null,
  };
}

export function selectActivePlugin(
  records: PluginRecord[],
  selectedPluginName?: string | null,
): PluginRecord | undefined {
  const canonicalSelectedName = selectedPluginName
    ? canonicalFirstPartyPluginId(selectedPluginName)
    : selectedPluginName;
  const selected = records.find(
    (record) =>
      canonicalFirstPartyPluginId(record.name) === canonicalSelectedName &&
      isPluginNavigable(record),
  );

  return selected ?? records.find(isPluginNavigable);
}

export function toPluginNavigationItems(
  records: PluginRecord[],
): PluginNavigationItem[] {
  return records.filter(isPluginNavigable).map((record) => ({
    id: record.name,
    title: pluginTitle(record),
    subtitle: `${record.source} · ${record.version}`,
    health: record.health,
    enabled: record.enabled,
    source: record.source,
  }));
}

export function summarizePluginRecords(
  records: PluginRecord[],
): PluginHostSummary {
  return records.reduce<PluginHostSummary>(
    (summary, record) => ({
      total: summary.total + 1,
      bundled: summary.bundled + countSource(record, "bundled"),
      market: summary.market + countSource(record, "market"),
      local: summary.local + countSource(record, "local"),
      development: summary.development + countSource(record, "development"),
      enabled: summary.enabled + (record.enabled ? 1 : 0),
      disabled: summary.disabled + (!record.enabled ? 1 : 0),
      failed: summary.failed + (record.health === "error" ? 1 : 0),
      incompatible:
        summary.incompatible + (record.health === "incompatible" ? 1 : 0),
    }),
    {
      total: 0,
      bundled: 0,
      market: 0,
      local: 0,
      development: 0,
      enabled: 0,
      disabled: 0,
      failed: 0,
      incompatible: 0,
    },
  );
}

export function pluginHostActionStarted(state: PluginHostState): PluginHostState {
  return {
    ...state,
    isBusy: true,
    error: null,
  };
}

export function pluginHostActionFailed(
  state: PluginHostState,
  error: unknown,
): PluginHostState {
  return {
    ...state,
    isLoading: false,
    isBusy: false,
    error: error instanceof Error ? error.message : String(error),
  };
}

function isPluginNavigable(record: PluginRecord) {
  return record.enabled &&
    record.health !== "disabled" &&
    record.health !== "incompatible";
}

function pluginTitle(record: PluginRecord) {
  return record.manifest.displayName ?? record.name;
}

function countSource(record: PluginRecord, source: PluginSource) {
  return record.source === source ? 1 : 0;
}

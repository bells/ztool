import type {
  PluginMarketEntry,
  PluginMarketSnapshot,
  PluginPermission,
} from "./contracts";

export type PluginChecksumStatus = "verified" | "unsigned";

export interface PluginInstallCard {
  name: string;
  title: string;
  version: string;
  author: string;
  description?: string;
  permissions: PluginPermission[];
  installedVersion?: string;
  isInstalled: boolean;
  checksumStatus: PluginChecksumStatus;
  releaseUrl: string;
}

export interface PluginMarketState {
  entries: PluginMarketEntry[];
  installCards: PluginInstallCard[];
  isLoading: boolean;
  error: string | null;
  stale: boolean;
  sourceUrl?: string;
  updatedAt?: string;
}

export const INITIAL_PLUGIN_MARKET_STATE: PluginMarketState = {
  entries: [],
  installCards: [],
  isLoading: false,
  error: null,
  stale: false,
};

export function toPluginInstallCards(
  entries: PluginMarketEntry[],
): PluginInstallCard[] {
  return entries.map((entry) => ({
    name: entry.name,
    title: entry.name,
    version: entry.version,
    author: entry.author,
    description: entry.description,
    permissions: entry.permissions,
    installedVersion: entry.installedVersion,
    isInstalled: Boolean(entry.installedVersion),
    checksumStatus: entry.sha256 ? "verified" : "unsigned",
    releaseUrl: entry.releaseUrl,
  }));
}

export function marketRefreshStarted(
  state: PluginMarketState,
): PluginMarketState {
  return {
    ...state,
    isLoading: true,
    error: null,
  };
}

export function marketRefreshSucceeded(
  snapshot: PluginMarketSnapshot,
): PluginMarketState {
  return {
    entries: snapshot.entries,
    installCards: toPluginInstallCards(snapshot.entries),
    isLoading: false,
    error: null,
    stale: snapshot.stale,
    sourceUrl: snapshot.sourceUrl,
    updatedAt: snapshot.updatedAt,
  };
}

export function marketRefreshFailed(
  state: PluginMarketState,
  error: unknown,
): PluginMarketState {
  return {
    ...state,
    isLoading: false,
    error: errorMessage(error),
    stale: true,
  };
}

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

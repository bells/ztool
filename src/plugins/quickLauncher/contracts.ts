export type QuickLauncherItemKind = "application" | "systemSetting";

export type QuickLauncherIndexSource = "empty" | "cache" | "scan";

export type QuickLauncherPlatformSupport = "supported" | "degraded" | "unsupported";

export type QuickLauncherRunningState =
  | "running"
  | "notRunning"
  | "unknown"
  | "notApplicable";

export interface QuickLauncherDiagnostic {
  code: string;
  message: string;
}

export interface QuickLauncherIndexSnapshot {
  revision: number;
  source: QuickLauncherIndexSource;
  refreshing: boolean;
  itemCount: number;
  lastUpdatedAt?: number;
  platformSupport: QuickLauncherPlatformSupport;
  diagnostics: QuickLauncherDiagnostic[];
}

export type {
  QuickLauncherActivateInput,
  QuickLauncherSearchInput,
} from "../../core/pluginHost/launcherContracts";

export interface QuickLauncherResultItem {
  id: string;
  kind: QuickLauncherItemKind;
  title: string;
  subtitle: string;
  running: QuickLauncherRunningState;
  iconKey?: string;
  matchedField: string;
}

export interface QuickLauncherSearchResult {
  revision: number;
  query: string;
  elapsedMicros: number;
  items: QuickLauncherResultItem[];
}

export type QuickLauncherActivationAction =
  | "focused"
  | "launched"
  | "launchedFallback"
  | "openedSetting";

export interface QuickLauncherActivationResult {
  itemId: string;
  action: QuickLauncherActivationAction;
  usageCount: number;
  activatedAt: number;
}

export interface QuickLauncherIconInput {
  itemId: string;
  iconKey?: string;
}

export interface QuickLauncherIconResult {
  itemId: string;
  dataUrl?: string;
}

export interface QuickLauncherError {
  operation: string;
  code: string;
  message: string;
  retryable: boolean;
}

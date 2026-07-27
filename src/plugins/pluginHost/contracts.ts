export const SUPPORTED_EXTENSION_API_VERSION = "1";
export const SUPPORTED_ZERO_HOST_VERSION = "0.1.0";

export const SUPPORTED_PLUGIN_PERMISSIONS = [
  "clipboard.read",
  "clipboard.write",
  "network",
  "storage.plugin",
  "ui.message",
  "process.execute",
  "system.wallpaper",
  "system.apps.read",
  "system.apps.execute",
  "system.window.focus",
  "system.settings.open",
] as const;

export type PluginPermission = (typeof SUPPORTED_PLUGIN_PERMISSIONS)[number];

export type PluginRuntime = "webview" | "script" | "binary";

export type PluginSource = "bundled" | "market" | "local" | "development";

export type PluginHealth =
  | "ready"
  | "active"
  | "disabled"
  | "error"
  | "incompatible";

export interface PluginEngines {
  zero?: string;
  ztool?: string;
  api?: string;
}

export interface PluginContributionView {
  id: string;
  title: string;
  surface?: "tray" | "main" | "preferences" | "about";
}

export interface PluginContributionCommand {
  id: string;
  title: string;
}

export interface PluginContributionSetting {
  key: string;
  type: "boolean" | "string" | "number";
  default: boolean | string | number;
  label?: string;
}

export type StatusBarIconId =
  | "zero"
  | "launch"
  | "caffeine-empty"
  | "caffeine-full"
  | "screenshot"
  | "paper"
  | "extension";

export type StatusBarActionType =
  | "toggle-tray"
  | "toggle-caffeine"
  | "start-screenshot"
  | "open-plugin";

export interface StatusBarAction {
  type: StatusBarActionType;
  commandId?: string;
}

export interface PluginContributionStatusBarItem {
  id: string;
  title: string;
  icon: StatusBarIconId;
  activeIcon?: StatusBarIconId;
  action: StatusBarAction;
  order?: number;
  visibleByDefault?: boolean;
}

export interface PluginContributions {
  views?: PluginContributionView[];
  commands?: PluginContributionCommand[];
  settings?: PluginContributionSetting[];
  statusBarItems?: PluginContributionStatusBarItem[];
}

export interface PluginManifest {
  name: string;
  version: string;
  author: string;
  main: string;
  permissions: PluginPermission[];
  id?: string;
  displayName?: string;
  description?: string;
  engines?: PluginEngines;
  platforms?: Array<"macos" | "windows" | "linux">;
  runtime?: PluginRuntime;
  contributes?: PluginContributions;
}

export interface PluginMarketEntry {
  name: string;
  version: string;
  author: string;
  repository: string;
  releaseUrl: string;
  downloadUrl: string;
  permissions: PluginPermission[];
  description?: string;
  sha256?: string;
  installedVersion?: string;
}

export interface PluginMarketIndex {
  schemaVersion: 1;
  plugins: PluginMarketEntry[];
  updatedAt?: string;
}

export interface PluginMarketSnapshot {
  sourceUrl: string;
  schemaVersion: 1;
  updatedAt?: string;
  entries: PluginMarketEntry[];
  stale: boolean;
}

export interface PluginValidationIssue {
  code: string;
  path: string;
  message: string;
}

export interface PluginManifestValidationReport {
  valid: boolean;
  issues: PluginValidationIssue[];
  manifest?: PluginManifest;
}

export interface PluginPackageValidationReport {
  valid: boolean;
  issues: PluginValidationIssue[];
  manifest?: PluginManifest;
  packagePath: string;
  sha256: string;
}

export interface PluginMarketValidationReport {
  valid: boolean;
  issues: PluginValidationIssue[];
  market?: PluginMarketIndex;
}

export interface PluginRecord {
  name: string;
  version: string;
  author: string;
  source: PluginSource;
  enabled: boolean;
  health: PluginHealth;
  manifest: PluginManifest;
  installedPath?: string;
  approvedPermissions: PluginPermission[];
  packageSha256?: string;
}

export interface PluginLifecycleResult {
  plugin?: PluginRecord;
  message: string;
}

export interface PluginHostApiError {
  code: string;
  message: string;
  pluginName?: string;
}

export interface ValidatePluginPackageInput {
  packagePath: string;
}

export interface InstallPluginPackageInput {
  packagePath: string;
  approvedPermissions: PluginPermission[];
  enabled?: boolean;
}

export interface InstallMarketPluginInput {
  entry: PluginMarketEntry;
  approvedPermissions: PluginPermission[];
  enabled?: boolean;
}

export interface PluginIdentityInput {
  name: string;
}

export interface SetPluginEnabledInput {
  name: string;
  enabled: boolean;
}

export interface NetworkFetchRequest {
  url: string;
  method?: "GET";
}

export interface NetworkFetchResponse {
  status: number;
  contentType?: string;
  bodyBase64: string;
}

export interface StorageWriteFileRequest {
  relativePath: string;
  dataBase64: string;
}

export interface StorageWriteFileResult {
  relativePath: string;
  bytesWritten: number;
}

export interface SystemSetWallpaperRequest {
  relativePath: string;
}

export interface NativeResourceError {
  operation: string;
  code: string;
  message: string;
  retryable: boolean;
}

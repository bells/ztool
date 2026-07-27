use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginPermission {
    #[serde(rename = "clipboard.read")]
    ClipboardRead,
    #[serde(rename = "clipboard.write")]
    ClipboardWrite,
    #[serde(rename = "network")]
    Network,
    #[serde(rename = "storage.plugin")]
    StoragePlugin,
    #[serde(rename = "ui.message")]
    UiMessage,
    #[serde(rename = "process.execute")]
    ProcessExecute,
    #[serde(rename = "system.wallpaper")]
    SystemWallpaper,
    #[serde(rename = "system.apps.read")]
    SystemAppsRead,
    #[serde(rename = "system.apps.execute")]
    SystemAppsExecute,
    #[serde(rename = "system.window.focus")]
    SystemWindowFocus,
    #[serde(rename = "system.settings.open")]
    SystemSettingsOpen,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PluginRuntime {
    Webview,
    Script,
    Binary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PluginSource {
    Bundled,
    Market,
    Local,
    Development,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PluginHealth {
    Ready,
    Active,
    Disabled,
    Error,
    Incompatible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginEngines {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zero: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ztool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PluginViewSurface {
    Tray,
    Main,
    Preferences,
    About,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginContributionView {
    pub id: String,
    pub title: String,
    pub surface: Option<PluginViewSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginContributionCommand {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginSettingDefault {
    Boolean(bool),
    String(String),
    Number(f64),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PluginSettingType {
    Boolean,
    String,
    Number,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginContributionSetting {
    pub key: String,
    #[serde(rename = "type")]
    pub setting_type: PluginSettingType,
    pub default: PluginSettingDefault,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StatusBarIconId {
    Zero,
    Launch,
    CaffeineEmpty,
    CaffeineFull,
    Screenshot,
    Paper,
    Extension,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StatusBarActionType {
    ToggleTray,
    ToggleCaffeine,
    StartScreenshot,
    OpenPlugin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusBarAction {
    #[serde(rename = "type")]
    pub action_type: StatusBarActionType,
    pub command_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginContributionStatusBarItem {
    pub id: String,
    pub title: String,
    pub icon: StatusBarIconId,
    pub active_icon: Option<StatusBarIconId>,
    pub action: StatusBarAction,
    pub order: Option<u32>,
    pub visible_by_default: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginContributions {
    pub views: Option<Vec<PluginContributionView>>,
    pub commands: Option<Vec<PluginContributionCommand>>,
    pub settings: Option<Vec<PluginContributionSetting>>,
    #[serde(rename = "statusBarItems")]
    pub status_bar_items: Option<Vec<PluginContributionStatusBarItem>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PluginPlatform {
    Macos,
    Windows,
    Linux,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub author: String,
    pub main: String,
    pub permissions: Vec<PluginPermission>,
    pub id: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub engines: Option<PluginEngines>,
    pub platforms: Option<Vec<PluginPlatform>>,
    pub runtime: Option<PluginRuntime>,
    pub contributes: Option<PluginContributions>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginMarketEntry {
    pub name: String,
    pub version: String,
    pub author: String,
    pub repository: String,
    pub release_url: String,
    pub download_url: String,
    pub permissions: Vec<PluginPermission>,
    pub description: Option<String>,
    pub sha256: Option<String>,
    pub installed_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginMarketIndex {
    pub schema_version: u16,
    pub plugins: Vec<PluginMarketEntry>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginValidationIssue {
    pub code: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginManifestValidationReport {
    pub valid: bool,
    pub issues: Vec<PluginValidationIssue>,
    pub manifest: Option<PluginManifest>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginPackageValidationReport {
    pub valid: bool,
    pub issues: Vec<PluginValidationIssue>,
    pub manifest: Option<PluginManifest>,
    pub package_path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginMarketValidationReport {
    pub valid: bool,
    pub issues: Vec<PluginValidationIssue>,
    pub market: Option<PluginMarketIndex>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRecord {
    pub name: String,
    pub version: String,
    pub author: String,
    pub source: PluginSource,
    pub enabled: bool,
    pub health: PluginHealth,
    pub manifest: PluginManifest,
    pub installed_path: Option<String>,
    pub approved_permissions: Vec<PluginPermission>,
    pub package_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginLifecycleResult {
    pub plugin: Option<PluginRecord>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginHostApiError {
    pub code: String,
    pub message: String,
    pub plugin_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatePluginPackageInput {
    pub package_path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallPluginPackageInput {
    pub package_path: String,
    pub approved_permissions: Vec<PluginPermission>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallMarketPluginInput {
    pub entry: PluginMarketEntry,
    pub approved_permissions: Vec<PluginPermission>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginIdentityInput {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPluginEnabledInput {
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkFetchRequest {
    pub url: String,
    pub method: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkFetchResponse {
    pub status: u16,
    pub content_type: Option<String>,
    pub body_base64: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageWriteFileRequest {
    pub relative_path: String,
    pub data_base64: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageWriteFileResult {
    pub relative_path: String,
    pub bytes_written: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSetWallpaperRequest {
    pub relative_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeResourceError {
    pub operation: String,
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

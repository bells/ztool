use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::brand::{
    canonical_data_root, canonical_first_party_contribution_id, canonical_first_party_plugin_id,
    default_home, ZERO_AWAKE_PLUGIN_ID, ZERO_FILE_PLUGIN_ID, ZERO_LAUNCH_PLUGIN_ID,
    ZERO_PAPER_PLUGIN_ID, ZERO_SNAP_PLUGIN_ID,
};

use super::contracts::{
    InstallPluginPackageInput, PluginContributionCommand, PluginContributionSetting,
    PluginContributionStatusBarItem, PluginContributionView, PluginContributions, PluginHealth,
    PluginLifecycleResult, PluginManifest, PluginMarketEntry, PluginPermission, PluginPlatform,
    PluginRecord, PluginRuntime, PluginSettingDefault, PluginSettingType, PluginSource,
    PluginViewSurface, StatusBarAction, StatusBarActionType, StatusBarIconId,
};
use super::package::{extract_zplugin_package, format_validation_issues, validate_zplugin_package};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PluginRegistryDiskState {
    #[serde(default)]
    schema_version: u16,
    records: Vec<PluginRecord>,
}

const PLUGIN_REGISTRY_SCHEMA_VERSION: u16 = 5;

pub struct PluginRegistry {
    root: PathBuf,
    records: Vec<PluginRecord>,
    diagnostics: Vec<String>,
}

pub struct PluginRegistryState {
    registry: Mutex<PluginRegistry>,
}

impl Default for PluginRegistryState {
    fn default() -> Self {
        Self {
            registry: Mutex::new(
                PluginRegistry::load_or_seed(PluginRegistry::default_root())
                    .expect("plugin registry should initialize"),
            ),
        }
    }
}

impl PluginRegistryState {
    pub fn with_registry<T>(
        &self,
        operation: impl FnOnce(&mut PluginRegistry) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| "plugin registry lock is poisoned".to_string())?;

        operation(&mut registry)
    }
}

impl PluginRegistry {
    pub fn load_or_seed(root: impl Into<PathBuf>) -> Result<Self, String> {
        let root = root.into();
        fs::create_dir_all(&root)
            .map_err(|error| format!("failed to create plugin root: {error}"))?;

        let registry_path = registry_path(&root);
        if !registry_path.exists() {
            return Ok(Self {
                root,
                records: bundled_plugin_records(),
                diagnostics: Vec::new(),
            });
        }

        match fs::read_to_string(&registry_path)
            .map_err(|error| format!("failed to read plugin registry: {error}"))
            .and_then(|content| {
                serde_json::from_str::<PluginRegistryDiskState>(&content)
                    .map_err(|error| format!("failed to parse plugin registry: {error}"))
            }) {
            Ok(mut state) => {
                canonicalize_records(&mut state.records);
                migrate_bundled_records(&mut state.records);
                state.schema_version = PLUGIN_REGISTRY_SCHEMA_VERSION;
                let registry = Self {
                    root,
                    records: state.records,
                    diagnostics: Vec::new(),
                };
                registry.save()?;
                Ok(registry)
            }
            Err(error) => Ok(Self {
                root,
                records: bundled_plugin_records(),
                diagnostics: vec![format!("registry recovery: {error}")],
            }),
        }
    }

    pub fn default_root() -> PathBuf {
        canonical_data_root(&default_home()).join("plugins")
    }

    pub fn records(&self) -> &[PluginRecord] {
        &self.records
    }

    pub fn diagnostics(&self) -> &[String] {
        &self.diagnostics
    }

    pub fn save(&self) -> Result<(), String> {
        let state = PluginRegistryDiskState {
            schema_version: PLUGIN_REGISTRY_SCHEMA_VERSION,
            records: self.records.clone(),
        };
        let content = serde_json::to_string_pretty(&state)
            .map_err(|error| format!("failed to serialize plugin registry: {error}"))?;

        fs::write(registry_path(&self.root), content)
            .map_err(|error| format!("failed to write plugin registry: {error}"))
    }

    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> Result<PluginRecord, String> {
        let record = self
            .records
            .iter_mut()
            .find(|record| record.name == name)
            .ok_or_else(|| format!("plugin {name} was not found"))?;

        record.enabled = enabled;
        record.health = if enabled {
            PluginHealth::Ready
        } else {
            PluginHealth::Disabled
        };

        Ok(record.clone())
    }

    pub fn validate_package(
        &self,
        package_path: impl AsRef<Path>,
    ) -> Result<super::contracts::PluginPackageValidationReport, String> {
        validate_zplugin_package(package_path.as_ref()).map_err(|error| error.message)
    }

    pub fn install_local_package(
        &mut self,
        input: InstallPluginPackageInput,
    ) -> Result<PluginRecord, String> {
        self.install_package(input, PluginSource::Local, None)
    }

    pub fn install_market_package_from_path(
        &mut self,
        entry: &PluginMarketEntry,
        input: InstallPluginPackageInput,
    ) -> Result<PluginRecord, String> {
        self.install_package(input, PluginSource::Market, Some(entry))
    }

    pub fn uninstall_plugin(&mut self, name: &str) -> Result<PluginLifecycleResult, String> {
        if is_protected_host_surface(name) {
            return Err(format!("{name} is a protected host surface"));
        }

        let index = self
            .records
            .iter()
            .position(|record| record.name == name)
            .ok_or_else(|| format!("plugin {name} was not found"))?;
        let record = self.records.remove(index);

        if record.source != PluginSource::Bundled {
            if let Some(installed_path) = &record.installed_path {
                let installed_path = PathBuf::from(installed_path);
                if installed_path.starts_with(&self.root) && installed_path.exists() {
                    fs::remove_dir_all(plugin_name_root(&self.root, &record.name))
                        .or_else(|_| fs::remove_dir_all(&installed_path))
                        .map_err(|error| {
                            format!("failed to remove plugin assets for {name}: {error}")
                        })?;
                }
            }
        }

        self.save()?;

        Ok(PluginLifecycleResult {
            plugin: Some(record),
            message: format!("plugin {name} uninstalled"),
        })
    }

    pub fn restore_bundled_defaults(&mut self) -> Result<Vec<PluginRecord>, String> {
        for bundled in bundled_plugin_records() {
            match self
                .records
                .iter_mut()
                .find(|record| record.name == bundled.name)
            {
                Some(record) if record.source == PluginSource::Bundled => {
                    record.enabled = true;
                    record.health = PluginHealth::Ready;
                }
                Some(_) => {}
                None => self.records.push(bundled),
            }
        }

        self.save()?;

        Ok(self.records.clone())
    }

    fn install_package(
        &mut self,
        input: InstallPluginPackageInput,
        source: PluginSource,
        market_entry: Option<&PluginMarketEntry>,
    ) -> Result<PluginRecord, String> {
        let package_path = PathBuf::from(&input.package_path);
        let report = validate_zplugin_package(&package_path).map_err(|error| error.message)?;
        if !report.valid {
            return Err(format_validation_issues(&report.issues));
        }

        let manifest = report
            .manifest
            .clone()
            .ok_or_else(|| "package manifest was not available after validation".to_string())?;

        if let Some(entry) = market_entry {
            validate_market_entry_matches_manifest(entry, &manifest)?;
        }

        if self
            .records
            .iter()
            .any(|record| record.name == manifest.name)
        {
            return Err(format!("plugin {} is already installed", manifest.name));
        }

        if !permissions_are_approved(&manifest.permissions, &input.approved_permissions) {
            return Err(format!(
                "permissions were not approved for plugin {}",
                manifest.name
            ));
        }

        let enabled = input.enabled.unwrap_or(true);
        let final_root = plugin_version_root(&self.root, &manifest.name, &manifest.version);
        if final_root.exists() {
            return Err(format!(
                "plugin {} version {} is already extracted",
                manifest.name, manifest.version
            ));
        }

        let staging_root = self.staging_root(&manifest.name, &manifest.version);
        if staging_root.exists() {
            fs::remove_dir_all(&staging_root).map_err(|error| {
                format!("failed to clear stale plugin staging directory: {error}")
            })?;
        }

        let install_result = (|| -> Result<(), String> {
            extract_zplugin_package(&package_path, &staging_root).map_err(|error| error.message)?;

            if let Some(parent) = final_root.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("failed to create plugin directory: {error}"))?;
            }

            fs::rename(&staging_root, &final_root)
                .map_err(|error| format!("failed to activate plugin install: {error}"))?;

            Ok(())
        })();

        if install_result.is_err() {
            let _ = fs::remove_dir_all(&staging_root);
            let _ = fs::remove_dir_all(&final_root);
        }
        install_result?;

        let record = PluginRecord {
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            author: manifest.author.clone(),
            source,
            enabled,
            health: if enabled {
                PluginHealth::Ready
            } else {
                PluginHealth::Disabled
            },
            manifest: manifest.clone(),
            installed_path: Some(final_root.to_string_lossy().into_owned()),
            approved_permissions: manifest.permissions.clone(),
            package_sha256: Some(report.sha256),
        };

        self.records.push(record.clone());
        if let Err(error) = self.save() {
            self.records.retain(|record| record.name != manifest.name);
            let _ = fs::remove_dir_all(plugin_name_root(&self.root, &manifest.name));
            return Err(error);
        }

        Ok(record)
    }

    fn staging_root(&self, name: &str, version: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();

        self.root
            .join(".installing")
            .join(format!("{name}-{version}-{nonce}"))
    }
}

fn registry_path(root: &Path) -> PathBuf {
    root.join("registry.json")
}

fn plugin_name_root(root: &Path, name: &str) -> PathBuf {
    root.join(name)
}

fn plugin_version_root(root: &Path, name: &str, version: &str) -> PathBuf {
    plugin_name_root(root, name).join(version)
}

fn bundled_plugin_records() -> Vec<PluginRecord> {
    vec![
        bundled_screenshot_record(),
        bundled_caffeine_record(),
        bundled_bing_wallpaper_record(),
        bundled_quick_launcher_record(),
        bundled_file_record(),
    ]
}

fn bundled_screenshot_record() -> PluginRecord {
    bundled_record(PluginManifest {
        name: ZERO_SNAP_PLUGIN_ID.into(),
        version: env!("CARGO_PKG_VERSION").into(),
        author: "watson".into(),
        main: "plugins/screenshot".into(),
        permissions: vec![PluginPermission::UiMessage],
        id: Some(ZERO_SNAP_PLUGIN_ID.into()),
        display_name: Some("Zero Snap".into()),
        description: Some("Shortcut, copy, save".into()),
        engines: None,
        platforms: Some(desktop_platforms()),
        runtime: Some(PluginRuntime::Webview),
        contributes: Some(PluginContributions {
            views: Some(vec![PluginContributionView {
                id: "zero.snap.main".into(),
                title: "Zero Snap".into(),
                surface: Some(PluginViewSurface::Main),
            }]),
            commands: Some(vec![
                PluginContributionCommand {
                    id: "zero.snap.capture".into(),
                    title: "Capture Zero Snap".into(),
                },
                PluginContributionCommand {
                    id: "zero.snap.copy".into(),
                    title: "Capture and Copy".into(),
                },
                PluginContributionCommand {
                    id: "zero.snap.save".into(),
                    title: "Capture and Save".into(),
                },
            ]),
            settings: None,
            status_bar_items: Some(vec![PluginContributionStatusBarItem {
                id: "zero.snap.status".into(),
                title: "Zero Snap".into(),
                icon: StatusBarIconId::Screenshot,
                active_icon: None,
                action: StatusBarAction {
                    action_type: StatusBarActionType::StartScreenshot,
                    command_id: Some("zero.snap.capture".into()),
                },
                order: Some(20),
                visible_by_default: Some(true),
            }]),
        }),
    })
}

fn bundled_caffeine_record() -> PluginRecord {
    bundled_record(PluginManifest {
        name: ZERO_AWAKE_PLUGIN_ID.into(),
        version: env!("CARGO_PKG_VERSION").into(),
        author: "watson".into(),
        main: "plugins/caffeine".into(),
        permissions: vec![PluginPermission::UiMessage],
        id: Some(ZERO_AWAKE_PLUGIN_ID.into()),
        display_name: Some("Zero Awake".into()),
        description: Some("Keep display and system awake".into()),
        engines: None,
        platforms: Some(desktop_platforms()),
        runtime: Some(PluginRuntime::Webview),
        contributes: Some(PluginContributions {
            views: Some(vec![PluginContributionView {
                id: "zero.awake.main".into(),
                title: "Zero Awake".into(),
                surface: Some(PluginViewSurface::Main),
            }]),
            commands: Some(vec![PluginContributionCommand {
                id: "zero.awake.toggle".into(),
                title: "Toggle Zero Awake".into(),
            }]),
            settings: Some(vec![PluginContributionSetting {
                key: "durationMinutes".into(),
                setting_type: PluginSettingType::Number,
                default: PluginSettingDefault::Number(0.0),
                label: Some("Duration minutes".into()),
            }]),
            status_bar_items: Some(vec![PluginContributionStatusBarItem {
                id: "zero.awake.status".into(),
                title: "Zero Awake".into(),
                icon: StatusBarIconId::CaffeineEmpty,
                active_icon: Some(StatusBarIconId::CaffeineFull),
                action: StatusBarAction {
                    action_type: StatusBarActionType::ToggleCaffeine,
                    command_id: Some("zero.awake.toggle".into()),
                },
                order: Some(10),
                visible_by_default: Some(true),
            }]),
        }),
    })
}

fn bundled_bing_wallpaper_record() -> PluginRecord {
    bundled_record(PluginManifest {
        name: ZERO_PAPER_PLUGIN_ID.into(),
        version: "1.0.0".into(),
        author: "bells".into(),
        main: "plugins/bingWallpaper".into(),
        permissions: vec![
            PluginPermission::Network,
            PluginPermission::StoragePlugin,
            PluginPermission::SystemWallpaper,
        ],
        id: Some(ZERO_PAPER_PLUGIN_ID.into()),
        display_name: Some("Zero Paper".into()),
        description: Some("Browse, download, and apply Bing daily wallpapers".into()),
        engines: None,
        platforms: Some(desktop_platforms()),
        runtime: Some(PluginRuntime::Webview),
        contributes: Some(PluginContributions {
            views: Some(vec![PluginContributionView {
                id: "zero.paper.main".into(),
                title: "Zero Paper".into(),
                surface: Some(PluginViewSurface::Main),
            }]),
            commands: Some(vec![
                PluginContributionCommand {
                    id: "zero.paper.refresh".into(),
                    title: "Refresh Bing wallpapers".into(),
                },
                PluginContributionCommand {
                    id: "zero.paper.apply".into(),
                    title: "Apply Bing wallpaper".into(),
                },
                PluginContributionCommand {
                    id: "zero.paper.download".into(),
                    title: "Download Bing wallpaper".into(),
                },
            ]),
            settings: None,
            status_bar_items: Some(vec![PluginContributionStatusBarItem {
                id: "zero.paper.status".into(),
                title: "Zero Paper".into(),
                icon: StatusBarIconId::Paper,
                active_icon: None,
                action: StatusBarAction {
                    action_type: StatusBarActionType::OpenPlugin,
                    command_id: None,
                },
                order: Some(30),
                visible_by_default: Some(true),
            }]),
        }),
    })
}

fn bundled_quick_launcher_record() -> PluginRecord {
    bundled_record(PluginManifest {
        name: ZERO_LAUNCH_PLUGIN_ID.into(),
        version: "1.0.0".into(),
        author: "bells".into(),
        main: "plugins/quickLauncher".into(),
        permissions: vec![
            PluginPermission::SystemAppsRead,
            PluginPermission::SystemAppsExecute,
            PluginPermission::SystemWindowFocus,
            PluginPermission::SystemSettingsOpen,
        ],
        id: Some(ZERO_LAUNCH_PLUGIN_ID.into()),
        display_name: Some("Zero Launch".into()),
        description: Some("Search, launch, and switch local apps and system settings".into()),
        engines: None,
        platforms: Some(vec![PluginPlatform::Macos, PluginPlatform::Windows]),
        runtime: Some(PluginRuntime::Webview),
        contributes: Some(PluginContributions {
            views: Some(vec![PluginContributionView {
                id: "zero.launch.main".into(),
                title: "Zero Launch".into(),
                surface: Some(PluginViewSurface::Main),
            }]),
            commands: Some(vec![
                PluginContributionCommand {
                    id: "zero.launch.show".into(),
                    title: "Show Zero Launch".into(),
                },
                PluginContributionCommand {
                    id: "zero.launch.refresh".into(),
                    title: "Refresh application index".into(),
                },
            ]),
            settings: None,
            status_bar_items: Some(vec![PluginContributionStatusBarItem {
                id: "zero.launch.status".into(),
                title: "Zero Launch".into(),
                icon: StatusBarIconId::Launch,
                active_icon: None,
                action: StatusBarAction {
                    action_type: StatusBarActionType::OpenPlugin,
                    command_id: None,
                },
                order: Some(40),
                visible_by_default: Some(true),
            }]),
        }),
    })
}

fn bundled_file_record() -> PluginRecord {
    bundled_record(PluginManifest {
        name: ZERO_FILE_PLUGIN_ID.into(),
        version: "1.0.0".into(),
        author: "bells".into(),
        main: "plugins/file".into(),
        permissions: Vec::new(),
        id: Some(ZERO_FILE_PLUGIN_ID.into()),
        display_name: Some("Zero File".into()),
        description: Some("Convert PDF and Word files with detected local providers".into()),
        engines: None,
        platforms: Some(vec![PluginPlatform::Macos, PluginPlatform::Windows]),
        runtime: Some(PluginRuntime::Webview),
        contributes: Some(PluginContributions {
            views: Some(vec![PluginContributionView {
                id: "zero.file.main".into(),
                title: "Zero File".into(),
                surface: Some(PluginViewSurface::Main),
            }]),
            commands: None,
            settings: None,
            status_bar_items: None,
        }),
    })
}

fn bundled_record(manifest: PluginManifest) -> PluginRecord {
    let approved_permissions = manifest.permissions.clone();
    let author = manifest.author.clone();
    let version = manifest.version.clone();

    PluginRecord {
        name: manifest.name.clone(),
        version,
        author,
        source: PluginSource::Bundled,
        enabled: true,
        health: PluginHealth::Ready,
        manifest,
        installed_path: None,
        approved_permissions,
        package_sha256: None,
    }
}

fn migrate_bundled_records(records: &mut Vec<PluginRecord>) {
    for bundled in bundled_plugin_records() {
        if let Some(record) = records
            .iter_mut()
            .find(|record| record.name == bundled.name && record.source == PluginSource::Bundled)
        {
            let enabled = record.enabled;
            let health = record.health.clone();
            *record = bundled;
            record.enabled = enabled;
            record.health = if enabled {
                health
            } else {
                PluginHealth::Disabled
            };
        } else if !records.iter().any(|record| record.name == bundled.name) {
            records.push(bundled);
        }
    }
}

fn canonicalize_records(records: &mut Vec<PluginRecord>) {
    let mut canonical = Vec::<(PluginRecord, bool)>::new();

    for mut record in std::mem::take(records) {
        let original_name = record.name.clone();
        canonicalize_record(&mut record);
        let was_already_canonical = original_name == record.name;

        if let Some((existing, existing_was_canonical)) = canonical
            .iter_mut()
            .find(|(existing, _)| existing.name == record.name)
        {
            if was_already_canonical || !*existing_was_canonical {
                *existing = record;
                *existing_was_canonical = was_already_canonical;
            }
        } else {
            canonical.push((record, was_already_canonical));
        }
    }

    *records = canonical.into_iter().map(|(record, _)| record).collect();
}

fn canonicalize_record(record: &mut PluginRecord) {
    record.name = canonical_first_party_plugin_id(&record.name).to_string();
    record.manifest.name = canonical_first_party_plugin_id(&record.manifest.name).to_string();
    if let Some(id) = &mut record.manifest.id {
        *id = canonical_first_party_plugin_id(id).to_string();
    }

    let Some(contributes) = &mut record.manifest.contributes else {
        return;
    };
    if let Some(views) = &mut contributes.views {
        for view in views {
            view.id = canonical_first_party_contribution_id(&view.id);
        }
    }
    if let Some(commands) = &mut contributes.commands {
        for command in commands {
            command.id = canonical_first_party_contribution_id(&command.id);
        }
    }
    if let Some(items) = &mut contributes.status_bar_items {
        for item in items {
            item.id = canonical_first_party_contribution_id(&item.id);
            if let Some(command_id) = &mut item.action.command_id {
                *command_id = canonical_first_party_contribution_id(command_id);
            }
        }
    }
}

fn desktop_platforms() -> Vec<PluginPlatform> {
    vec![
        PluginPlatform::Macos,
        PluginPlatform::Windows,
        PluginPlatform::Linux,
    ]
}

fn permissions_are_approved(requested: &[PluginPermission], approved: &[PluginPermission]) -> bool {
    requested
        .iter()
        .all(|permission| approved.iter().any(|approved| approved == permission))
}

fn validate_market_entry_matches_manifest(
    entry: &PluginMarketEntry,
    manifest: &PluginManifest,
) -> Result<(), String> {
    if entry.name != manifest.name
        || entry.version != manifest.version
        || entry.author != manifest.author
    {
        return Err(format!(
            "market metadata does not match package manifest for {}",
            entry.name
        ));
    }

    Ok(())
}

fn is_protected_host_surface(name: &str) -> bool {
    matches!(name, "preferences" | "about" | "quit")
}

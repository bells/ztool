use serde::{Deserialize, Serialize};
use tauri::Manager;
use tauri_plugin_global_shortcut::GlobalShortcutExt;

use crate::brand::{ZERO_LAUNCH_PLUGIN_ID, ZERO_SNAP_PLUGIN_ID};
use crate::plugins::registry::PluginRegistryState;
use crate::services::quick_launcher::QuickLauncherState;
use crate::{QUICK_LAUNCHER_SHORTCUT, SCREENSHOT_SHORTCUT};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GlobalShortcutActionId {
    SnapCapture,
    LaunchPanel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GlobalShortcutRegistrationState {
    Active,
    Inactive,
    Conflict,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalShortcutSnapshot {
    pub id: GlobalShortcutActionId,
    pub plugin_name: String,
    pub accelerator: String,
    pub enabled: bool,
    pub registered: bool,
    pub platform_supported: bool,
    pub registration_state: GlobalShortcutRegistrationState,
    pub diagnostic_code: Option<String>,
}

pub fn global_shortcut_snapshots(
    app: &tauri::AppHandle,
) -> Result<Vec<GlobalShortcutSnapshot>, String> {
    let registry = app.state::<PluginRegistryState>();
    let (snap_enabled, launch_enabled) = registry.with_registry(|registry| {
        Ok((
            plugin_enabled(registry.records(), ZERO_SNAP_PLUGIN_ID),
            plugin_enabled(registry.records(), ZERO_LAUNCH_PLUGIN_ID),
        ))
    })?;
    let shortcut_manager = app.global_shortcut();
    let launcher_diagnostic = app
        .state::<QuickLauncherState>()
        .snapshot()
        .map_err(|error| error.to_string())?
        .diagnostics
        .into_iter()
        .find(|diagnostic| diagnostic.code == "launcher.shortcut_conflict")
        .map(|diagnostic| diagnostic.code);

    Ok(vec![
        resolve_shortcut_snapshot(ShortcutSnapshotInput {
            id: GlobalShortcutActionId::SnapCapture,
            plugin_name: ZERO_SNAP_PLUGIN_ID,
            accelerator: SCREENSHOT_SHORTCUT,
            enabled: snap_enabled,
            registered: shortcut_manager.is_registered(SCREENSHOT_SHORTCUT),
            platform_supported: cfg!(any(target_os = "macos", target_os = "windows")),
            diagnostic_code: None,
        }),
        resolve_shortcut_snapshot(ShortcutSnapshotInput {
            id: GlobalShortcutActionId::LaunchPanel,
            plugin_name: ZERO_LAUNCH_PLUGIN_ID,
            accelerator: QUICK_LAUNCHER_SHORTCUT,
            enabled: launch_enabled,
            registered: shortcut_manager.is_registered(QUICK_LAUNCHER_SHORTCUT),
            platform_supported: cfg!(any(target_os = "macos", target_os = "windows")),
            diagnostic_code: launcher_diagnostic,
        }),
    ])
}

struct ShortcutSnapshotInput<'a> {
    id: GlobalShortcutActionId,
    plugin_name: &'a str,
    accelerator: &'a str,
    enabled: bool,
    registered: bool,
    platform_supported: bool,
    diagnostic_code: Option<String>,
}

fn resolve_shortcut_snapshot(input: ShortcutSnapshotInput<'_>) -> GlobalShortcutSnapshot {
    let registration_state = if !input.platform_supported {
        GlobalShortcutRegistrationState::Unsupported
    } else if input.diagnostic_code.is_some() {
        GlobalShortcutRegistrationState::Conflict
    } else if input.enabled && input.registered {
        GlobalShortcutRegistrationState::Active
    } else {
        GlobalShortcutRegistrationState::Inactive
    };

    GlobalShortcutSnapshot {
        id: input.id,
        plugin_name: input.plugin_name.into(),
        accelerator: input.accelerator.into(),
        enabled: input.enabled,
        registered: input.registered,
        platform_supported: input.platform_supported,
        registration_state,
        diagnostic_code: input.diagnostic_code,
    }
}

fn plugin_enabled(records: &[crate::plugins::contracts::PluginRecord], name: &str) -> bool {
    records
        .iter()
        .any(|record| record.name == name && record.enabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(
        enabled: bool,
        registered: bool,
        platform_supported: bool,
        diagnostic_code: Option<&str>,
    ) -> GlobalShortcutSnapshot {
        resolve_shortcut_snapshot(ShortcutSnapshotInput {
            id: GlobalShortcutActionId::LaunchPanel,
            plugin_name: ZERO_LAUNCH_PLUGIN_ID,
            accelerator: QUICK_LAUNCHER_SHORTCUT,
            enabled,
            registered,
            platform_supported,
            diagnostic_code: diagnostic_code.map(str::to_string),
        })
    }

    #[test]
    fn resolves_registration_states_without_claiming_false_activity() {
        assert_eq!(
            snapshot(true, true, true, None).registration_state,
            GlobalShortcutRegistrationState::Active
        );
        assert_eq!(
            snapshot(false, true, true, None).registration_state,
            GlobalShortcutRegistrationState::Inactive
        );
        assert_eq!(
            snapshot(true, false, true, Some("launcher.shortcut_conflict")).registration_state,
            GlobalShortcutRegistrationState::Conflict
        );
        assert_eq!(
            snapshot(true, true, false, None).registration_state,
            GlobalShortcutRegistrationState::Unsupported
        );
    }

    #[test]
    fn serializes_a_symmetric_camel_case_contract() {
        let value = serde_json::to_value(snapshot(true, true, true, None)).unwrap();
        assert_eq!(value["id"], "launchPanel");
        assert_eq!(value["pluginName"], ZERO_LAUNCH_PLUGIN_ID);
        assert_eq!(value["registrationState"], "active");
        assert_eq!(value["platformSupported"], true);
    }
}

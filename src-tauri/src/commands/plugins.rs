use tauri::{Manager, State};

use crate::plugins::contracts::{
    InstallMarketPluginInput, InstallPluginPackageInput, PluginIdentityInput,
    PluginLifecycleResult, PluginMarketEntry, PluginPackageValidationReport, PluginRecord,
    SetPluginEnabledInput, ValidatePluginPackageInput,
};
use crate::plugins::market::{fetch_market_json, PluginMarketSnapshot, PluginMarketState};
use crate::plugins::package::{download_package_to_staging, PluginPackageDownloadRequest};
use crate::plugins::registry::PluginRegistryState;

#[tauri::command]
pub async fn refresh_plugin_market(
    state: State<'_, PluginMarketState>,
) -> Result<PluginMarketSnapshot, String> {
    let source_url = state.source_url().map_err(|error| error.message)?;
    let market_json = fetch_market_json(&source_url)
        .await
        .map_err(|error| error.message)?;

    state
        .refresh_from_json(&market_json)
        .map_err(|error| error.message)
}

#[tauri::command]
pub fn list_market_plugins(
    state: State<'_, PluginMarketState>,
    registry_state: State<'_, PluginRegistryState>,
) -> Result<Vec<PluginMarketEntry>, String> {
    let mut entries = state.cached_entries().map_err(|error| error.message)?;
    let records = registry_state.with_registry(|registry| Ok(registry.records().to_vec()))?;

    for entry in &mut entries {
        entry.installed_version = records
            .iter()
            .find(|record| record.name == entry.name)
            .map(|record| record.version.clone());
    }

    Ok(entries)
}

#[tauri::command]
pub fn list_plugins(state: State<'_, PluginRegistryState>) -> Result<Vec<PluginRecord>, String> {
    state.with_registry(|registry| Ok(registry.records().to_vec()))
}

#[tauri::command]
pub async fn validate_plugin_package(
    input: ValidatePluginPackageInput,
    app: tauri::AppHandle,
) -> Result<PluginPackageValidationReport, String> {
    run_plugin_worker(app, move |app| {
        app.state::<PluginRegistryState>()
            .with_registry(|registry| registry.validate_package(input.package_path))
    })
    .await
}

#[tauri::command]
pub async fn install_plugin_package(
    input: InstallPluginPackageInput,
    app: tauri::AppHandle,
) -> Result<PluginRecord, String> {
    run_plugin_worker(app, move |app| {
        let (record, prior_version) =
            app.state::<PluginRegistryState>()
                .with_registry(|registry| {
                    let prior_version = registry
                        .records()
                        .iter()
                        .find(|record| record.name == crate::brand::ZERO_FILE_PLUGIN_ID)
                        .map(|record| record.version.clone());
                    registry
                        .install_local_package(input)
                        .map(|record| (record, prior_version))
                })?;
        let _ = crate::services::status_bar::refresh_status_bar(app);
        sync_file_capability_lifecycle(app, &record, prior_version.as_deref());
        if record.name == crate::brand::ZERO_LAUNCH_PLUGIN_ID {
            sync_quick_launcher_lifecycle(app, record.enabled);
            let _ = crate::sync_quick_launcher_shortcut(app, record.enabled);
        }
        Ok(record)
    })
    .await
}

#[tauri::command]
pub async fn install_market_plugin(
    input: InstallMarketPluginInput,
    app: tauri::AppHandle,
) -> Result<PluginRecord, String> {
    let staging_dir = std::env::temp_dir().join(format!(
        "zero-market-plugin-{}-{}",
        input.entry.name,
        std::process::id()
    ));
    let download = download_package_to_staging(
        &PluginPackageDownloadRequest {
            download_url: input.entry.download_url.clone(),
            sha256: input.entry.sha256.clone(),
        },
        &staging_dir,
    )
    .await
    .map_err(|error| error.message)?;

    let install_input = InstallPluginPackageInput {
        package_path: download.staged_path.to_string_lossy().into_owned(),
        approved_permissions: input.approved_permissions,
        enabled: input.enabled,
    };

    run_plugin_worker(app, move |app| {
        let result = app
            .state::<PluginRegistryState>()
            .with_registry(|registry| {
                let prior_version = registry
                    .records()
                    .iter()
                    .find(|record| record.name == crate::brand::ZERO_FILE_PLUGIN_ID)
                    .map(|record| record.version.clone());
                registry
                    .install_market_package_from_path(&input.entry, install_input)
                    .map(|record| (record, prior_version))
            });
        let _ = std::fs::remove_dir_all(&staging_dir);
        let (record, prior_version) = result?;
        let _ = crate::services::status_bar::refresh_status_bar(app);
        sync_file_capability_lifecycle(app, &record, prior_version.as_deref());
        Ok(record)
    })
    .await
}

#[tauri::command]
pub async fn uninstall_plugin(
    input: PluginIdentityInput,
    app: tauri::AppHandle,
) -> Result<PluginLifecycleResult, String> {
    run_plugin_worker(app, move |app| {
        let result = app
            .state::<PluginRegistryState>()
            .with_registry(|registry| registry.uninstall_plugin(&input.name))?;
        let _ = crate::services::status_bar::refresh_status_bar(app);
        if input.name == crate::brand::ZERO_FILE_PLUGIN_ID {
            reset_file_engine_lifecycle(
                app,
                crate::services::file::runtime::FileCapabilityInvalidationCause::EngineRemoved,
                "The Zero File engine was removed.",
            );
        }
        if result
            .plugin
            .as_ref()
            .is_some_and(|record| record.name == crate::brand::ZERO_LAUNCH_PLUGIN_ID)
        {
            sync_quick_launcher_lifecycle(app, false);
            let _ = crate::sync_quick_launcher_shortcut(app, false);
        }
        Ok(result)
    })
    .await
}

#[tauri::command]
pub async fn set_plugin_enabled(
    input: SetPluginEnabledInput,
    app: tauri::AppHandle,
) -> Result<PluginRecord, String> {
    run_plugin_worker(app, move |app| {
        let record = app
            .state::<PluginRegistryState>()
            .with_registry(|registry| {
                let record = registry.set_enabled(&input.name, input.enabled)?;
                registry.save()?;
                Ok(record)
            })?;
        let _ = crate::services::status_bar::refresh_status_bar(app);
        if record.name == crate::brand::ZERO_FILE_PLUGIN_ID {
            reset_file_engine_lifecycle(
                app,
                if record.enabled {
                    crate::services::file::runtime::FileCapabilityInvalidationCause::EngineRepaired
                } else {
                    crate::services::file::runtime::FileCapabilityInvalidationCause::EngineRemoved
                },
                "The Zero File engine lifecycle changed.",
            );
        }
        if record.name == crate::brand::ZERO_LAUNCH_PLUGIN_ID {
            sync_quick_launcher_lifecycle(app, record.enabled);
            let _ = crate::sync_quick_launcher_shortcut(app, record.enabled);
        }
        Ok(record)
    })
    .await
}

#[tauri::command]
pub async fn restore_bundled_plugins(app: tauri::AppHandle) -> Result<Vec<PluginRecord>, String> {
    run_plugin_worker(app, move |app| {
        let records = app
            .state::<PluginRegistryState>()
            .with_registry(|registry| registry.restore_bundled_defaults())?;
        let _ = crate::services::status_bar::refresh_status_bar(app);
        if records
            .iter()
            .any(|record| record.name == crate::brand::ZERO_FILE_PLUGIN_ID)
        {
            reset_file_engine_lifecycle(
                app,
                crate::services::file::runtime::FileCapabilityInvalidationCause::EngineRepaired,
                "The bundled Zero File engine was restored.",
            );
        }
        sync_quick_launcher_lifecycle(app, true);
        let _ = crate::sync_quick_launcher_shortcut(app, true);
        Ok(records)
    })
    .await
}

async fn run_plugin_worker<T: Send + 'static>(
    app: tauri::AppHandle,
    operation: impl FnOnce(&tauri::AppHandle) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || operation(&app))
        .await
        .map_err(|_| "plugin lifecycle worker stopped unexpectedly".to_string())?
}

fn sync_quick_launcher_lifecycle(app: &tauri::AppHandle, enabled: bool) {
    if enabled {
        crate::bundled_plugins::start_quick_launcher(app);
    } else {
        app.state::<crate::services::quick_launcher::QuickLauncherState>()
            .set_enabled(false);
    }
}

fn sync_file_capability_lifecycle(
    app: &tauri::AppHandle,
    record: &PluginRecord,
    prior_version: Option<&str>,
) {
    if record.name != crate::brand::ZERO_FILE_PLUGIN_ID {
        return;
    }
    let cause = if prior_version.is_some() {
        crate::services::file::runtime::FileCapabilityInvalidationCause::EngineUpgraded
    } else {
        crate::services::file::runtime::FileCapabilityInvalidationCause::EngineInstalled
    };
    reset_file_engine_lifecycle(
        app,
        cause,
        "The installed Zero File engine version changed.",
    );
}

fn reset_file_engine_lifecycle(
    app: &tauri::AppHandle,
    cause: crate::services::file::runtime::FileCapabilityInvalidationCause,
    message: &str,
) {
    app.state::<crate::services::file::FileConversionState>()
        .invalidate_capabilities(cause);
    app.state::<crate::services::file::engine_bridge::FileEngineBridgeState>()
        .bridge
        .reset_and_destroy(app, message);
}

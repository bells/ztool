// The existing objc 0.2 macros expand a historical `cargo-clippy` cfg in this crate.
#![allow(unexpected_cfgs)]

use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, ShortcutState};
use tauri_plugin_positioner::{Position, WindowExt};

pub mod brand;
mod bundled_plugins;
pub mod commands;
pub mod migration;
pub mod plugins;
pub mod services;

pub const SCREENSHOT_SHORTCUT: &str = "CommandOrControl+Shift+A";
pub const QUICK_LAUNCHER_SHORTCUT: &str = "CommandOrControl+Shift+Space";
const TRAY_WINDOW_LABEL: &str = "tray";

fn is_quick_launcher_shortcut(modifiers: Modifiers, key: Code) -> bool {
    #[cfg(target_os = "macos")]
    let command_or_control = Modifiers::SUPER;
    #[cfg(not(target_os = "macos"))]
    let command_or_control = Modifiers::CONTROL;

    modifiers == (command_or_control | Modifiers::SHIFT) && key == Code::Space
}

fn plugin_is_enabled(app: &tauri::AppHandle, plugin_name: &str) -> bool {
    app.state::<plugins::registry::PluginRegistryState>()
        .with_registry(|registry| {
            Ok(registry
                .records()
                .iter()
                .any(|record| record.name == plugin_name && record.enabled))
        })
        .unwrap_or(false)
}

fn quick_launcher_is_enabled(app: &tauri::AppHandle) -> bool {
    plugin_is_enabled(app, brand::ZERO_LAUNCH_PLUGIN_ID)
}

pub fn sync_quick_launcher_shortcut(app: &tauri::AppHandle, enabled: bool) -> Result<(), String> {
    let shortcuts = app.global_shortcut();
    if enabled {
        if shortcuts.is_registered(QUICK_LAUNCHER_SHORTCUT) {
            return Ok(());
        }
        shortcuts
            .register(QUICK_LAUNCHER_SHORTCUT)
            .map_err(|error| {
                let message = format!("Could not register {QUICK_LAUNCHER_SHORTCUT}: {error}");
                app.state::<services::quick_launcher::QuickLauncherState>()
                    .add_diagnostic("launcher.shortcut_conflict", &message);
                message
            })
    } else {
        if shortcuts.is_registered(QUICK_LAUNCHER_SHORTCUT) {
            shortcuts
                .unregister(QUICK_LAUNCHER_SHORTCUT)
                .map_err(|error| format!("failed to unregister launcher shortcut: {error}"))?;
        }
        commands::quick_launcher::hide_quick_launcher_window(app.clone())
    }
}

pub fn toggle_tray_quick_panel(app: &tauri::AppHandle) -> Result<(), String> {
    use services::surface_activity::{hide_surface, show_surface};

    if let Some(window) = app.get_webview_window(TRAY_WINDOW_LABEL) {
        if window
            .is_visible()
            .map_err(|error| format!("读取托盘窗口状态失败: {error}"))?
        {
            hide_surface(&window).map_err(|error| format!("隐藏托盘窗口失败: {error}"))?;
        } else {
            window
                .as_ref()
                .window()
                .move_window(Position::TrayCenter)
                .map_err(|error| format!("移动托盘窗口失败: {error}"))?;
            show_surface(&window).map_err(|error| format!("显示托盘窗口失败: {error}"))?;
            window
                .set_focus()
                .map_err(|error| format!("聚焦托盘窗口失败: {error}"))?;
        }
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let performance_trace = services::performance::PerformanceTrace::default();
    let migration_started = performance_trace.begin();
    let migration_report = migration::migrate_default_home();
    performance_trace.finish(
        "migration",
        if migration_report.diagnostics.is_empty() {
            if migration_report.completed_fast_path {
                "fast-path"
            } else {
                "migrated"
            }
        } else {
            "error"
        },
        migration_started,
    );
    for diagnostic in migration_report.diagnostics {
        eprintln!("Zero data migration: {diagnostic}");
    }

    let managed_state_started = performance_trace.begin();
    let builder = bundled_plugins::manage_states(tauri::Builder::default());
    performance_trace.finish("managed_state_construction", "ok", managed_state_started);
    let registry_started = performance_trace.begin();
    let registry_state = plugins::registry::PluginRegistryState::default();
    let registry_outcome = if registry_state.startup_write_performed() {
        "persisted-change"
    } else {
        "unchanged"
    };
    performance_trace.finish(
        "plugin_registry_load_write",
        registry_outcome,
        registry_started,
    );
    let builder = plugins::engine_assets::register(builder)
        .manage(registry_state)
        .manage(performance_trace.clone());

    let tauri_setup_started = performance_trace.begin();
    let app = builder
        .manage(plugins::market::PluginMarketState::default())
        .manage(services::status_bar::StatusBarState::default())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcuts([SCREENSHOT_SHORTCUT])
                .expect("invalid screenshot shortcut")
                .with_handler(|app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    if is_quick_launcher_shortcut(shortcut.mods, shortcut.key) {
                        if quick_launcher_is_enabled(app) {
                            let _ =
                                commands::quick_launcher::show_quick_launcher_window(app.clone());
                        }
                    } else if plugin_is_enabled(app, brand::ZERO_SNAP_PLUGIN_ID) {
                        let screenshot_app = app.clone();
                        tauri::async_runtime::spawn_blocking(move || {
                            let _ = services::screenshot::start_screenshot_session(
                                screenshot_app,
                                "copy".into(),
                            );
                        });
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_positioner::init())
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            }

            let status_bar_started = app
                .state::<services::performance::PerformanceTrace>()
                .begin();
            let status_bar_result = services::status_bar::refresh_status_bar(app.handle());
            app.state::<services::performance::PerformanceTrace>()
                .finish(
                    "status_bar_creation",
                    if status_bar_result.is_ok() {
                        "ok"
                    } else {
                        "error"
                    },
                    status_bar_started,
                );

            if quick_launcher_is_enabled(app.handle()) {
                bundled_plugins::start_quick_launcher(app.handle());
                let _ = sync_quick_launcher_shortcut(app.handle(), true);
            }

            #[cfg(debug_assertions)]
            bundled_plugins::start_file_engine_smoke_if_requested(app.handle());

            let trace = app
                .state::<services::performance::PerformanceTrace>()
                .inner()
                .clone();
            if trace.emits_logs() {
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                    trace.mark("settled_idle", "ok");
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::quit_app,
            commands::app::mark_frontend_ready,
            commands::app::mark_surface_ready,
            commands::app::record_plugin_activation,
            commands::app::get_performance_trace,
            services::surface_activity::get_surface_activity,
            services::surface_activity::hide_current_surface,
            services::surface_activity::close_current_surface,
            commands::app::show_about_window,
            commands::app::show_main_window,
            commands::app::show_preferences_window,
            commands::plugins::refresh_plugin_market,
            commands::plugins::list_market_plugins,
            commands::plugins::list_plugins,
            commands::plugins::validate_plugin_package,
            commands::plugins::install_market_plugin,
            commands::plugins::install_plugin_package,
            commands::plugins::uninstall_plugin,
            commands::plugins::set_plugin_enabled,
            commands::plugins::restore_bundled_plugins,
            commands::status_bar::get_status_bar_settings,
            commands::status_bar::update_status_bar_settings,
            commands::status_bar::get_status_bar_items,
            commands::status_bar::run_status_bar_item_action,
            commands::shortcuts::get_global_shortcut_snapshots,
            commands::caffeine::get_caffeine_state,
            commands::caffeine::toggle_keep_awake,
            commands::file::get_file_conversion_capabilities,
            commands::file::refresh_file_conversion_capabilities,
            commands::file::choose_file_conversion_inputs,
            commands::file::inspect_file_conversion_inputs,
            commands::file::enqueue_file_conversions,
            commands::file::list_file_conversion_jobs,
            commands::file::start_file_conversion_queue,
            commands::file::cancel_file_conversion_job,
            commands::file::remove_file_conversion_job,
            commands::file::retry_file_conversion_job,
            commands::file::clear_completed_file_conversion_jobs,
            commands::file::open_file_conversion_output,
            commands::file::reveal_file_conversion_output,
            services::file::engine_bridge::file_engine_ready,
            services::file::engine_bridge::file_engine_read_input,
            services::file::engine_bridge::file_engine_write_output,
            services::file::engine_bridge::file_engine_progress,
            services::file::engine_bridge::file_engine_complete,
            services::file::engine_bridge::file_engine_print_rendered,
            commands::bing_wallpaper::get_bing_wallpaper_snapshot,
            commands::bing_wallpaper::refresh_bing_wallpapers,
            commands::bing_wallpaper::get_bing_wallpaper_preview,
            commands::bing_wallpaper::read_bing_wallpaper_preview,
            commands::bing_wallpaper::release_bing_wallpaper_preview,
            commands::bing_wallpaper::save_bing_wallpaper_to_downloads,
            commands::bing_wallpaper::apply_bing_wallpaper,
            commands::quick_launcher::get_quick_launcher_snapshot,
            commands::quick_launcher::refresh_quick_launcher_index,
            commands::quick_launcher::search_quick_launcher,
            commands::quick_launcher::get_quick_launcher_icon,
            commands::quick_launcher::get_quick_launcher_icons,
            commands::quick_launcher::refresh_quick_launcher_running_state,
            commands::quick_launcher::activate_quick_launcher_item,
            commands::quick_launcher::show_quick_launcher_window,
            commands::quick_launcher::hide_quick_launcher_window,
            commands::screenshot::get_screenshot_capabilities,
            commands::screenshot::start_screenshot,
            commands::screenshot::init_screenshot_session,
            commands::screenshot::read_screenshot_media,
            commands::screenshot::prepare_screenshot_commit,
            commands::screenshot::upload_screenshot_commit,
            commands::screenshot::cancel_screenshot_session,
            commands::screenshot::init_pin_window,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");
    performance_trace.finish("tauri_setup", "ok", tauri_setup_started);
    app.run(|app, event| {
        if matches!(
            event,
            tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
        ) {
            app.state::<services::file::FileConversionState>()
                .shutdown_cleanup();
            app.state::<services::screenshot::ScreenshotSessionStore>()
                .cleanup_all();
            app.state::<services::file::engine_bridge::FileEngineBridgeState>()
                .bridge
                .reset_and_destroy(app, "Zero is shutting down.");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launcher_shortcut_matches_expected_modifiers_only() {
        #[cfg(target_os = "macos")]
        let command_or_control = Modifiers::SUPER;
        #[cfg(not(target_os = "macos"))]
        let command_or_control = Modifiers::CONTROL;

        assert!(is_quick_launcher_shortcut(
            command_or_control | Modifiers::SHIFT,
            Code::Space
        ));
        assert!(!is_quick_launcher_shortcut(command_or_control, Code::Space));
        assert!(!is_quick_launcher_shortcut(
            command_or_control | Modifiers::SHIFT,
            Code::KeyA
        ));
    }
}

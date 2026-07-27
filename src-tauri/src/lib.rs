// The existing objc 0.2 macros expand a historical `cargo-clippy` cfg in this crate.
#![allow(unexpected_cfgs)]

use std::sync::{Arc, Mutex};

use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, ShortcutState};
use tauri_plugin_positioner::{on_tray_event, Position, WindowExt};

pub mod brand;
pub mod commands;
pub mod migration;
pub mod plugins;
pub mod services;

const TRAY_CLICK_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(280);
const SCREENSHOT_SHORTCUT: &str = "CommandOrControl+Shift+A";
pub const QUICK_LAUNCHER_SHORTCUT: &str = "CommandOrControl+Shift+Space";
const TRAY_WINDOW_LABEL: &str = "tray";

fn should_accept_tray_toggle(
    last_toggle_at: &mut Option<std::time::Instant>,
    now: std::time::Instant,
) -> bool {
    if last_toggle_at.is_some_and(|previous| now.duration_since(previous) < TRAY_CLICK_DEBOUNCE) {
        return false;
    }

    *last_toggle_at = Some(now);
    true
}

fn is_quick_launcher_shortcut(modifiers: Modifiers, key: Code) -> bool {
    #[cfg(target_os = "macos")]
    let command_or_control = Modifiers::SUPER;
    #[cfg(not(target_os = "macos"))]
    let command_or_control = Modifiers::CONTROL;

    modifiers == (command_or_control | Modifiers::SHIFT) && key == Code::Space
}

fn quick_launcher_is_enabled(app: &tauri::AppHandle) -> bool {
    app.state::<plugins::registry::PluginRegistryState>()
        .with_registry(|registry| {
            Ok(registry
                .records()
                .iter()
                .any(|record| record.name == brand::ZERO_LAUNCH_PLUGIN_ID && record.enabled))
        })
        .unwrap_or(false)
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
    if let Some(window) = app.get_webview_window(TRAY_WINDOW_LABEL) {
        if window
            .is_visible()
            .map_err(|error| format!("读取托盘窗口状态失败: {error}"))?
        {
            window
                .hide()
                .map_err(|error| format!("隐藏托盘窗口失败: {error}"))?;
        } else {
            window
                .as_ref()
                .window()
                .move_window(Position::TrayCenter)
                .map_err(|error| format!("移动托盘窗口失败: {error}"))?;
            window
                .show()
                .map_err(|error| format!("显示托盘窗口失败: {error}"))?;
            window
                .set_focus()
                .map_err(|error| format!("聚焦托盘窗口失败: {error}"))?;
        }
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let migration_report = migration::migrate_default_home();
    for diagnostic in migration_report.diagnostics {
        eprintln!("Zero data migration: {diagnostic}");
    }

    tauri::Builder::default()
        .manage(services::caffeine::CaffeineState::new())
        .manage(services::bing_wallpaper::BingWallpaperState::default())
        .manage(plugins::market::PluginMarketState::default())
        .manage(plugins::registry::PluginRegistryState::default())
        .manage(services::screenshot::ScreenshotSessionStore::default())
        .manage(services::status_bar::StatusBarState::default())
        .manage(services::quick_launcher::QuickLauncherState::default())
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
                    } else {
                        let _ = services::screenshot::start_screenshot_session(
                            app.clone(),
                            "copy".into(),
                        );
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

            let last_tray_toggle_at = Arc::new(Mutex::new(None::<std::time::Instant>));
            let last_tray_toggle_at_tray = last_tray_toggle_at.clone();
            let primary_status_bar_icon = services::status_bar::primary_status_bar_icon_image()
                .map_err(std::io::Error::other)?;

            let _tray = TrayIconBuilder::with_id(brand::PRIMARY_STATUS_ITEM_ID)
                .icon(primary_status_bar_icon)
                .icon_as_template(true)
                .tooltip(brand::PRODUCT_NAME)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(move |tray, event| {
                    let app_handle = tray.app_handle();
                    on_tray_event(&app_handle, &event);

                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        ..
                    } = event
                    {
                        let now = std::time::Instant::now();
                        if let Ok(mut last_toggle) = last_tray_toggle_at_tray.lock() {
                            if !should_accept_tray_toggle(&mut last_toggle, now) {
                                return;
                            }
                        }

                        let _ = toggle_tray_quick_panel(tray.app_handle());
                    }
                })
                .build(app)?;

            let _ = services::status_bar::refresh_status_bar(app.handle());

            let launcher_state = app.state::<services::quick_launcher::QuickLauncherState>();
            if let Err(error) = launcher_state.start_watcher(app.handle().clone()) {
                launcher_state.add_diagnostic("launcher.watcher_unavailable", error);
            }
            let launcher_app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let refresh_app = launcher_app.clone();
                let result = tauri::async_runtime::spawn_blocking(move || {
                    refresh_app
                        .state::<services::quick_launcher::QuickLauncherState>()
                        .refresh(&services::quick_launcher::system_language())
                })
                .await;
                if let Err(error) = result {
                    launcher_app
                        .state::<services::quick_launcher::QuickLauncherState>()
                        .add_diagnostic(
                            "launcher.refresh_join_failed",
                            format!("Launcher startup refresh failed: {error}"),
                        );
                }
            });

            if quick_launcher_is_enabled(app.handle()) {
                let _ = sync_quick_launcher_shortcut(app.handle(), true);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::quit_app,
            commands::app::show_about_window,
            commands::app::show_main_window,
            commands::app::show_preferences_window,
            commands::bing_wallpaper::get_bing_wallpaper_snapshot,
            commands::bing_wallpaper::refresh_bing_wallpapers,
            commands::bing_wallpaper::get_bing_wallpaper_preview,
            commands::bing_wallpaper::save_bing_wallpaper_to_downloads,
            commands::bing_wallpaper::apply_bing_wallpaper,
            commands::caffeine::get_caffeine_state,
            commands::caffeine::toggle_keep_awake,
            commands::plugins::refresh_plugin_market,
            commands::plugins::list_market_plugins,
            commands::plugins::list_plugins,
            commands::plugins::validate_plugin_package,
            commands::plugins::install_market_plugin,
            commands::plugins::install_plugin_package,
            commands::plugins::uninstall_plugin,
            commands::plugins::set_plugin_enabled,
            commands::plugins::restore_bundled_plugins,
            commands::quick_launcher::get_quick_launcher_snapshot,
            commands::quick_launcher::refresh_quick_launcher_index,
            commands::quick_launcher::search_quick_launcher,
            commands::quick_launcher::get_quick_launcher_icon,
            commands::quick_launcher::activate_quick_launcher_item,
            commands::quick_launcher::show_quick_launcher_window,
            commands::quick_launcher::hide_quick_launcher_window,
            commands::status_bar::get_status_bar_settings,
            commands::status_bar::update_status_bar_settings,
            commands::status_bar::get_status_bar_items,
            commands::status_bar::run_status_bar_item_action,
            commands::screenshot::get_screenshot_capabilities,
            commands::screenshot::start_screenshot,
            commands::screenshot::init_screenshot_session,
            commands::screenshot::commit_screenshot,
            commands::screenshot::cancel_screenshot_session,
            commands::screenshot::pin_screenshot,
            commands::screenshot::init_pin_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_toggle_debounce_rejects_repeated_click_events() {
        let start = std::time::Instant::now();
        let mut last_toggle_at = None;

        assert!(should_accept_tray_toggle(&mut last_toggle_at, start));
        assert!(!should_accept_tray_toggle(
            &mut last_toggle_at,
            start + TRAY_CLICK_DEBOUNCE / 2
        ));
        assert!(should_accept_tray_toggle(
            &mut last_toggle_at,
            start + TRAY_CLICK_DEBOUNCE + std::time::Duration::from_millis(1)
        ));
    }

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

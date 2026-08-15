use tauri::Manager;

pub fn manage_states(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder
        .manage(crate::services::caffeine::CaffeineState::new())
        .manage(crate::services::bing_wallpaper::BingWallpaperState::default())
        .manage(crate::services::quick_launcher::QuickLauncherState::default())
        .manage(crate::services::screenshot::ScreenshotSessionStore::default())
}

pub fn start_quick_launcher(app: &tauri::AppHandle) {
    let launcher_state = app.state::<crate::services::quick_launcher::QuickLauncherState>();
    if let Err(error) = launcher_state.start_watcher(app.clone()) {
        launcher_state.add_diagnostic("launcher.watcher_unavailable", error);
    }

    let launcher_app = app.clone();
    tauri::async_runtime::spawn(async move {
        let refresh_app = launcher_app.clone();
        let result = tauri::async_runtime::spawn_blocking(move || {
            refresh_app
                .state::<crate::services::quick_launcher::QuickLauncherState>()
                .refresh(&crate::services::quick_launcher::system_language())
        })
        .await;
        if let Err(error) = result {
            launcher_app
                .state::<crate::services::quick_launcher::QuickLauncherState>()
                .add_diagnostic(
                    "launcher.refresh_join_failed",
                    format!("Launcher startup refresh failed: {error}"),
                );
        }
    });
}

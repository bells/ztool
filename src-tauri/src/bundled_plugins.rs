use tauri::Manager;

pub fn manage_states(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    let engine_bridge = crate::services::file::engine_bridge::FileEngineBridgeState::default();
    builder
        .manage(crate::services::caffeine::CaffeineState::default())
        .manage(crate::services::bing_wallpaper::BingWallpaperState::default())
        .manage(crate::services::quick_launcher::QuickLauncherState::default())
        .manage(crate::services::file::FileConversionState::default())
        .manage(engine_bridge)
        .manage(crate::services::screenshot::ScreenshotSessionStore::default())
}

pub fn start_quick_launcher(app: &tauri::AppHandle) {
    let launcher_state = app.state::<crate::services::quick_launcher::QuickLauncherState>();
    launcher_state.set_enabled(true);

    let launcher_app = app.clone();
    tauri::async_runtime::spawn(async move {
        let initialization_app = launcher_app.clone();
        let result = tauri::async_runtime::spawn_blocking(move || {
            let trace =
                initialization_app.state::<crate::services::performance::PerformanceTrace>();
            let started = trace.begin();
            let result = initialization_app
                .state::<crate::services::quick_launcher::QuickLauncherState>()
                .initialize(initialization_app.clone());
            trace.finish(
                "launch_initialization",
                if result.is_ok() { "ok" } else { "error" },
                started,
            );
            result
        })
        .await;
        match result {
            Ok(Ok(())) => {}
            Ok(Err(error)) => launcher_app
                .state::<crate::services::quick_launcher::QuickLauncherState>()
                .add_diagnostic("launcher.initialize_failed", error.message),
            Err(error) => launcher_app
                .state::<crate::services::quick_launcher::QuickLauncherState>()
                .add_diagnostic(
                    "launcher.initialize_join_failed",
                    format!("Launcher startup initialization failed: {error}"),
                ),
        }
    });
}

#[cfg(debug_assertions)]
pub fn start_file_engine_smoke_if_requested(app: &tauri::AppHandle) {
    use crate::services::file::contracts::{
        FileConversionEnqueueItem, FileConversionEnqueueRequest, FileConversionJobState,
    };

    let Ok(source_path) = std::env::var("ZERO_FILE_ENGINE_SMOKE_INPUT") else {
        return;
    };
    let initialization = app
        .path()
        .app_cache_dir()
        .map_err(|_| "The Zero cache directory is unavailable.".to_string())
        .and_then(|cache_root| {
            app.state::<crate::services::file::FileConversionState>()
                .initialize_with_engine(
                    app.clone(),
                    std::sync::Arc::clone(
                        &app.state::<crate::services::file::engine_bridge::FileEngineBridgeState>()
                            .bridge,
                    ),
                    cache_root.join("file-conversion"),
                )
                .map_err(|error| error.message)
        });
    if let Err(error) = initialization {
        eprintln!("ZERO_FILE_ENGINE_SMOKE initialization failed: {error}");
        return;
    }
    let output_directory = std::env::var("ZERO_FILE_ENGINE_SMOKE_OUTPUT")
        .unwrap_or_else(|_| "/private/tmp/zero-file-engine-smoke".into());
    if let Err(error) = std::fs::create_dir_all(&output_directory) {
        eprintln!("ZERO_FILE_ENGINE_SMOKE failed to create output directory: {error}");
        return;
    }
    let state = app.state::<crate::services::file::FileConversionState>();
    let batch = match state.enqueue(FileConversionEnqueueRequest {
        items: vec![FileConversionEnqueueItem {
            source_path,
            output_directory: Some(output_directory),
        }],
    }) {
        Ok(batch) if batch.jobs.len() == 1 => batch,
        Ok(batch) => {
            eprintln!(
                "ZERO_FILE_ENGINE_SMOKE enqueue rejected: {:?}",
                batch.rejected_candidates
            );
            return;
        }
        Err(error) => {
            eprintln!("ZERO_FILE_ENGINE_SMOKE enqueue failed: {error:?}");
            return;
        }
    };
    let job_id = batch.jobs[0].id.clone();
    let (_, should_spawn) = match state.start() {
        Ok(result) => result,
        Err(error) => {
            eprintln!("ZERO_FILE_ENGINE_SMOKE start failed: {error:?}");
            return;
        }
    };
    if should_spawn {
        let worker_app = app.clone();
        tauri::async_runtime::spawn(async move {
            crate::services::file::runtime::run_file_conversion_worker(worker_app).await;
        });
    }
    let monitor_app = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let snapshots = monitor_app
                .state::<crate::services::file::FileConversionState>()
                .snapshots()
                .unwrap_or_default();
            let Some(job) = snapshots.iter().find(|job| job.id == job_id) else {
                continue;
            };
            match &job.state {
                FileConversionJobState::Completed { result } => {
                    eprintln!(
                        "ZERO_FILE_ENGINE_SMOKE completed: {} {:?}",
                        result.output_path, result.quality_profile
                    );
                    break;
                }
                FileConversionJobState::Failed { error }
                | FileConversionJobState::Cancelled { error } => {
                    eprintln!("ZERO_FILE_ENGINE_SMOKE failed: {error:?}");
                    break;
                }
                _ => {}
            }
        }
    });
}

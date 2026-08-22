use tauri::{Emitter, Manager, State};

use crate::services::file::contracts::{
    FileConversionBatchResult, FileConversionCandidate, FileConversionCapabilitySnapshot,
    FileConversionEnqueueRequest, FileConversionError, FileConversionInspectRequest,
    FileConversionJobRequest, FileConversionJobSnapshot,
};
use crate::services::file::result_actions::CompletedOutputAction;
use crate::services::file::runtime::{
    run_file_conversion_worker, FileCapabilityInvalidationCause, FILE_CONVERSION_JOB_UPDATED_EVENT,
};
use crate::services::file::FileConversionState;

#[tauri::command]
pub async fn get_file_conversion_capabilities(
    app: tauri::AppHandle,
) -> Result<FileConversionCapabilitySnapshot, FileConversionError> {
    ensure_initialized(&app)?;
    let capability_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        capability_app.state::<FileConversionState>().capabilities()
    })
    .await
    .map_err(|_| command_error("The File capability worker stopped unexpectedly."))
}

#[tauri::command]
pub async fn refresh_file_conversion_capabilities(
    app: tauri::AppHandle,
) -> Result<FileConversionCapabilitySnapshot, FileConversionError> {
    ensure_initialized(&app)?;
    let capability_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = capability_app.state::<FileConversionState>();
        state.invalidate_capabilities(FileCapabilityInvalidationCause::NativeProviderChanged);
        state.capabilities()
    })
    .await
    .map_err(|_| command_error("The File capability refresh worker stopped unexpectedly."))
}

#[tauri::command]
pub async fn choose_file_conversion_inputs(
    app: tauri::AppHandle,
) -> Result<Vec<FileConversionCandidate>, FileConversionError> {
    let paths = tauri::async_runtime::spawn_blocking(|| {
        rfd::FileDialog::new()
            .add_filter("PDF and Word documents", &["pdf", "docx"])
            .pick_files()
            .unwrap_or_default()
    })
    .await
    .map_err(|_| command_error("The native file picker stopped unexpectedly."))?;
    Ok(app.state::<FileConversionState>().inspect_paths(
        paths
            .into_iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect(),
    ))
}

#[tauri::command]
pub async fn inspect_file_conversion_inputs(
    input: FileConversionInspectRequest,
    app: tauri::AppHandle,
) -> Result<Vec<FileConversionCandidate>, FileConversionError> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<FileConversionState>()
            .inspect_paths(input.source_paths)
    })
    .await
    .map_err(|_| command_error("The File input inspection worker stopped unexpectedly."))
}

#[tauri::command]
pub async fn enqueue_file_conversions(
    input: FileConversionEnqueueRequest,
    app: tauri::AppHandle,
) -> Result<FileConversionBatchResult, FileConversionError> {
    tauri::async_runtime::spawn_blocking(move || app.state::<FileConversionState>().enqueue(input))
        .await
        .map_err(|_| command_error("The File enqueue worker stopped unexpectedly."))?
}

#[tauri::command]
pub fn list_file_conversion_jobs(
    state: State<'_, FileConversionState>,
) -> Result<Vec<FileConversionJobSnapshot>, FileConversionError> {
    state.snapshots()
}

#[tauri::command]
pub fn start_file_conversion_queue(
    app: tauri::AppHandle,
) -> Result<Vec<FileConversionJobSnapshot>, FileConversionError> {
    ensure_initialized(&app)?;
    let state = app.state::<FileConversionState>();
    let (updates, should_spawn) = state.start()?;
    app.state::<crate::services::file::engine_bridge::FileEngineBridgeState>()
        .bridge
        .cancel_idle_teardown();
    emit_updates(&app, &updates);
    if should_spawn {
        let worker_app = app.clone();
        tauri::async_runtime::spawn(async move {
            run_file_conversion_worker(worker_app).await;
        });
    }
    Ok(updates)
}

fn ensure_initialized(app: &tauri::AppHandle) -> Result<(), FileConversionError> {
    let trace = app.state::<crate::services::performance::PerformanceTrace>();
    let started = trace.begin();
    let temp_root = app
        .path()
        .app_cache_dir()
        .map_err(|_| command_error("The Zero cache directory is unavailable."))?
        .join("file-conversion");
    let bridge = std::sync::Arc::clone(
        &app.state::<crate::services::file::engine_bridge::FileEngineBridgeState>()
            .bridge,
    );
    let result =
        app.state::<FileConversionState>()
            .initialize_with_engine(app.clone(), bridge, temp_root);
    trace.finish(
        "file_initialization",
        if result.is_ok() { "ok" } else { "error" },
        started,
    );
    result
}

#[tauri::command]
pub fn cancel_file_conversion_job(
    input: FileConversionJobRequest,
    app: tauri::AppHandle,
) -> Result<Vec<FileConversionJobSnapshot>, FileConversionError> {
    let updates = app.state::<FileConversionState>().cancel(&input.job_id)?;
    emit_updates(&app, &updates);
    Ok(updates)
}

#[tauri::command]
pub fn remove_file_conversion_job(
    input: FileConversionJobRequest,
    state: State<'_, FileConversionState>,
) -> Result<FileConversionJobSnapshot, FileConversionError> {
    state.remove(&input.job_id)
}

#[tauri::command]
pub fn retry_file_conversion_job(
    input: FileConversionJobRequest,
    app: tauri::AppHandle,
) -> Result<FileConversionJobSnapshot, FileConversionError> {
    let snapshot = app.state::<FileConversionState>().retry(&input.job_id)?;
    let _ = app.emit(FILE_CONVERSION_JOB_UPDATED_EVENT, snapshot.clone());
    Ok(snapshot)
}

#[tauri::command]
pub fn clear_completed_file_conversion_jobs(
    state: State<'_, FileConversionState>,
) -> Result<Vec<FileConversionJobSnapshot>, FileConversionError> {
    state.clear_completed()
}

#[tauri::command]
pub async fn open_file_conversion_output(
    input: FileConversionJobRequest,
    app: tauri::AppHandle,
) -> Result<(), FileConversionError> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<FileConversionState>()
            .run_completed_output_action(&input.job_id, CompletedOutputAction::Open)
    })
    .await
    .map_err(|_| command_error("The File output-open worker stopped unexpectedly."))?
}

#[tauri::command]
pub async fn reveal_file_conversion_output(
    input: FileConversionJobRequest,
    app: tauri::AppHandle,
) -> Result<(), FileConversionError> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<FileConversionState>()
            .run_completed_output_action(&input.job_id, CompletedOutputAction::Reveal)
    })
    .await
    .map_err(|_| command_error("The File output-reveal worker stopped unexpectedly."))?
}

fn emit_updates(app: &tauri::AppHandle, updates: &[FileConversionJobSnapshot]) {
    for update in updates {
        let _ = app.emit(FILE_CONVERSION_JOB_UPDATED_EVENT, update.clone());
    }
}

fn command_error(message: &str) -> FileConversionError {
    FileConversionError {
        code: crate::services::file::contracts::FileConversionErrorCode::Internal,
        message: message.into(),
        retryable: true,
        provider_id: None,
        diagnostic: None,
    }
}

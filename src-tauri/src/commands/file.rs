use tauri::{Emitter, Manager, State};

use crate::services::file::contracts::{
    FileConversionBatchResult, FileConversionCandidate, FileConversionCapabilitySnapshot,
    FileConversionEnqueueRequest, FileConversionError, FileConversionInspectRequest,
    FileConversionJobRequest, FileConversionJobSnapshot,
};
use crate::services::file::result_actions::CompletedOutputAction;
use crate::services::file::runtime::{
    run_file_conversion_worker, FILE_CONVERSION_JOB_UPDATED_EVENT,
};
use crate::services::file::FileConversionState;

#[tauri::command]
pub fn get_file_conversion_capabilities(
    state: State<'_, FileConversionState>,
) -> FileConversionCapabilitySnapshot {
    state.capabilities()
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
pub fn inspect_file_conversion_inputs(
    input: FileConversionInspectRequest,
    state: State<'_, FileConversionState>,
) -> Vec<FileConversionCandidate> {
    state.inspect_paths(input.source_paths)
}

#[tauri::command]
pub fn enqueue_file_conversions(
    input: FileConversionEnqueueRequest,
    state: State<'_, FileConversionState>,
) -> Result<FileConversionBatchResult, FileConversionError> {
    state.enqueue(input)
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
    let state = app.state::<FileConversionState>();
    let (updates, should_spawn) = state.start()?;
    emit_updates(&app, &updates);
    if should_spawn {
        let worker_app = app.clone();
        tauri::async_runtime::spawn(async move {
            run_file_conversion_worker(worker_app).await;
        });
    }
    Ok(updates)
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
pub fn open_file_conversion_output(
    input: FileConversionJobRequest,
    state: State<'_, FileConversionState>,
) -> Result<(), FileConversionError> {
    state.run_completed_output_action(&input.job_id, CompletedOutputAction::Open)
}

#[tauri::command]
pub fn reveal_file_conversion_output(
    input: FileConversionJobRequest,
    state: State<'_, FileConversionState>,
) -> Result<(), FileConversionError> {
    state.run_completed_output_action(&input.job_id, CompletedOutputAction::Reveal)
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

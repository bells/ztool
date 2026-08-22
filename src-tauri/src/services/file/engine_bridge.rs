use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use getrandom::fill;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, WebviewUrl};

use crate::brand::ZERO_FILE_PLUGIN_ID;
use crate::plugins::engine_assets::FILE_ENGINE_SCHEME;
use crate::plugins::registry::{ActivePluginEngine, PluginRegistryState};

use super::contracts::{
    FileConversionDirection, FileConversionError, FileConversionErrorCode, FileConversionProgress,
    FileConversionQualityProfile, FileConversionStage,
};
use super::provider::{
    provider_error, FileConversionCancellationToken, FileConversionProgressSink,
    ProviderConversionOutput, ProviderConversionRequest,
};

pub const FILE_ENGINE_LABEL: &str = "zero-file-engine";
pub const FILE_ENGINE_VERSION: &str = "1.0.0";
pub const FILE_ENGINE_PROTOCOL_VERSION: u16 = 1;
const FILE_ENGINE_PLUGIN_ID: &str = ZERO_FILE_PLUGIN_ID;
const FILE_ENGINE_RUN_EVENT: &str = "zero://file-engine/run";
const FILE_ENGINE_CANCEL_EVENT: &str = "zero://file-engine/cancel";
const MAX_INPUT_BYTES: u64 = 512 * 1024 * 1024;
const MAX_OUTPUT_BYTES: usize = 768 * 1024 * 1024;
const ENGINE_STARTUP_TIMEOUT: Duration = Duration::from_secs(2);
const ENGINE_JOB_TIMEOUT: Duration = Duration::from_secs(120);
const ENGINE_COMPLETION_POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEngineRunRequest {
    protocol_version: u16,
    token: String,
    plugin_id: String,
    engine_version: String,
    job_id: String,
    direction: FileConversionDirection,
    input_name: &'static str,
    output_name: &'static str,
    deadline_ms: u64,
    max_input_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileEngineCancelRequest {
    protocol_version: u16,
    token: String,
    job_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileEngineReadyRequest {
    protocol_version: u16,
    engine_version: String,
    plugin_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileEngineProgressRequest {
    protocol_version: u16,
    token: String,
    engine_version: String,
    job_id: String,
    stage: FileEngineProgressStage,
    percent: Option<u8>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
enum FileEngineProgressStage {
    Parsing,
    Analyzing,
    Rendering,
    Packaging,
    Printing,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileEngineCompletionRequest {
    protocol_version: u16,
    token: String,
    engine_version: String,
    job_id: String,
    status: FileEngineCompletionStatus,
    quality_profile: Option<FileConversionQualityProfile>,
    #[serde(default)]
    warning_keys: Vec<String>,
    page_count: Option<u32>,
    error_code: Option<FileEngineErrorCode>,
    diagnostic: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
enum FileEngineCompletionStatus {
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
enum FileEngineErrorCode {
    PasswordRequired,
    InvalidInput,
    UnsupportedInput,
    Timeout,
    Cancelled,
    ProviderFailed,
    InvalidProviderOutput,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileEngineRenderMeasurement {
    protocol_version: u16,
    token: String,
    engine_version: String,
    job_id: String,
    section_count: u32,
    measured_page_count: u32,
    page_rects: Vec<FileEnginePageRect>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FileEnginePageRect {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

enum EngineMessage {
    Progress(FileConversionProgress),
    Completed(FileEngineCompletion),
    Failed(FileConversionError),
}

struct FileEngineCompletion {
    quality_profile: FileConversionQualityProfile,
    warning_keys: Vec<String>,
    page_count: Option<u32>,
}

struct CompletionWait<'a> {
    app: &'a tauri::AppHandle,
    token: &'a str,
    job_id: &'a str,
    receiver: Receiver<EngineMessage>,
    progress: &'a dyn FileConversionProgressSink,
    cancellation: &'a FileConversionCancellationToken,
    deadline: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompletionWaitState {
    Continue,
    Cancelled,
    TimedOut,
}

struct EngineSession {
    job_id: String,
    engine_version: String,
    direction: FileConversionDirection,
    input_path: PathBuf,
    output_path: PathBuf,
    deadline: Instant,
    sender: Sender<EngineMessage>,
}

#[derive(Default)]
struct EngineRuntime {
    ready_version: Option<String>,
    sessions: HashMap<String, EngineSession>,
}

#[derive(Default)]
pub struct FileEngineBridge {
    runtime: Mutex<EngineRuntime>,
    ready: Condvar,
}

#[derive(Clone, Default)]
pub struct FileEngineBridgeState {
    pub bridge: Arc<FileEngineBridge>,
}

struct LeasedEngine {
    engine: ActivePluginEngine,
    installed: bool,
}

impl FileEngineBridge {
    pub fn convert(
        self: &Arc<Self>,
        app: &tauri::AppHandle,
        request: &ProviderConversionRequest,
        progress: &dyn FileConversionProgressSink,
        cancellation: &FileConversionCancellationToken,
    ) -> Result<ProviderConversionOutput, FileConversionError> {
        cancellation.check()?;
        let leased_engine = acquire_engine(app)?;
        let engine_version = leased_engine.engine.package_version.clone();
        let input_name = match request.direction {
            FileConversionDirection::PdfToDocx => "input.pdf",
            FileConversionDirection::DocxToPdf => "input.docx",
        };
        let output_name = match request.direction {
            FileConversionDirection::PdfToDocx => "provider-output.docx",
            FileConversionDirection::DocxToPdf => "provider-output.pdf",
        };
        let input_path = request.temp_directory.join(input_name);
        let output_path = request.temp_directory.join(output_name);
        let mut issued_token = None;
        let result = (|| {
            stage_validated_input(&request.source_path, &input_path)?;
            let (sender, receiver) = mpsc::channel();
            let token = random_token()?;
            issued_token = Some(token.clone());
            let deadline = Instant::now() + ENGINE_JOB_TIMEOUT;
            self.runtime
                .lock()
                .map_err(|_| bridge_error("The built-in engine session store is unavailable."))?
                .sessions
                .insert(
                    token.clone(),
                    EngineSession {
                        job_id: request.job_id.clone(),
                        engine_version: engine_version.clone(),
                        direction: request.direction,
                        input_path,
                        output_path: output_path.clone(),
                        deadline,
                        sender,
                    },
                );

            self.ensure_window_ready(app, &leased_engine)?;
            app.emit_to(
                FILE_ENGINE_LABEL,
                FILE_ENGINE_RUN_EVENT,
                FileEngineRunRequest {
                    protocol_version: FILE_ENGINE_PROTOCOL_VERSION,
                    token: token.clone(),
                    plugin_id: FILE_ENGINE_PLUGIN_ID.into(),
                    engine_version: engine_version.clone(),
                    job_id: request.job_id.clone(),
                    direction: request.direction,
                    input_name,
                    output_name,
                    deadline_ms: now_ms().saturating_add(ENGINE_JOB_TIMEOUT.as_millis() as u64),
                    max_input_bytes: MAX_INPUT_BYTES,
                },
            )
            .map_err(|_| bridge_error("The built-in engine job could not be dispatched."))?;
            self.wait_for_completion(CompletionWait {
                app,
                token: &token,
                job_id: &request.job_id,
                receiver,
                progress,
                cancellation,
                deadline,
            })
        })();

        if let Some(token) = issued_token {
            self.revoke(&token);
        }
        release_engine(app, &leased_engine);
        match result {
            Ok(completion) => Ok(ProviderConversionOutput {
                path: output_path,
                provider_origin: super::contracts::FileConversionProviderOrigin::BuiltIn,
                engine_version: Some(engine_version),
                quality_profile: completion.quality_profile,
                warning_keys: completion.warning_keys,
                page_count: completion.page_count,
            }),
            Err(error) => {
                let _ = fs::remove_file(output_path);
                Err(error)
            }
        }
    }

    fn ensure_window_ready(
        self: &Arc<Self>,
        app: &tauri::AppHandle,
        leased_engine: &LeasedEngine,
    ) -> Result<(), FileConversionError> {
        let expected_version = leased_engine.engine.package_version.clone();
        if self
            .runtime
            .lock()
            .map_err(|_| bridge_error("The built-in engine readiness state is unavailable."))?
            .ready_version
            .as_deref()
            .is_some_and(|version| version != expected_version)
        {
            if let Some(window) = app.get_webview_window(FILE_ENGINE_LABEL) {
                let _ = window.destroy();
            }
            if let Ok(mut runtime) = self.runtime.lock() {
                runtime.ready_version = None;
            }
        }
        if app.get_webview_window(FILE_ENGINE_LABEL).is_none() {
            let (sender, receiver) = mpsc::sync_channel(1);
            let engine_app = app.clone();
            let bridge = Arc::clone(self);
            let window_url = if leased_engine.installed {
                tauri::Url::parse(&format!(
                    "{FILE_ENGINE_SCHEME}://localhost/{expected_version}/index.html"
                ))
                .map(WebviewUrl::CustomProtocol)
                .map_err(|_| bridge_error("The installed File engine URL is invalid."))?
            } else {
                WebviewUrl::App("index.html".into())
            };
            app.run_on_main_thread(move || {
                let result = (|| {
                    if engine_app.get_webview_window(FILE_ENGINE_LABEL).is_some() {
                        return Ok(());
                    }
                    let window = tauri::WebviewWindowBuilder::new(
                        &engine_app,
                        FILE_ENGINE_LABEL,
                        window_url,
                    )
                    .title("Zero File Engine")
                    .visible(false)
                    .decorations(false)
                    .skip_taskbar(true)
                    .inner_size(900.0, 1200.0)
                    .disable_drag_drop_handler()
                    .on_navigation(|url| {
                        matches!(url.scheme(), "tauri" | "ipc" | FILE_ENGINE_SCHEME)
                            || url.host_str().is_some_and(|host| {
                                host == "tauri.localhost"
                                    || (cfg!(debug_assertions) && host == "localhost")
                            })
                    })
                    .build()
                    .map_err(|_| {
                        bridge_error("The isolated built-in engine WebView could not start.")
                    })?;
                    window.on_window_event(move |event| {
                        if matches!(event, tauri::WindowEvent::Destroyed) {
                            bridge.fail_all_sessions(
                                "The built-in engine WebView stopped unexpectedly.",
                            );
                        }
                    });
                    Ok(())
                })();
                let _ = sender.send(result);
            })
            .map_err(|_| bridge_error("The isolated built-in engine WebView could not start."))?;
            receiver
                .recv_timeout(ENGINE_STARTUP_TIMEOUT)
                .map_err(|_| {
                    bridge_error("The isolated built-in engine WebView did not start in time.")
                })??;
        }
        let runtime = self
            .runtime
            .lock()
            .map_err(|_| bridge_error("The built-in engine readiness state is unavailable."))?;
        let (runtime, timeout) = self
            .ready
            .wait_timeout_while(runtime, ENGINE_STARTUP_TIMEOUT, |runtime| {
                runtime.ready_version.as_deref() != Some(expected_version.as_str())
            })
            .map_err(|_| bridge_error("The built-in engine readiness wait failed."))?;
        if timeout.timed_out()
            || runtime.ready_version.as_deref() != Some(expected_version.as_str())
        {
            return Err(provider_error(
                FileConversionErrorCode::Timeout,
                "The built-in engine did not become ready in time.",
                true,
                None,
            ));
        }
        Ok(())
    }

    fn wait_for_completion(
        &self,
        wait: CompletionWait<'_>,
    ) -> Result<FileEngineCompletion, FileConversionError> {
        loop {
            match completion_wait_state(wait.cancellation, wait.deadline) {
                CompletionWaitState::Cancelled => {
                    let _ = wait.app.emit_to(
                        FILE_ENGINE_LABEL,
                        FILE_ENGINE_CANCEL_EVENT,
                        FileEngineCancelRequest {
                            protocol_version: FILE_ENGINE_PROTOCOL_VERSION,
                            token: wait.token.into(),
                            job_id: wait.job_id.into(),
                        },
                    );
                    return Err(provider_error(
                        FileConversionErrorCode::Cancelled,
                        "The conversion was cancelled.",
                        true,
                        None,
                    ));
                }
                CompletionWaitState::TimedOut => {
                    return Err(provider_error(
                        FileConversionErrorCode::Timeout,
                        "The built-in engine exceeded the conversion deadline.",
                        true,
                        None,
                    ));
                }
                CompletionWaitState::Continue => {}
            }
            match wait.receiver.recv_timeout(ENGINE_COMPLETION_POLL_INTERVAL) {
                Ok(EngineMessage::Progress(update)) => wait.progress.report(update)?,
                Ok(EngineMessage::Completed(completion)) => return Ok(completion),
                Ok(EngineMessage::Failed(error)) => return Err(error),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(bridge_error("The built-in engine stopped unexpectedly."));
                }
            }
        }
    }

    fn revoke(&self, token: &str) {
        if let Ok(mut runtime) = self.runtime.lock() {
            runtime.sessions.remove(token);
        }
    }

    fn fail_all_sessions(&self, message: &str) {
        if let Ok(mut runtime) = self.runtime.lock() {
            runtime.ready_version = None;
            for (_, session) in runtime.sessions.drain() {
                let _ = session
                    .sender
                    .send(EngineMessage::Failed(bridge_error(message)));
            }
        }
    }

    fn validate_session<'a>(
        runtime: &'a EngineRuntime,
        token: &str,
        job_id: Option<&str>,
        engine_version: Option<&str>,
    ) -> Result<&'a EngineSession, String> {
        let session = runtime
            .sessions
            .get(token)
            .ok_or_else(|| "The File engine capability token is invalid or expired.".to_string())?;
        if job_id.is_some_and(|job_id| job_id != session.job_id) {
            return Err("The File engine job binding does not match.".into());
        }
        if engine_version.is_some_and(|version| version != session.engine_version) {
            return Err("The File engine version binding does not match.".into());
        }
        if Instant::now() >= session.deadline {
            return Err("The File engine capability token has expired.".into());
        }
        Ok(session)
    }
}

#[tauri::command]
pub fn file_engine_ready(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, FileEngineBridgeState>,
    request: FileEngineReadyRequest,
) -> Result<(), String> {
    require_engine_window(&window)?;
    let trusted_version = window
        .app_handle()
        .state::<PluginRegistryState>()
        .with_registry(|registry| {
            registry
                .engine_asset_root(FILE_ENGINE_PLUGIN_ID, &request.engine_version)
                .map(|_| true)
        })
        .unwrap_or(false)
        || development_assets_enabled() && request.engine_version == FILE_ENGINE_VERSION;
    if request.protocol_version != FILE_ENGINE_PROTOCOL_VERSION
        || request.plugin_id != FILE_ENGINE_PLUGIN_ID
        || !trusted_version
    {
        return Err("The File engine protocol or identity is incompatible.".into());
    }
    let mut runtime = state
        .bridge
        .runtime
        .lock()
        .map_err(|_| "The File engine readiness state is unavailable.".to_string())?;
    runtime.ready_version = Some(request.engine_version);
    state.bridge.ready.notify_all();
    Ok(())
}

#[tauri::command]
pub fn file_engine_read_input(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, FileEngineBridgeState>,
    request: tauri::ipc::Request<'_>,
) -> Result<tauri::ipc::Response, String> {
    require_engine_window(&window)?;
    let token = token_header(&request)?;
    let runtime = state
        .bridge
        .runtime
        .lock()
        .map_err(|_| "The File engine session store is unavailable.".to_string())?;
    let session = FileEngineBridge::validate_session(
        &runtime,
        token,
        job_header(&request),
        Some(engine_header(&request)?),
    )?;
    let metadata = fs::metadata(&session.input_path)
        .map_err(|_| "The staged File engine input is missing.".to_string())?;
    if metadata.len() == 0 || metadata.len() > MAX_INPUT_BYTES {
        return Err("The staged File engine input size is invalid.".into());
    }
    let bytes = fs::read(&session.input_path)
        .map_err(|_| "The staged File engine input could not be read.".to_string())?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[tauri::command]
pub fn file_engine_write_output(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, FileEngineBridgeState>,
    request: tauri::ipc::Request<'_>,
) -> Result<(), String> {
    require_engine_window(&window)?;
    let token = token_header(&request)?;
    let bytes = match request.body() {
        tauri::ipc::InvokeBody::Raw(bytes) => bytes,
        tauri::ipc::InvokeBody::Json(_) => {
            return Err("The File engine output must use raw binary IPC.".into())
        }
    };
    if bytes.is_empty() || bytes.len() > MAX_OUTPUT_BYTES {
        return Err("The File engine output size is invalid.".into());
    }
    let runtime = state
        .bridge
        .runtime
        .lock()
        .map_err(|_| "The File engine session store is unavailable.".to_string())?;
    let session = FileEngineBridge::validate_session(
        &runtime,
        token,
        job_header(&request),
        Some(engine_header(&request)?),
    )?;
    if session.direction != FileConversionDirection::PdfToDocx {
        return Err("Raw engine output is not valid for this conversion direction.".into());
    }
    fs::write(&session.output_path, bytes)
        .map_err(|_| "The staged File engine output could not be written.".to_string())
}

#[tauri::command]
pub fn file_engine_progress(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, FileEngineBridgeState>,
    request: FileEngineProgressRequest,
) -> Result<(), String> {
    require_engine_window(&window)?;
    validate_protocol(request.protocol_version)?;
    let runtime = state
        .bridge
        .runtime
        .lock()
        .map_err(|_| "The File engine session store is unavailable.".to_string())?;
    let session = FileEngineBridge::validate_session(
        &runtime,
        &request.token,
        Some(&request.job_id),
        Some(&request.engine_version),
    )?;
    let stage = match request.stage {
        FileEngineProgressStage::Packaging | FileEngineProgressStage::Printing => {
            FileConversionStage::Finalizing
        }
        FileEngineProgressStage::Parsing
        | FileEngineProgressStage::Analyzing
        | FileEngineProgressStage::Rendering => FileConversionStage::Converting,
    };
    let progress =
        request
            .percent
            .map_or(FileConversionProgress::Indeterminate { stage }, |percent| {
                FileConversionProgress::Percentage {
                    stage,
                    percent: percent.min(100),
                }
            });
    session
        .sender
        .send(EngineMessage::Progress(progress))
        .map_err(|_| "The File conversion job is no longer listening.".to_string())
}

#[tauri::command]
pub fn file_engine_complete(
    window: tauri::WebviewWindow,
    state: tauri::State<'_, FileEngineBridgeState>,
    request: FileEngineCompletionRequest,
) -> Result<(), String> {
    require_engine_window(&window)?;
    validate_protocol(request.protocol_version)?;
    let session = {
        let mut runtime = state
            .bridge
            .runtime
            .lock()
            .map_err(|_| "The File engine session store is unavailable.".to_string())?;
        FileEngineBridge::validate_session(
            &runtime,
            &request.token,
            Some(&request.job_id),
            Some(&request.engine_version),
        )?;
        runtime
            .sessions
            .remove(&request.token)
            .expect("validated session")
    };
    let message = completion_message(&session, request)?;
    session
        .sender
        .send(message)
        .map_err(|_| "The File conversion job is no longer listening.".to_string())
}

fn completion_message(
    session: &EngineSession,
    request: FileEngineCompletionRequest,
) -> Result<EngineMessage, String> {
    Ok(match request.status {
        FileEngineCompletionStatus::Completed => {
            let profile = request
                .quality_profile
                .ok_or_else(|| "Completed engine output requires a quality profile.".to_string())?;
            if !session.output_path.is_file() {
                return Err("The completed File engine output is missing.".into());
            }
            EngineMessage::Completed(FileEngineCompletion {
                quality_profile: profile,
                warning_keys: bounded_warning_keys(request.warning_keys)?,
                page_count: request.page_count,
            })
        }
        FileEngineCompletionStatus::Cancelled => EngineMessage::Failed(provider_error(
            FileConversionErrorCode::Cancelled,
            "The conversion was cancelled.",
            true,
            None,
        )),
        FileEngineCompletionStatus::Failed => {
            let code = map_engine_error(
                request
                    .error_code
                    .unwrap_or(FileEngineErrorCode::ProviderFailed),
            );
            let mut error = provider_error(
                code,
                "The built-in File engine could not convert this document.",
                true,
                None,
            );
            error.diagnostic = request.diagnostic.map(|value| bounded_string(&value, 512));
            EngineMessage::Failed(error)
        }
    })
}

#[tauri::command]
pub async fn file_engine_print_rendered(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    state: tauri::State<'_, FileEngineBridgeState>,
    request: FileEngineRenderMeasurement,
) -> Result<(), String> {
    require_engine_window(&window)?;
    validate_protocol(request.protocol_version)?;
    validate_render_measurement(&request)?;
    let output_path = {
        let runtime = state
            .bridge
            .runtime
            .lock()
            .map_err(|_| "The File engine session store is unavailable.".to_string())?;
        let session = FileEngineBridge::validate_session(
            &runtime,
            &request.token,
            Some(&request.job_id),
            Some(&request.engine_version),
        )?;
        if session.direction != FileConversionDirection::DocxToPdf {
            return Err("Native printing is not valid for this conversion direction.".into());
        }
        session.output_path.clone()
    };
    let page_rects = request.page_rects;
    tauri::async_runtime::spawn_blocking(move || {
        super::native_print::print_engine_webview_to_pdf(
            &app,
            &output_path,
            &page_rects,
            Duration::from_secs(30),
        )
    })
    .await
    .map_err(|_| "The native print worker stopped unexpectedly.".to_string())?
}

fn validate_render_measurement(request: &FileEngineRenderMeasurement) -> Result<(), String> {
    if request.section_count == 0
        || request.section_count > 512
        || request.measured_page_count == 0
        || request.page_rects.len() != request.measured_page_count as usize
        || request.page_rects.len() > 512
        || request.page_rects.iter().any(|rect| {
            !rect.x.is_finite()
                || !rect.y.is_finite()
                || !rect.width.is_finite()
                || !rect.height.is_finite()
                || rect.x < 0.0
                || rect.y < 0.0
                || rect.x > 1_000_000.0
                || rect.y > 1_000_000.0
                || rect.width <= 0.0
                || rect.height <= 0.0
                || rect.width > 20_000.0
                || rect.height > 20_000.0
        })
    {
        return Err("The rendered document did not contain printable pages.".into());
    }
    Ok(())
}

fn stage_validated_input(source: &Path, destination: &Path) -> Result<(), FileConversionError> {
    let metadata = fs::metadata(source).map_err(|_| {
        provider_error(
            FileConversionErrorCode::InvalidInput,
            "The validated source is no longer readable.",
            false,
            None,
        )
    })?;
    if !metadata.is_file() || metadata.len() == 0 || metadata.len() > MAX_INPUT_BYTES {
        return Err(provider_error(
            FileConversionErrorCode::InvalidInput,
            "The validated source size is outside the built-in engine limit.",
            false,
            None,
        ));
    }
    fs::copy(source, destination).map(|_| ()).map_err(|_| {
        provider_error(
            FileConversionErrorCode::PermissionDenied,
            "The source could not be copied into Zero's private engine workspace.",
            true,
            None,
        )
    })
}

fn random_token() -> Result<String, FileConversionError> {
    let mut bytes = [0_u8; 32];
    fill(&mut bytes)
        .map_err(|_| bridge_error("A secure File engine capability token could not be created."))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

fn require_engine_window(window: &tauri::WebviewWindow) -> Result<(), String> {
    (window.label() == FILE_ENGINE_LABEL)
        .then_some(())
        .ok_or_else(|| "This command is restricted to the isolated Zero File engine.".into())
}

fn token_header<'a>(request: &'a tauri::ipc::Request<'_>) -> Result<&'a str, String> {
    request
        .headers()
        .get("x-zero-file-token")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty() && value.len() <= 128)
        .ok_or_else(|| "The File engine capability token header is missing.".into())
}

fn job_header<'a>(request: &'a tauri::ipc::Request<'_>) -> Option<&'a str> {
    request
        .headers()
        .get("x-zero-file-job")
        .and_then(|value| value.to_str().ok())
}

fn engine_header<'a>(request: &'a tauri::ipc::Request<'_>) -> Result<&'a str, String> {
    request
        .headers()
        .get("x-zero-file-engine")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty() && value.len() <= 64)
        .ok_or_else(|| "The File engine version header is missing.".into())
}

fn acquire_engine(app: &tauri::AppHandle) -> Result<LeasedEngine, FileConversionError> {
    match app
        .state::<PluginRegistryState>()
        .with_registry(|registry| registry.acquire_active_engine(FILE_ENGINE_PLUGIN_ID))
    {
        Ok(engine) => Ok(LeasedEngine {
            engine,
            installed: true,
        }),
        Err(_error) if development_assets_enabled() => Ok(LeasedEngine {
            engine: ActivePluginEngine {
                plugin_id: FILE_ENGINE_PLUGIN_ID.into(),
                package_version: FILE_ENGINE_VERSION.into(),
                engine_root: PathBuf::new(),
            },
            installed: false,
        }),
        Err(error) => {
            let mut failure = provider_error(
                FileConversionErrorCode::EngineUnavailable,
                "The signed Zero File engine package is not installed or ready.",
                true,
                None,
            );
            failure.diagnostic = Some(bounded_string(&error, 512));
            Err(failure)
        }
    }
}

fn release_engine(app: &tauri::AppHandle, lease: &LeasedEngine) {
    if lease.installed {
        let _ = app
            .state::<PluginRegistryState>()
            .with_registry(|registry| {
                registry.release_engine(&lease.engine.plugin_id, &lease.engine.package_version)
            });
    }
}

pub fn development_assets_enabled() -> bool {
    cfg!(debug_assertions)
        && std::env::var("ZERO_FILE_ENGINE_DEV_ASSETS").ok().as_deref() == Some("1")
}

fn validate_protocol(version: u16) -> Result<(), String> {
    (version == FILE_ENGINE_PROTOCOL_VERSION)
        .then_some(())
        .ok_or_else(|| "The File engine protocol version is incompatible.".into())
}

fn completion_wait_state(
    cancellation: &FileConversionCancellationToken,
    deadline: Instant,
) -> CompletionWaitState {
    if cancellation.is_cancelled() {
        CompletionWaitState::Cancelled
    } else if Instant::now() >= deadline {
        CompletionWaitState::TimedOut
    } else {
        CompletionWaitState::Continue
    }
}

fn bounded_warning_keys(keys: Vec<String>) -> Result<Vec<String>, String> {
    if keys.len() > 8
        || keys
            .iter()
            .any(|key| key.len() > 96 || !key.starts_with("file."))
    {
        return Err("The File engine warning metadata is invalid.".into());
    }
    Ok(keys)
}

fn bounded_string(value: &str, limit: usize) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .take(limit)
        .collect()
}

fn map_engine_error(code: FileEngineErrorCode) -> FileConversionErrorCode {
    match code {
        FileEngineErrorCode::PasswordRequired => FileConversionErrorCode::PasswordRequired,
        FileEngineErrorCode::InvalidInput => FileConversionErrorCode::InvalidInput,
        FileEngineErrorCode::UnsupportedInput => FileConversionErrorCode::UnsupportedInput,
        FileEngineErrorCode::Timeout => FileConversionErrorCode::Timeout,
        FileEngineErrorCode::Cancelled => FileConversionErrorCode::Cancelled,
        FileEngineErrorCode::ProviderFailed => FileConversionErrorCode::ProviderFailed,
        FileEngineErrorCode::InvalidProviderOutput => {
            FileConversionErrorCode::InvalidProviderOutput
        }
    }
}

fn bridge_error(message: &str) -> FileConversionError {
    provider_error(FileConversionErrorCode::ProviderFailed, message, true, None)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_session(token: &str, deadline: Instant) -> (EngineSession, Receiver<EngineMessage>) {
        let (sender, receiver) = mpsc::channel();
        let root = std::env::temp_dir().join(format!("zero-engine-bridge-{token}"));
        (
            EngineSession {
                job_id: format!("job-{token}"),
                engine_version: FILE_ENGINE_VERSION.into(),
                direction: FileConversionDirection::PdfToDocx,
                input_path: root.join("input.pdf"),
                output_path: root.join("provider-output.docx"),
                deadline,
                sender,
            },
            receiver,
        )
    }

    fn completed_request(token: &str) -> FileEngineCompletionRequest {
        FileEngineCompletionRequest {
            protocol_version: FILE_ENGINE_PROTOCOL_VERSION,
            token: token.into(),
            engine_version: FILE_ENGINE_VERSION.into(),
            job_id: format!("job-{token}"),
            status: FileEngineCompletionStatus::Completed,
            quality_profile: Some(FileConversionQualityProfile::EditableReconstruction),
            warning_keys: vec!["file.quality.editableReconstructionWarning".into()],
            page_count: Some(1),
            error_code: None,
            diagnostic: None,
        }
    }

    fn valid_measurement() -> FileEngineRenderMeasurement {
        FileEngineRenderMeasurement {
            protocol_version: FILE_ENGINE_PROTOCOL_VERSION,
            token: "token".into(),
            engine_version: FILE_ENGINE_VERSION.into(),
            job_id: "job".into(),
            section_count: 1,
            measured_page_count: 1,
            page_rects: vec![FileEnginePageRect {
                x: 0.0,
                y: 0.0,
                width: 612.0,
                height: 792.0,
            }],
        }
    }

    #[test]
    fn render_measurement_rejects_unknown_fields_and_invalid_page_counts() {
        let malformed = serde_json::json!({
            "protocolVersion": 1,
            "token": "token",
            "engineVersion": "1.0.0",
            "jobId": "job",
            "sectionCount": 1,
            "measuredPageCount": 1,
            "pageRects": [{"x": 0, "y": 0, "width": 612, "height": 792}],
            "unexpected": true,
        });
        assert!(serde_json::from_value::<FileEngineRenderMeasurement>(malformed).is_err());

        let mut measurement = valid_measurement();
        measurement.measured_page_count = 0;
        assert!(validate_render_measurement(&measurement).is_err());
        measurement.measured_page_count = 513;
        measurement.page_rects = vec![measurement.page_rects[0].clone(); 513];
        assert!(validate_render_measurement(&measurement).is_err());
    }

    #[test]
    fn render_measurement_rejects_non_finite_negative_and_out_of_range_geometry() {
        for rect in [
            FileEnginePageRect {
                x: f64::NAN,
                y: 0.0,
                width: 612.0,
                height: 792.0,
            },
            FileEnginePageRect {
                x: -1.0,
                y: 0.0,
                width: 612.0,
                height: 792.0,
            },
            FileEnginePageRect {
                x: 0.0,
                y: 1_000_001.0,
                width: 612.0,
                height: 792.0,
            },
            FileEnginePageRect {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 792.0,
            },
            FileEnginePageRect {
                x: 0.0,
                y: 0.0,
                width: 612.0,
                height: 20_001.0,
            },
        ] {
            let mut measurement = valid_measurement();
            measurement.page_rects = vec![rect];
            assert!(validate_render_measurement(&measurement).is_err());
        }
    }

    #[test]
    fn render_measurement_accepts_multi_page_portrait_and_landscape_sections() {
        let measurement = FileEngineRenderMeasurement {
            protocol_version: FILE_ENGINE_PROTOCOL_VERSION,
            token: "token".into(),
            engine_version: FILE_ENGINE_VERSION.into(),
            job_id: "job".into(),
            section_count: 2,
            measured_page_count: 2,
            page_rects: vec![
                FileEnginePageRect {
                    x: 0.0,
                    y: 0.0,
                    width: 612.0,
                    height: 792.0,
                },
                FileEnginePageRect {
                    x: 0.0,
                    y: 792.0,
                    width: 792.0,
                    height: 612.0,
                },
            ],
        };
        assert!(validate_render_measurement(&measurement).is_ok());
    }

    #[test]
    fn session_binding_rejects_wrong_job_version_and_expired_tokens() {
        let (session, _receiver) =
            test_session("current", Instant::now() + Duration::from_secs(10));
        let mut runtime = EngineRuntime::default();
        runtime.sessions.insert("current".into(), session);

        assert!(FileEngineBridge::validate_session(
            &runtime,
            "current",
            Some("wrong-job"),
            Some(FILE_ENGINE_VERSION),
        )
        .is_err());
        assert!(FileEngineBridge::validate_session(
            &runtime,
            "current",
            Some("job-current"),
            Some("0.0.0"),
        )
        .is_err());

        let (expired, _receiver) =
            test_session("expired", Instant::now() - Duration::from_millis(1));
        runtime.sessions.insert("expired".into(), expired);
        assert!(FileEngineBridge::validate_session(
            &runtime,
            "expired",
            Some("job-expired"),
            Some(FILE_ENGINE_VERSION),
        )
        .is_err());
    }

    #[test]
    fn malformed_completion_cannot_claim_success_or_unbounded_metadata() {
        let malformed = serde_json::json!({
            "protocolVersion": 1,
            "token": "token",
            "engineVersion": "1.0.0",
            "jobId": "job-token",
            "status": "completed",
            "qualityProfile": "editableReconstruction",
            "warningKeys": [],
            "pageCount": 1,
            "unexpected": true,
        });
        assert!(serde_json::from_value::<FileEngineCompletionRequest>(malformed).is_err());

        let (session, _receiver) = test_session("token", Instant::now() + Duration::from_secs(10));
        let mut missing_profile = completed_request("token");
        missing_profile.quality_profile = None;
        assert!(completion_message(&session, missing_profile).is_err());

        let mut invalid_warning = completed_request("token");
        invalid_warning.warning_keys = vec!["not-a-file-warning".into()];
        fs::create_dir_all(session.output_path.parent().unwrap()).unwrap();
        fs::write(&session.output_path, b"output").unwrap();
        assert!(completion_message(&session, invalid_warning).is_err());
        fs::remove_dir_all(session.output_path.parent().unwrap()).unwrap();
    }

    #[test]
    fn cancellation_timeout_and_webview_crash_are_terminal_and_bounded() {
        let cancellation = FileConversionCancellationToken::default();
        assert_eq!(
            completion_wait_state(&cancellation, Instant::now() + Duration::from_secs(1)),
            CompletionWaitState::Continue
        );
        cancellation.cancel();
        assert_eq!(
            completion_wait_state(&cancellation, Instant::now() + Duration::from_secs(1)),
            CompletionWaitState::Cancelled
        );
        assert!(ENGINE_COMPLETION_POLL_INTERVAL <= Duration::from_secs(2));
        assert_eq!(
            completion_wait_state(
                &FileConversionCancellationToken::default(),
                Instant::now() - Duration::from_millis(1),
            ),
            CompletionWaitState::TimedOut
        );

        let bridge = FileEngineBridge::default();
        let (first, first_receiver) =
            test_session("first", Instant::now() + Duration::from_secs(10));
        let (second, second_receiver) =
            test_session("second", Instant::now() + Duration::from_secs(10));
        {
            let mut runtime = bridge.runtime.lock().unwrap();
            runtime.ready_version = Some(FILE_ENGINE_VERSION.into());
            runtime.sessions.insert("first".into(), first);
            runtime.sessions.insert("second".into(), second);
        }
        bridge.fail_all_sessions("simulated WebView crash");
        for receiver in [first_receiver, second_receiver] {
            match receiver.recv().expect("crash must wake every job") {
                EngineMessage::Failed(error) => {
                    assert_eq!(error.code, FileConversionErrorCode::ProviderFailed)
                }
                _ => panic!("crash must be reported as a provider failure"),
            }
        }
        let runtime = bridge.runtime.lock().unwrap();
        assert!(runtime.ready_version.is_none());
        assert!(runtime.sessions.is_empty());
    }
}

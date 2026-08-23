use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Cursor,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Mutex,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use getrandom::fill;
#[cfg(target_os = "macos")]
use objc::{
    class, msg_send,
    runtime::{Object, BOOL, YES},
    sel, sel_impl,
};
#[cfg(target_os = "macos")]
use objc2_app_kit::{
    NSScreenSaverWindowLevel, NSWindow, NSWindowAnimationBehavior, NSWindowCollectionBehavior,
    NSWindowSharingType,
};
use serde::{Deserialize, Serialize};
use tauri::{Manager, WebviewUrl};

use crate::services::surface_activity::{close_surface, hide_surface, show_surface};

#[cfg(target_os = "macos")]
mod capture_targets;

const CAPTURE_WINDOW_LABEL: &str = "capture";
const TRAY_WINDOW_LABEL: &str = "tray";
const MAIN_WINDOW_LABEL: &str = "main";
const PIN_WINDOW_LABEL: &str = "pin";
pub const DEFAULT_SAVE_FILE_NAME: &str = "zero-snap.png";
const PIN_TITLEBAR_HEIGHT: f64 = 30.0;
const SCREENSHOT_SESSION_TTL_MS: u64 = 15 * 60 * 1000;
const SCREENSHOT_UPLOAD_LEASE_TTL_MS: u64 = 30 * 1000;
const SCREENSHOT_REVEAL_TIMEOUT: Duration = Duration::from_secs(15);
pub const MAX_SCREENSHOT_PNG_BYTES: usize = 100 * 1024 * 1024;
const MAX_SCREENSHOT_PINS: usize = 16;
const MAX_SCREENSHOT_DIMENSION: u32 = 32_768;
const MAX_SCREENSHOT_PIXELS: u64 = 268_435_456;
const MAX_SCREENSHOT_UPLOAD_LEASES: usize = 4;
static LAST_SESSION_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize)]
pub struct ScreenshotCapabilities {
    pub platform: String,
    pub selection_visual: bool,
    pub custom_overlay: bool,
    pub system_launcher: bool,
    pub active_actions: Vec<String>,
    pub pending_tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScreenshotStartResult {
    pub mode: String,
    pub platform: String,
    pub action: String,
    pub message: String,
    pub session_id: Option<String>,
    pub capture_window_label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSessionPayload {
    pub session_id: String,
    pub initial_action: String,
    pub media: ScreenshotMediaDescriptor,
    pub targets: Vec<ScreenshotTargetCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotSourceBounds {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ScreenshotTargetKind {
    Window,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotTargetCandidate {
    pub id: String,
    pub kind: ScreenshotTargetKind,
    pub bounds: ScreenshotSourceBounds,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotCommitResult {
    pub copied: bool,
    pub saved_path: Option<String>,
    pub pin_window_label: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScreenshotCancelResult {
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PinPayload {
    pub media: ScreenshotMediaDescriptor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenshotMediaDescriptor {
    pub token: String,
    pub mime_type: String,
    pub byte_length: u64,
    pub width: u32,
    pub height: u32,
    pub expires_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScreenshotMediaInput {
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrepareScreenshotCommitInput {
    pub session_id: String,
    pub action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotUploadLease {
    pub token: String,
    pub session_id: String,
    pub action: String,
    pub max_bytes: u64,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone)]
struct ScreenshotSession {
    id: String,
    initial_action: String,
    media: ScreenshotMedia,
    targets: Vec<ScreenshotTargetCandidate>,
    revealed: bool,
    expires_at_ms: u64,
    restore_window_label: Option<String>,
}

#[derive(Debug, Clone)]
struct ScreenshotMedia {
    token: String,
    path: PathBuf,
    byte_length: u64,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone)]
struct ScreenshotCommitLease {
    token: String,
    session_id: String,
    action: String,
    save_path: Option<PathBuf>,
    expires_at_ms: u64,
}

#[derive(Debug, Clone)]
struct PinResource {
    media: ScreenshotMedia,
}

#[derive(Debug, Default)]
pub struct ScreenshotSessionStore {
    active: Mutex<Option<ScreenshotSession>>,
    leases: Mutex<HashMap<String, ScreenshotCommitLease>>,
    pins: Mutex<HashMap<String, PinResource>>,
}

impl ScreenshotSessionStore {
    fn set_active(&self, session: ScreenshotSession) {
        let previous = self
            .active
            .lock()
            .ok()
            .and_then(|mut active| active.replace(session));
        if let Some(previous) = previous {
            self.remove_session_leases(&previous.id);
            remove_owned_media(&previous.media.path);
        }
    }

    fn active(&self) -> Option<ScreenshotSession> {
        self.active.lock().ok().and_then(|active| active.clone())
    }

    fn claim_reveal(&self, session_id: &str) -> Result<bool, ScreenshotError> {
        let mut active = self.active.lock().map_err(|_| {
            screenshot_error(
                "screenshot.session_store",
                "截图会话暂不可用，请重新截图",
                true,
            )
        })?;
        let session = active.as_mut().ok_or_else(|| {
            screenshot_error(
                "screenshot.session_missing",
                "截图会话不存在或已结束",
                false,
            )
        })?;
        if session.id != session_id {
            return Err(screenshot_error(
                "screenshot.session_scope",
                "截图会话已更新，请重新开始截图",
                false,
            ));
        }
        if session.revealed {
            return Ok(false);
        }
        session.revealed = true;
        Ok(true)
    }

    fn is_awaiting_reveal(&self, session_id: &str) -> bool {
        self.active.lock().is_ok_and(|active| {
            active
                .as_ref()
                .is_some_and(|session| session.id == session_id && !session.revealed)
        })
    }

    fn clear_active(&self) {
        let previous = self.active.lock().ok().and_then(|mut active| active.take());
        if let Some(previous) = previous {
            self.remove_session_leases(&previous.id);
            remove_owned_media(&previous.media.path);
        }
    }

    fn clear_active_if(&self, session_id: &str) {
        let previous = self.active.lock().ok().and_then(|mut active| {
            if active
                .as_ref()
                .is_some_and(|session| session.id == session_id)
            {
                active.take()
            } else {
                None
            }
        });
        if let Some(previous) = previous {
            self.remove_session_leases(&previous.id);
            remove_owned_media(&previous.media.path);
        }
    }

    fn set_pin(&self, label: String, pin: PinResource) -> Result<(), ScreenshotError> {
        let mut pins = self
            .pins
            .lock()
            .map_err(|_| screenshot_error("screenshot.pin_store", "钉图资源暂不可用", true))?;
        if !pins.contains_key(&label) && pins.len() >= MAX_SCREENSHOT_PINS {
            return Err(screenshot_error(
                "screenshot.pin_limit",
                "钉图窗口数量已达到上限，请先关闭一个钉图",
                true,
            ));
        }
        let previous = pins.insert(label, pin);
        drop(pins);
        if let Some(previous) = previous {
            remove_owned_media(&previous.media.path);
        }
        Ok(())
    }

    fn pin(&self, label: &str) -> Option<PinResource> {
        self.pins
            .lock()
            .ok()
            .and_then(|pins| pins.get(label).cloned())
    }

    pub fn remove_pin(&self, label: &str) {
        let pin = self
            .pins
            .lock()
            .ok()
            .and_then(|mut pins| pins.remove(label));
        if let Some(pin) = pin {
            remove_owned_media(&pin.media.path);
        }
    }

    fn insert_lease(&self, lease: ScreenshotCommitLease) -> Result<(), ScreenshotError> {
        let mut leases = self.leases.lock().map_err(|_| {
            screenshot_error("screenshot.lease_store", "截图提交凭证暂不可用", true)
        })?;
        let now = now_ms();
        leases.retain(|_, lease| lease.expires_at_ms > now);
        if leases.len() >= MAX_SCREENSHOT_UPLOAD_LEASES {
            return Err(screenshot_error(
                "screenshot.lease_limit",
                "截图提交请求过多，请稍后重试",
                true,
            ));
        }
        leases.insert(lease.token.clone(), lease);
        Ok(())
    }

    fn take_lease(&self, token: &str) -> Option<ScreenshotCommitLease> {
        self.leases
            .lock()
            .ok()
            .and_then(|mut leases| leases.remove(token))
    }

    fn remove_session_leases(&self, session_id: &str) {
        if let Ok(mut leases) = self.leases.lock() {
            leases.retain(|_, lease| lease.session_id != session_id);
        }
    }

    fn resolve_media(&self, token: &str, window_label: &str) -> Option<ScreenshotMedia> {
        if window_label == CAPTURE_WINDOW_LABEL {
            let active = self.active()?;
            if active.expires_at_ms <= now_ms() {
                self.clear_active_if(&active.id);
                return None;
            }
            return (active.media.token == token).then_some(active.media);
        }
        self.pin(window_label)
            .and_then(|pin| (pin.media.token == token).then_some(pin.media))
    }

    fn owned_directories(&self) -> HashSet<PathBuf> {
        let mut directories = HashSet::new();
        if let Some(session) = self.active() {
            if let Some(parent) = session.media.path.parent() {
                directories.insert(parent.to_path_buf());
            }
        }
        if let Ok(pins) = self.pins.lock() {
            directories.extend(
                pins.values()
                    .filter_map(|pin| pin.media.path.parent().map(Path::to_path_buf)),
            );
        }
        directories
    }

    pub fn cleanup_all(&self) {
        self.clear_active();
        let pins = self
            .pins
            .lock()
            .ok()
            .map(|mut pins| pins.drain().map(|(_, pin)| pin).collect::<Vec<_>>())
            .unwrap_or_default();
        for pin in pins {
            remove_owned_media(&pin.media.path);
        }
        if let Ok(mut leases) = self.leases.lock() {
            leases.clear();
        }
    }

    #[cfg(test)]
    fn resource_counts(&self) -> (usize, usize, usize) {
        (
            usize::from(self.active().is_some()),
            self.leases.lock().map(|leases| leases.len()).unwrap_or(0),
            self.pins.lock().map(|pins| pins.len()).unwrap_or(0),
        )
    }
}

pub fn screenshot_capabilities() -> ScreenshotCapabilities {
    ScreenshotCapabilities {
        platform: platform_name().into(),
        selection_visual: true,
        custom_overlay: cfg!(target_os = "macos"),
        system_launcher: cfg!(any(target_os = "macos", target_os = "windows")),
        active_actions: vec!["copy".into(), "save".into(), "cancel".into()],
        pending_tools: vec![
            "rectangle".into(),
            "ellipse".into(),
            "arrow".into(),
            "pen".into(),
            "mosaic".into(),
            "text".into(),
            "pin".into(),
        ],
    }
}

pub fn start_screenshot_session(
    app: tauri::AppHandle,
    action: String,
) -> Result<ScreenshotStartResult, ScreenshotError> {
    let normalized_action = normalize_start_action(action);
    let restore_window_label = hide_visible_shell_windows(&app);

    #[cfg(target_os = "macos")]
    {
        let store = app.state::<ScreenshotSessionStore>();
        let prepared = (|| {
            let root = screenshot_media_root(&app)?;
            cleanup_stale_screenshot_media(&root, &store);
            let session_id = random_resource_token("session")?;
            let media_token = random_resource_token("media")?;
            let created_at_ms = now_ms();
            let session_directory = root.join(format!("session-{created_at_ms}-{session_id}"));
            create_owner_only_directory(&session_directory)?;
            let media_path = session_directory.join("capture.png");
            let target_snapshot = capture_targets::prepare_capture_target_snapshot()
                .inspect_err(|error| eprintln!("Zero Snap window targeting unavailable: {error}"))
                .ok();
            let capture = capture_fullscreen_png(&media_path).inspect_err(|_error| {
                let _ = fs::remove_dir_all(&session_directory);
            })?;
            Ok::<_, ScreenshotError>((
                session_id,
                media_token,
                created_at_ms,
                media_path,
                capture,
                target_snapshot,
            ))
        })();
        let (session_id, media_token, created_at_ms, media_path, capture, target_snapshot) =
            match prepared {
                Ok(prepared) => prepared,
                Err(error) => {
                    restore_shell_window(&app, restore_window_label.as_deref());
                    return Err(error);
                }
            };
        let (byte_length, width, height) = capture;
        let targets = target_snapshot
            .as_ref()
            .map(|snapshot| {
                capture_targets::resolve_capture_targets(
                    snapshot,
                    width,
                    height,
                    std::process::id(),
                )
            })
            .unwrap_or_default();
        let restore_window_label_for_error = restore_window_label.clone();
        store.set_active(ScreenshotSession {
            id: session_id.clone(),
            initial_action: normalized_action.clone(),
            media: ScreenshotMedia {
                token: media_token,
                path: media_path,
                byte_length,
                width,
                height,
            },
            targets,
            revealed: false,
            expires_at_ms: created_at_ms.saturating_add(SCREENSHOT_SESSION_TTL_MS),
            restore_window_label,
        });
        if let Err(error) = open_capture_window(&app, &session_id) {
            store.clear_active_if(&session_id);
            restore_shell_window(&app, restore_window_label_for_error.as_deref());
            return Err(error);
        }
        Ok(ScreenshotStartResult {
            mode: "custom-overlay".into(),
            platform: "macOS".into(),
            action: normalized_action,
            message: "截图编辑器已打开".into(),
            session_id: Some(session_id),
            capture_window_label: Some(CAPTURE_WINDOW_LABEL.into()),
        })
    }

    #[cfg(not(target_os = "macos"))]
    {
        if let Err(error) = launch_system_screenshot(&normalized_action) {
            restore_shell_window(&app, restore_window_label.as_deref());
            return Err(screenshot_error("screenshot.system_launcher", error, true));
        }
        Ok(ScreenshotStartResult {
            mode: "system-launcher".into(),
            platform: platform_name().into(),
            action: normalized_action.clone(),
            message: if normalized_action == "save" {
                "截图工具已打开，完成后会保存图片".into()
            } else {
                "截图工具已打开，完成后会复制到剪贴板".into()
            },
            session_id: None,
            capture_window_label: None,
        })
    }
}

pub fn init_screenshot_session(
    app: tauri::AppHandle,
    window_label: &str,
    session_id: Option<String>,
) -> Result<CaptureSessionPayload, ScreenshotError> {
    require_capture_window(window_label)?;
    let active = validate_session(&app, session_id.as_deref())?;
    Ok(CaptureSessionPayload {
        session_id: active.id,
        initial_action: active.initial_action,
        media: media_descriptor(&active.media, Some(active.expires_at_ms)),
        targets: active.targets,
    })
}

pub fn read_screenshot_media(
    app: tauri::AppHandle,
    window_label: &str,
    input: ScreenshotMediaInput,
) -> Result<Vec<u8>, ScreenshotError> {
    validate_resource_token(&input.token)?;
    let media = app
        .state::<ScreenshotSessionStore>()
        .resolve_media(&input.token, window_label)
        .ok_or_else(|| {
            screenshot_error(
                "screenshot.media_scope",
                "截图资源不存在、已过期或不属于当前窗口",
                false,
            )
        })?;
    validate_owned_media_file(&media)?;
    let bytes = fs::read(&media.path).map_err(|_| {
        screenshot_error(
            "screenshot.media_missing",
            "截图资源已被移除，请重新截图",
            true,
        )
    })?;
    if bytes.len() as u64 != media.byte_length {
        return Err(screenshot_error(
            "screenshot.media_changed",
            "截图资源校验失败，请重新截图",
            false,
        ));
    }
    crate::services::performance::record_media_transfer(&app, "screenshot_read", bytes.len());
    Ok(bytes)
}

pub fn reveal_screenshot_capture(
    app: tauri::AppHandle,
    window_label: &str,
    session_id: String,
) -> Result<(), ScreenshotError> {
    require_capture_window(window_label)?;
    validate_session(&app, Some(&session_id))?;
    let store = app.state::<ScreenshotSessionStore>();
    if !store.claim_reveal(&session_id)? {
        return Ok(());
    }

    let capture_window = app.get_webview_window(CAPTURE_WINDOW_LABEL).ok_or_else(|| {
        screenshot_error(
            "screenshot.capture_window_missing",
            "截图编辑窗口已关闭，请重新截图",
            true,
        )
    });
    let reveal_result = match capture_window {
        Ok(capture_window) => reveal_capture_window_on_main_thread(capture_window),
        Err(error) => Err(error),
    };
    if let Err(error) = reveal_result {
        cleanup_failed_reveal(&app, &session_id);
        return Err(error);
    }
    Ok(())
}

pub fn prepare_screenshot_commit(
    app: tauri::AppHandle,
    window_label: &str,
    input: PrepareScreenshotCommitInput,
    resolved_save_path: Option<PathBuf>,
) -> Result<ScreenshotUploadLease, ScreenshotError> {
    require_capture_window(window_label)?;
    let active = validate_session(&app, Some(&input.session_id))?;
    let action = normalize_commit_action(&input.action)?;
    let save_path =
        match action.as_str() {
            "save" => Some(resolved_save_path.ok_or_else(|| {
                screenshot_error("screenshot.save_cancelled", "已取消保存", false)
            })?),
            _ if resolved_save_path.is_some() => {
                return Err(screenshot_error(
                    "screenshot.unexpected_path",
                    "当前截图动作不接受保存路径",
                    false,
                ));
            }
            _ => None,
        };
    let token = random_resource_token("upload")?;
    let expires_at_ms = now_ms().saturating_add(SCREENSHOT_UPLOAD_LEASE_TTL_MS);
    app.state::<ScreenshotSessionStore>()
        .insert_lease(ScreenshotCommitLease {
            token: token.clone(),
            session_id: active.id.clone(),
            action: action.clone(),
            save_path,
            expires_at_ms,
        })?;
    Ok(ScreenshotUploadLease {
        token,
        session_id: active.id,
        action,
        max_bytes: MAX_SCREENSHOT_PNG_BYTES as u64,
        expires_at_ms,
    })
}

pub fn validate_screenshot_commit_request(
    app: &tauri::AppHandle,
    window_label: &str,
    input: &PrepareScreenshotCommitInput,
) -> Result<(), ScreenshotError> {
    require_capture_window(window_label)?;
    validate_session(app, Some(&input.session_id))?;
    normalize_commit_action(&input.action)?;
    Ok(())
}

pub fn upload_screenshot_commit(
    app: tauri::AppHandle,
    window_label: &str,
    lease_token: &str,
    session_id: Option<&str>,
    action: Option<&str>,
    body: Option<&[u8]>,
) -> Result<ScreenshotCommitResult, ScreenshotError> {
    validate_resource_token(lease_token)?;
    let store = app.state::<ScreenshotSessionStore>();
    let lease = store.take_lease(lease_token).ok_or_else(|| {
        screenshot_error(
            "screenshot.lease_invalid",
            "截图提交凭证不存在或已被使用",
            false,
        )
    })?;
    let bytes = validate_upload_request(&lease, window_label, session_id, action, body, now_ms())?;
    validate_session(&app, Some(&lease.session_id))?;
    let (width, height) = validate_png_bytes(bytes)?;
    crate::services::performance::record_media_transfer(&app, "screenshot_upload", bytes.len());

    match lease.action.as_str() {
        "copy" => {
            copy_png_to_clipboard(bytes)
                .map_err(|message| screenshot_error("screenshot.clipboard", message, true))?;
            finish_capture_session(&app, &lease.session_id);
            Ok(ScreenshotCommitResult {
                copied: true,
                saved_path: None,
                pin_window_label: None,
            })
        }
        "save" => {
            let path = lease.save_path.ok_or_else(|| {
                screenshot_error("screenshot.save_destination", "截图保存位置不可用", false)
            })?;
            write_png_to_path(&path, bytes)?;
            let saved_path = path.to_string_lossy().into_owned();
            finish_capture_session(&app, &lease.session_id);
            Ok(ScreenshotCommitResult {
                copied: false,
                saved_path: Some(saved_path),
                pin_window_label: None,
            })
        }
        "pin" => create_pin_from_upload(&app, bytes, width, height),
        _ => Err(screenshot_error(
            "screenshot.action",
            "不支持的截图动作",
            false,
        )),
    }
}

pub fn cancel_screenshot_session(
    app: tauri::AppHandle,
    window_label: &str,
    session_id: String,
) -> Result<ScreenshotCancelResult, ScreenshotError> {
    require_capture_window(window_label)?;
    let active = validate_session(&app, Some(&session_id))?;
    app.state::<ScreenshotSessionStore>()
        .clear_active_if(&session_id);
    if let Some(capture) = app.get_webview_window(CAPTURE_WINDOW_LABEL) {
        let _ = close_surface(&capture);
    }
    restore_shell_window(&app, active.restore_window_label.as_deref());
    Ok(ScreenshotCancelResult { ok: true })
}

pub fn init_pin_window(
    app: tauri::AppHandle,
    window_label: &str,
) -> Result<PinPayload, ScreenshotError> {
    if !window_label.starts_with("pin-") {
        return Err(screenshot_error(
            "screenshot.pin_window",
            "当前窗口不是有效的钉图窗口",
            false,
        ));
    }
    let pin = app
        .state::<ScreenshotSessionStore>()
        .pin(window_label)
        .ok_or_else(|| {
            screenshot_error("screenshot.pin_missing", "钉图内容不存在或已被释放", false)
        })?;
    validate_owned_media_file(&pin.media)?;
    Ok(PinPayload {
        media: media_descriptor(&pin.media, None),
    })
}

fn create_pin_from_upload(
    app: &tauri::AppHandle,
    bytes: &[u8],
    width: u32,
    height: u32,
) -> Result<ScreenshotCommitResult, ScreenshotError> {
    let root = screenshot_media_root(app)?;
    let token = random_resource_token("pin-media")?;
    let created_at_ms = now_ms();
    let media = stage_pin_media(&root, token, created_at_ms, bytes, width, height)?;
    let label = create_pin_label();
    let store = app.state::<ScreenshotSessionStore>();
    if let Err(error) = store.set_pin(
        label.clone(),
        PinResource {
            media: media.clone(),
        },
    ) {
        remove_owned_media(&media.path);
        return Err(error);
    }
    let pin_window = match tauri::WebviewWindowBuilder::new(app, &label, WebviewUrl::App("".into()))
        .title("Pinned Image")
        .decorations(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(false)
        .inner_size(width as f64, height as f64 + PIN_TITLEBAR_HEIGHT)
        .build()
    {
        Ok(window) => window,
        Err(error) => {
            store.remove_pin(&label);
            return Err(screenshot_error(
                "screenshot.pin_window",
                format!("打开钉图窗口失败: {error}"),
                true,
            ));
        }
    };
    let cleanup_app = app.clone();
    let cleanup_label = label.clone();
    pin_window.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::Destroyed) {
            cleanup_app
                .state::<ScreenshotSessionStore>()
                .remove_pin(&cleanup_label);
        }
    });
    if let Err(error) = show_surface(&pin_window).and_then(|_| pin_window.set_focus()) {
        let _ = close_surface(&pin_window);
        store.remove_pin(&label);
        return Err(screenshot_error(
            "screenshot.pin_show",
            format!("显示钉图窗口失败: {error}"),
            true,
        ));
    }
    Ok(ScreenshotCommitResult {
        copied: false,
        saved_path: None,
        pin_window_label: Some(label),
    })
}

fn finish_capture_session(app: &tauri::AppHandle, session_id: &str) {
    let active = app.state::<ScreenshotSessionStore>().active();
    app.state::<ScreenshotSessionStore>()
        .clear_active_if(session_id);
    if let Some(capture) = app.get_webview_window(CAPTURE_WINDOW_LABEL) {
        let _ = close_surface(&capture);
    }
    if let Some(active) = active.filter(|active| active.id == session_id) {
        restore_shell_window(app, active.restore_window_label.as_deref());
    }
}

fn validate_session(
    app: &tauri::AppHandle,
    requested_session_id: Option<&str>,
) -> Result<ScreenshotSession, ScreenshotError> {
    let store = app.state::<ScreenshotSessionStore>();
    let active = store.active().ok_or_else(|| {
        screenshot_error(
            "screenshot.session_missing",
            "截图会话不存在或已结束",
            false,
        )
    })?;
    if active.expires_at_ms <= now_ms() {
        store.clear_active_if(&active.id);
        return Err(screenshot_error(
            "screenshot.session_expired",
            "截图会话已过期，请重新截图",
            true,
        ));
    }
    if requested_session_id.is_some_and(|requested| requested != active.id) {
        return Err(screenshot_error(
            "screenshot.session_scope",
            "截图会话已更新，请重新开始截图",
            false,
        ));
    }
    Ok(active)
}

fn validate_owned_media_file(media: &ScreenshotMedia) -> Result<(), ScreenshotError> {
    if media.byte_length == 0 || media.byte_length > MAX_SCREENSHOT_PNG_BYTES as u64 {
        return Err(screenshot_error(
            "screenshot.media_size",
            "截图资源大小无效",
            false,
        ));
    }
    let metadata = fs::metadata(&media.path).map_err(|_| {
        screenshot_error(
            "screenshot.media_missing",
            "截图资源已被移除，请重新截图",
            true,
        )
    })?;
    if !metadata.is_file() || metadata.len() != media.byte_length {
        return Err(screenshot_error(
            "screenshot.media_changed",
            "截图资源校验失败，请重新截图",
            false,
        ));
    }
    Ok(())
}

fn validate_upload_request<'a>(
    lease: &ScreenshotCommitLease,
    window_label: &str,
    session_id: Option<&str>,
    action: Option<&str>,
    body: Option<&'a [u8]>,
    current_time_ms: u64,
) -> Result<&'a [u8], ScreenshotError> {
    require_capture_window(window_label)?;
    if lease.expires_at_ms <= current_time_ms {
        return Err(screenshot_error(
            "screenshot.lease_expired",
            "截图提交凭证已过期，请重试",
            true,
        ));
    }
    if session_id != Some(lease.session_id.as_str()) || action != Some(lease.action.as_str()) {
        return Err(screenshot_error(
            "screenshot.lease_scope",
            "截图提交凭证与当前会话或动作不匹配",
            false,
        ));
    }
    body.ok_or_else(|| {
        screenshot_error(
            "screenshot.raw_required",
            "截图提交必须使用原始 PNG 字节",
            false,
        )
    })
}

fn write_png_to_path(path: &Path, bytes: &[u8]) -> Result<(), ScreenshotError> {
    fs::write(path, bytes).map_err(|error| {
        screenshot_error(
            "screenshot.save_failed",
            format!("保存截图失败: {error}"),
            true,
        )
    })
}

fn stage_pin_media(
    root: &Path,
    token: String,
    created_at_ms: u64,
    bytes: &[u8],
    width: u32,
    height: u32,
) -> Result<ScreenshotMedia, ScreenshotError> {
    let directory = root.join(format!("pin-{created_at_ms}-{token}"));
    create_owner_only_directory(&directory)?;
    let temporary_path = directory.join("pin.tmp");
    let media_path = directory.join("pin.png");
    if let Err(error) =
        fs::write(&temporary_path, bytes).and_then(|_| fs::rename(&temporary_path, &media_path))
    {
        let _ = fs::remove_dir_all(&directory);
        return Err(screenshot_error(
            "screenshot.pin_write",
            format!("创建钉图资源失败: {error}"),
            true,
        ));
    }
    Ok(ScreenshotMedia {
        token,
        path: media_path,
        byte_length: bytes.len() as u64,
        width,
        height,
    })
}

fn media_descriptor(
    media: &ScreenshotMedia,
    expires_at_ms: Option<u64>,
) -> ScreenshotMediaDescriptor {
    ScreenshotMediaDescriptor {
        token: media.token.clone(),
        mime_type: "image/png".into(),
        byte_length: media.byte_length,
        width: media.width,
        height: media.height,
        expires_at_ms,
    }
}

fn validate_png_bytes(bytes: &[u8]) -> Result<(u32, u32), ScreenshotError> {
    if bytes.is_empty() || bytes.len() > MAX_SCREENSHOT_PNG_BYTES {
        return Err(screenshot_error(
            "screenshot.png_size",
            "PNG 数据大小超出允许范围",
            false,
        ));
    }
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(screenshot_error(
            "screenshot.png_signature",
            "截图数据不是有效的 PNG",
            false,
        ));
    }
    let reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| screenshot_error("screenshot.png_header", "无法读取 PNG 头", false))?;
    if reader.format() != Some(image::ImageFormat::Png) {
        return Err(screenshot_error(
            "screenshot.png_type",
            "截图数据类型不是 PNG",
            false,
        ));
    }
    let (width, height) = reader
        .into_dimensions()
        .map_err(|_| screenshot_error("screenshot.png_dimensions", "无法读取 PNG 尺寸", false))?;
    validate_dimensions(width, height)?;
    Ok((width, height))
}

fn validate_dimensions(width: u32, height: u32) -> Result<(), ScreenshotError> {
    if width == 0
        || height == 0
        || width > MAX_SCREENSHOT_DIMENSION
        || height > MAX_SCREENSHOT_DIMENSION
        || u64::from(width).saturating_mul(u64::from(height)) > MAX_SCREENSHOT_PIXELS
    {
        return Err(screenshot_error(
            "screenshot.png_bounds",
            "PNG 尺寸超出允许范围",
            false,
        ));
    }
    Ok(())
}

fn hide_visible_shell_windows(app: &tauri::AppHandle) -> Option<String> {
    let mut restore_window_label = None;
    for label in [TRAY_WINDOW_LABEL, MAIN_WINDOW_LABEL] {
        if let Some(window) = app.get_webview_window(label) {
            if window.is_visible().unwrap_or(false) {
                if restore_window_label.is_none() {
                    restore_window_label = Some(label.to_string());
                }
                let _ = hide_surface(&window);
            }
        }
    }
    restore_window_label
}

fn restore_shell_window(app: &tauri::AppHandle, label: Option<&str>) {
    if let Some(label) = label {
        if let Some(window) = app.get_webview_window(label) {
            let _ = show_surface(&window);
            let _ = window.set_focus();
        }
    }
}

fn normalize_start_action(action: String) -> String {
    match action.as_str() {
        "copy" | "save" => action,
        _ => "copy".into(),
    }
}

fn normalize_commit_action(action: &str) -> Result<String, ScreenshotError> {
    match action {
        "copy" | "save" | "pin" => Ok(action.into()),
        _ => Err(screenshot_error(
            "screenshot.action",
            "不支持的截图动作",
            false,
        )),
    }
}

fn create_session_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or_default();
    let mut previous = LAST_SESSION_ID.load(Ordering::Relaxed);
    loop {
        let next = nanos.max(previous.saturating_add(1));
        match LAST_SESSION_ID.compare_exchange(previous, next, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(_) => return next.to_string(),
            Err(current) => previous = current,
        }
    }
}

fn create_pin_label() -> String {
    format!("{PIN_WINDOW_LABEL}-{}", create_session_id())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn random_resource_token(prefix: &str) -> Result<String, ScreenshotError> {
    let mut bytes = [0_u8; 32];
    fill(&mut bytes)
        .map_err(|_| screenshot_error("screenshot.random", "无法创建安全的截图资源凭证", true))?;
    Ok(format!("{prefix}-{}", URL_SAFE_NO_PAD.encode(bytes)))
}

fn validate_resource_token(token: &str) -> Result<(), ScreenshotError> {
    if token.is_empty()
        || token.len() > 128
        || !token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(screenshot_error(
            "screenshot.token",
            "截图资源凭证格式无效",
            false,
        ));
    }
    Ok(())
}

fn screenshot_media_root(app: &tauri::AppHandle) -> Result<PathBuf, ScreenshotError> {
    let root = app
        .path()
        .app_cache_dir()
        .map_err(|error| {
            screenshot_error(
                "screenshot.cache_root",
                format!("无法定位截图缓存目录: {error}"),
                true,
            )
        })?
        .join("screenshot-media");
    create_owner_only_directory(&root)?;
    Ok(root)
}

fn create_owner_only_directory(path: &Path) -> Result<(), ScreenshotError> {
    fs::create_dir_all(path).map_err(|error| {
        screenshot_error(
            "screenshot.cache_create",
            format!("无法创建截图缓存目录: {error}"),
            true,
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|error| {
            screenshot_error(
                "screenshot.cache_permissions",
                format!("无法保护截图缓存目录: {error}"),
                false,
            )
        })?;
    }
    Ok(())
}

fn cleanup_stale_screenshot_media(root: &Path, store: &ScreenshotSessionStore) {
    cleanup_stale_screenshot_media_at(root, store, now_ms());
}

fn cleanup_stale_screenshot_media_at(
    root: &Path,
    store: &ScreenshotSessionStore,
    current_time_ms: u64,
) {
    let owned = store.owned_directories();
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if owned.contains(&path) || !path.is_dir() {
            continue;
        }
        let Some(created_at_ms) = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(owned_directory_created_at)
        else {
            continue;
        };
        if current_time_ms.saturating_sub(created_at_ms) >= SCREENSHOT_SESSION_TTL_MS {
            let _ = fs::remove_dir_all(path);
        }
    }
}

fn owned_directory_created_at(name: &str) -> Option<u64> {
    let remainder = name
        .strip_prefix("session-")
        .or_else(|| name.strip_prefix("pin-"))?;
    remainder.split('-').next()?.parse().ok()
}

fn remove_owned_media(path: &Path) {
    let _ = fs::remove_file(path);
    if let Some(parent) = path.parent() {
        let name_is_owned = parent
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(owned_directory_created_at)
            .is_some();
        if name_is_owned {
            let _ = fs::remove_dir_all(parent);
        }
    }
}

#[cfg(target_os = "macos")]
fn capture_fullscreen_png(path: &Path) -> Result<(u64, u32, u32), ScreenshotError> {
    let status = std::process::Command::new("screencapture")
        .args(["-x", "-t", "png"])
        .arg(path)
        .status()
        .map_err(|error| {
            screenshot_error(
                "screenshot.capture_launch",
                format!("调用系统截图失败: {error}"),
                true,
            )
        })?;
    if !status.success() {
        return Err(screenshot_error(
            "screenshot.capture_failed",
            "系统截图命令执行失败",
            true,
        ));
    }
    let metadata = fs::metadata(path).map_err(|error| {
        screenshot_error(
            "screenshot.capture_missing",
            format!("读取截图文件失败: {error}"),
            true,
        )
    })?;
    if metadata.len() == 0 || metadata.len() > MAX_SCREENSHOT_PNG_BYTES as u64 {
        return Err(screenshot_error(
            "screenshot.capture_size",
            "截图文件大小超出允许范围",
            false,
        ));
    }
    let reader = image::ImageReader::open(path)
        .and_then(|reader| reader.with_guessed_format())
        .map_err(|_| screenshot_error("screenshot.capture_header", "无法读取截图头", false))?;
    if reader.format() != Some(image::ImageFormat::Png) {
        return Err(screenshot_error(
            "screenshot.capture_type",
            "系统截图结果不是 PNG",
            false,
        ));
    }
    let (width, height) = reader.into_dimensions().map_err(|_| {
        screenshot_error("screenshot.capture_dimensions", "无法读取截图尺寸", false)
    })?;
    validate_dimensions(width, height)?;
    Ok((metadata.len(), width, height))
}

fn open_capture_window(app: &tauri::AppHandle, session_id: &str) -> Result<(), ScreenshotError> {
    if let Some(existing) = app.get_webview_window(CAPTURE_WINDOW_LABEL) {
        let _ = close_surface(&existing);
    }
    let monitor = app
        .primary_monitor()
        .map_err(|error| {
            screenshot_error(
                "screenshot.monitor",
                format!("读取主显示器失败: {error}"),
                true,
            )
        })?
        .ok_or_else(|| {
            screenshot_error(
                "screenshot.monitor_missing",
                "未找到可用于截图的主显示器",
                true,
            )
        })?;
    let monitor_position = *monitor.position();
    let monitor_size = *monitor.size();
    let capture_window =
        tauri::WebviewWindowBuilder::new(app, CAPTURE_WINDOW_LABEL, WebviewUrl::App("".into()))
            .decorations(false)
            .resizable(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .transparent(false)
            .shadow(false)
            .visible(false)
            .focused(false)
            .build()
            .map_err(|error| {
                screenshot_error(
                    "screenshot.capture_window",
                    format!("打开截图编辑窗口失败: {error}"),
                    true,
                )
            })?;
    let cleanup_app = app.clone();
    let cleanup_session_id = session_id.to_string();
    capture_window.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::Destroyed) {
            let store = cleanup_app.state::<ScreenshotSessionStore>();
            let active = store
                .active()
                .filter(|session| session.id == cleanup_session_id);
            store.clear_active_if(&cleanup_session_id);
            if let Some(active) = active {
                restore_shell_window(&cleanup_app, active.restore_window_label.as_deref());
            }
        }
    });
    // AppKit keeps the lower-left corner fixed while resizing. Size the hidden
    // window first so the final position restores the monitor's global top-left.
    let prepare_result = capture_window
        .set_size(monitor_size)
        .and_then(|_| capture_window.set_position(monitor_position));
    if let Err(error) = prepare_result {
        let _ = close_surface(&capture_window);
        return Err(screenshot_error(
            "screenshot.capture_show",
            format!("显示截图编辑窗口失败: {error}"),
            true,
        ));
    }
    let timeout_app = app.clone();
    let timeout_session_id = session_id.to_string();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(SCREENSHOT_REVEAL_TIMEOUT).await;
        if timeout_app
            .state::<ScreenshotSessionStore>()
            .is_awaiting_reveal(&timeout_session_id)
        {
            cleanup_failed_reveal(&timeout_app, &timeout_session_id);
        }
    });
    Ok(())
}

fn reveal_capture_window_on_main_thread(
    capture_window: tauri::WebviewWindow,
) -> Result<(), ScreenshotError> {
    let scheduling_window = capture_window.clone();
    let (sender, receiver) = mpsc::sync_channel(1);
    scheduling_window
        .run_on_main_thread(move || {
            let result = configure_and_reveal_capture_window(&capture_window);
            let _ = sender.send(result);
        })
        .map_err(|error| {
            screenshot_error(
                "screenshot.capture_reveal_schedule",
                format!("调度截图窗口显示失败: {error}"),
                true,
            )
        })?;
    receiver
        .recv_timeout(SCREENSHOT_REVEAL_TIMEOUT)
        .map_err(|_| {
            screenshot_error(
                "screenshot.capture_reveal_timeout",
                "截图窗口准备超时，请重新截图",
                true,
            )
        })?
}

fn configure_and_reveal_capture_window(
    capture_window: &tauri::WebviewWindow,
) -> Result<(), ScreenshotError> {
    configure_native_capture_window(capture_window)?;
    show_surface(capture_window)
        .and_then(|_| capture_window.set_focus())
        .map_err(|error| {
            screenshot_error(
                "screenshot.capture_show",
                format!("显示截图编辑窗口失败: {error}"),
                true,
            )
        })
}

#[cfg(target_os = "macos")]
fn configure_native_capture_window(
    capture_window: &tauri::WebviewWindow,
) -> Result<(), ScreenshotError> {
    let native_window = capture_window.ns_window().map_err(|error| {
        screenshot_error(
            "screenshot.capture_native_window",
            format!("读取 macOS 截图窗口失败: {error}"),
            true,
        )
    })?;
    unsafe {
        let native_window = &*native_window.cast::<NSWindow>();
        native_window.setLevel(NSScreenSaverWindowLevel);
        native_window.setSharingType(NSWindowSharingType::None);
        native_window.setAnimationBehavior(NSWindowAnimationBehavior::None);
        native_window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::Stationary,
        );
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn configure_native_capture_window(
    _capture_window: &tauri::WebviewWindow,
) -> Result<(), ScreenshotError> {
    Ok(())
}

fn cleanup_failed_reveal(app: &tauri::AppHandle, session_id: &str) {
    let active = app
        .state::<ScreenshotSessionStore>()
        .active()
        .filter(|session| session.id == session_id);
    app.state::<ScreenshotSessionStore>()
        .clear_active_if(session_id);
    if let Some(capture) = app.get_webview_window(CAPTURE_WINDOW_LABEL) {
        let _ = close_surface(&capture);
    }
    if let Some(active) = active {
        restore_shell_window(app, active.restore_window_label.as_deref());
    }
}

fn require_capture_window(window_label: &str) -> Result<(), ScreenshotError> {
    (window_label == CAPTURE_WINDOW_LABEL)
        .then_some(())
        .ok_or_else(|| {
            screenshot_error(
                "screenshot.window_scope",
                "当前命令仅允许截图编辑窗口调用",
                false,
            )
        })
}

fn screenshot_error(
    code: impl Into<String>,
    message: impl Into<String>,
    retryable: bool,
) -> ScreenshotError {
    ScreenshotError {
        code: code.into(),
        message: message.into(),
        retryable,
    }
}

fn platform_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "macOS"
    } else if cfg!(target_os = "windows") {
        "Windows"
    } else if cfg!(target_os = "linux") {
        "Linux"
    } else {
        "Unknown"
    }
}

#[cfg(target_os = "macos")]
fn copy_png_to_clipboard(bytes: &[u8]) -> Result<(), String> {
    unsafe {
        let pool: *mut Object = msg_send![class!(NSAutoreleasePool), new];
        let pasteboard: *mut Object = msg_send![class!(NSPasteboard), generalPasteboard];
        let result = copy_png_to_pasteboard(bytes, pasteboard);
        let _: () = msg_send![pool, drain];
        result
    }
}

#[cfg(target_os = "macos")]
unsafe fn copy_png_to_pasteboard(bytes: &[u8], pasteboard: *mut Object) -> Result<(), String> {
    if pasteboard.is_null() {
        return Err("复制到剪贴板失败：系统剪贴板不可用".into());
    }
    let data: *mut Object =
        msg_send![class!(NSData), dataWithBytes:bytes.as_ptr() length:bytes.len()];
    if data.is_null() {
        return Err("复制到剪贴板失败：无法创建 PNG 数据".into());
    }
    let png_type: *mut Object =
        msg_send![class!(NSString), stringWithUTF8String:c"public.png".as_ptr()];
    let _: isize = msg_send![pasteboard, clearContents];
    let copied: BOOL = msg_send![pasteboard, setData:data forType:png_type];
    if copied == YES {
        Ok(())
    } else {
        Err("复制到剪贴板失败：系统拒绝 PNG 数据".into())
    }
}

#[cfg(not(target_os = "macos"))]
fn copy_png_to_clipboard(_bytes: &[u8]) -> Result<(), String> {
    Err("当前平台暂不支持图片复制".into())
}

#[cfg(target_os = "windows")]
fn launch_system_screenshot(_action: &str) -> Result<(), String> {
    std::process::Command::new("explorer.exe")
        .arg("ms-screenclip:")
        .spawn()
        .or_else(|_| std::process::Command::new("SnippingTool.exe").spawn())
        .map_err(|error| format!("打开截图工具失败: {error}"))?;
    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn launch_system_screenshot(_action: &str) -> Result<(), String> {
    Err("当前平台暂不支持系统截图入口".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_fixture(width: u32, height: u32) -> Vec<u8> {
        let image = image::RgbaImage::new(width, height);
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut Cursor::new(&mut png), image::ImageFormat::Png)
            .expect("fixture should encode");
        png
    }

    fn media(path: PathBuf, token: &str) -> ScreenshotMedia {
        ScreenshotMedia {
            token: token.into(),
            path,
            byte_length: 1,
            width: 1,
            height: 1,
        }
    }

    #[test]
    fn normalizes_actions() {
        assert_eq!(normalize_start_action("unknown".into()), "copy");
        assert_eq!(normalize_start_action("save".into()), "save");
        assert!(normalize_commit_action("pin").is_ok());
        assert!(normalize_commit_action("unknown").is_err());
    }

    #[test]
    fn validates_png_signature_dimensions_and_bounds() {
        let png = png_fixture(2, 1);
        assert_eq!(
            validate_png_bytes(&png).expect("PNG should validate"),
            (2, 1)
        );
        assert_eq!(
            validate_png_bytes(b"not-png")
                .expect_err("signature should fail")
                .code,
            "screenshot.png_signature"
        );
        assert!(validate_dimensions(MAX_SCREENSHOT_DIMENSION + 1, 1).is_err());
        assert!(validate_dimensions(20_000, 20_000).is_err());
    }

    #[test]
    fn resource_tokens_reject_traversal() {
        assert!(validate_resource_token("media-valid_123").is_ok());
        assert!(validate_resource_token("../../capture.png").is_err());
        assert!(validate_resource_token("media/escape").is_err());
    }

    #[test]
    fn media_and_lease_contracts_serialize_symmetrically() {
        let descriptor = ScreenshotMediaDescriptor {
            token: "media-1".into(),
            mime_type: "image/png".into(),
            byte_length: 42,
            width: 3,
            height: 2,
            expires_at_ms: Some(99),
        };
        assert_eq!(
            serde_json::to_value(descriptor).expect("descriptor should serialize"),
            serde_json::json!({
                "token": "media-1",
                "mimeType": "image/png",
                "byteLength": 42,
                "width": 3,
                "height": 2,
                "expiresAtMs": 99,
            })
        );
        assert!(
            serde_json::from_value::<PrepareScreenshotCommitInput>(serde_json::json!({
                "sessionId": "session-1",
                "action": "copy",
                "savePath": "/tmp/not-accepted.png",
            }))
            .is_err()
        );
    }

    #[test]
    fn capture_session_serializes_only_opaque_source_pixel_targets() {
        let payload = CaptureSessionPayload {
            session_id: "session-1".into(),
            initial_action: "copy".into(),
            media: ScreenshotMediaDescriptor {
                token: "media-1".into(),
                mime_type: "image/png".into(),
                byte_length: 42,
                width: 1440,
                height: 900,
                expires_at_ms: Some(99),
            },
            targets: vec![ScreenshotTargetCandidate {
                id: "target-0".into(),
                kind: ScreenshotTargetKind::Window,
                bounds: ScreenshotSourceBounds {
                    x: 10,
                    y: 20,
                    width: 300,
                    height: 200,
                },
            }],
        };
        let value = serde_json::to_value(payload).expect("capture payload should serialize");
        assert_eq!(
            value["targets"],
            serde_json::json!([{
                "id": "target-0",
                "kind": "window",
                "bounds": { "x": 10, "y": 20, "width": 300, "height": 200 }
            }])
        );
        let serialized = value.to_string();
        for forbidden in [
            "appName",
            "title",
            "pid",
            "processId",
            "z",
            "globalX",
            "globalY",
        ] {
            assert!(!serialized.contains(forbidden), "{forbidden}");
        }
    }

    #[test]
    fn upload_validation_rejects_interruption_expiry_and_scope_mismatches() {
        let lease = ScreenshotCommitLease {
            token: "upload-1".into(),
            session_id: "session-1".into(),
            action: "copy".into(),
            save_path: None,
            expires_at_ms: 100,
        };
        let bytes = b"raw";
        assert_eq!(
            validate_upload_request(
                &lease,
                CAPTURE_WINDOW_LABEL,
                Some("session-1"),
                Some("copy"),
                Some(bytes),
                99,
            )
            .expect("request should validate"),
            bytes
        );
        for (window, session, action, body, now, code) in [
            (
                "pin-1",
                Some("session-1"),
                Some("copy"),
                Some(bytes.as_slice()),
                99,
                "screenshot.window_scope",
            ),
            (
                CAPTURE_WINDOW_LABEL,
                Some("wrong"),
                Some("copy"),
                Some(bytes.as_slice()),
                99,
                "screenshot.lease_scope",
            ),
            (
                CAPTURE_WINDOW_LABEL,
                Some("session-1"),
                Some("save"),
                Some(bytes.as_slice()),
                99,
                "screenshot.lease_scope",
            ),
            (
                CAPTURE_WINDOW_LABEL,
                Some("session-1"),
                Some("copy"),
                None,
                99,
                "screenshot.raw_required",
            ),
            (
                CAPTURE_WINDOW_LABEL,
                Some("session-1"),
                Some("copy"),
                Some(bytes.as_slice()),
                100,
                "screenshot.lease_expired",
            ),
        ] {
            assert_eq!(
                validate_upload_request(&lease, window, session, action, body, now)
                    .expect_err("request should fail")
                    .code,
                code
            );
        }
    }

    #[test]
    fn save_and_pin_staging_write_exact_raw_png_bytes() {
        let root =
            std::env::temp_dir().join(format!("zero-screenshot-actions-{}", create_session_id()));
        create_owner_only_directory(&root).expect("action root");
        let png = png_fixture(3, 2);
        let save_path = root.join("saved.png");
        write_png_to_path(&save_path, &png).expect("save should succeed");
        assert_eq!(fs::read(&save_path).expect("saved bytes"), png);

        let pin = stage_pin_media(&root, "pin-media-test".into(), 10, &png, 3, 2)
            .expect("pin staging should succeed");
        assert_eq!(pin.width, 3);
        assert_eq!(pin.height, 2);
        assert_eq!(fs::read(&pin.path).expect("pin bytes"), png);
        remove_owned_media(&pin.path);
        assert!(!pin.path.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn store_consumes_leases_once_and_cleans_terminal_resources() {
        let root =
            std::env::temp_dir().join(format!("zero-screenshot-store-{}", create_session_id()));
        let session_directory = root.join("session-1-test");
        fs::create_dir_all(&session_directory).expect("session directory");
        let session_path = session_directory.join("capture.png");
        fs::write(&session_path, b"x").expect("session media");
        let pin_directory = root.join("pin-1-test");
        fs::create_dir_all(&pin_directory).expect("pin directory");
        let pin_path = pin_directory.join("pin.png");
        fs::write(&pin_path, b"x").expect("pin media");
        let store = ScreenshotSessionStore::default();
        store.set_active(ScreenshotSession {
            id: "session-1".into(),
            initial_action: "copy".into(),
            media: media(session_path, "media-session"),
            targets: Vec::new(),
            revealed: false,
            expires_at_ms: u64::MAX,
            restore_window_label: Some("tray".into()),
        });
        store
            .set_pin(
                "pin-1".into(),
                PinResource {
                    media: media(pin_path, "media-pin"),
                },
            )
            .expect("pin should insert");
        store
            .insert_lease(ScreenshotCommitLease {
                token: "upload-1".into(),
                session_id: "session-1".into(),
                action: "copy".into(),
                save_path: None,
                expires_at_ms: u64::MAX,
            })
            .expect("lease should insert");
        assert!(store.take_lease("upload-1").is_some());
        assert!(store.take_lease("upload-1").is_none());
        store.cleanup_all();
        assert_eq!(store.resource_counts(), (0, 0, 0));
        assert!(!session_directory.exists());
        assert!(!pin_directory.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn pin_store_rejects_growth_beyond_the_live_window_bound() {
        let root =
            std::env::temp_dir().join(format!("zero-screenshot-pin-bound-{}", create_session_id()));
        let store = ScreenshotSessionStore::default();
        for index in 0..MAX_SCREENSHOT_PINS {
            let directory = root.join(format!("pin-{index}"));
            fs::create_dir_all(&directory).unwrap();
            let path = directory.join("pin.png");
            fs::write(&path, b"x").unwrap();
            store
                .set_pin(
                    format!("pin-{index}"),
                    PinResource {
                        media: media(path, &format!("media-{index}")),
                    },
                )
                .unwrap();
        }
        let overflow_path = root.join("overflow.png");
        fs::write(&overflow_path, b"x").unwrap();
        let error = store
            .set_pin(
                "pin-overflow".into(),
                PinResource {
                    media: media(overflow_path.clone(), "media-overflow"),
                },
            )
            .unwrap_err();
        assert_eq!(error.code, "screenshot.pin_limit");
        assert_eq!(store.resource_counts().2, MAX_SCREENSHOT_PINS);
        store.cleanup_all();
        let _ = fs::remove_file(overflow_path);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn stale_cleanup_only_removes_owned_expired_directories() {
        let root =
            std::env::temp_dir().join(format!("zero-screenshot-cleanup-{}", create_session_id()));
        fs::create_dir_all(&root).expect("root");
        let expired = root.join("session-10-old");
        let current = root.join(format!("pin-{}-current", SCREENSHOT_SESSION_TTL_MS + 5));
        let unrelated = root.join("user-content");
        for path in [&expired, &current, &unrelated] {
            fs::create_dir_all(path).expect("fixture directory");
        }
        cleanup_stale_screenshot_media_at(
            &root,
            &ScreenshotSessionStore::default(),
            SCREENSHOT_SESSION_TTL_MS + 10,
        );
        assert!(!expired.exists());
        assert!(current.exists());
        assert!(unrelated.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn media_resolution_rejects_wrong_window_and_token() {
        let root =
            std::env::temp_dir().join(format!("zero-screenshot-scope-{}", create_session_id()));
        let directory = root.join("session-1-fixture");
        fs::create_dir_all(&directory).expect("scope directory");
        let path = directory.join("capture.png");
        fs::write(&path, b"x").expect("scope media");
        let store = ScreenshotSessionStore::default();
        store.set_active(ScreenshotSession {
            id: "session-1".into(),
            initial_action: "copy".into(),
            media: media(path, "media-session"),
            targets: Vec::new(),
            revealed: false,
            expires_at_ms: u64::MAX,
            restore_window_label: None,
        });
        assert!(store
            .resolve_media("media-session", CAPTURE_WINDOW_LABEL)
            .is_some());
        assert!(store
            .resolve_media("wrong-token", CAPTURE_WINDOW_LABEL)
            .is_none());
        assert!(store.resolve_media("media-session", "pin-wrong").is_none());
        store.cleanup_all();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reveal_claim_is_session_scoped_and_idempotent() {
        let root =
            std::env::temp_dir().join(format!("zero-screenshot-reveal-{}", create_session_id()));
        let directory = root.join("session-1-fixture");
        fs::create_dir_all(&directory).expect("reveal directory");
        let path = directory.join("capture.png");
        fs::write(&path, b"x").expect("reveal media");
        let store = ScreenshotSessionStore::default();
        store.set_active(ScreenshotSession {
            id: "session-1".into(),
            initial_action: "copy".into(),
            media: media(path, "media-session"),
            targets: Vec::new(),
            revealed: false,
            expires_at_ms: u64::MAX,
            restore_window_label: None,
        });

        assert!(store.is_awaiting_reveal("session-1"));
        assert_eq!(
            store.claim_reveal("wrong-session").unwrap_err().code,
            "screenshot.session_scope"
        );
        assert!(store.claim_reveal("session-1").expect("first reveal"));
        assert!(!store.is_awaiting_reveal("session-1"));
        assert!(!store.claim_reveal("session-1").expect("repeat reveal"));
        store.cleanup_all();
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn expired_leases_are_pruned_and_cleanup_is_idempotent() {
        let store = ScreenshotSessionStore::default();
        store
            .insert_lease(ScreenshotCommitLease {
                token: "upload-expired".into(),
                session_id: "session-1".into(),
                action: "copy".into(),
                save_path: None,
                expires_at_ms: 1,
            })
            .expect("expired lease can be inserted before pruning");
        store
            .insert_lease(ScreenshotCommitLease {
                token: "upload-current".into(),
                session_id: "session-1".into(),
                action: "copy".into(),
                save_path: None,
                expires_at_ms: u64::MAX,
            })
            .expect("current lease should insert");
        assert!(store.take_lease("upload-expired").is_none());
        assert!(store.take_lease("upload-current").is_some());
        store.cleanup_all();
        store.cleanup_all();
        assert_eq!(store.resource_counts(), (0, 0, 0));
    }

    #[test]
    fn cancellation_revokes_leases_and_stale_cleanup_cannot_remove_new_session() {
        let root =
            std::env::temp_dir().join(format!("zero-screenshot-cancel-{}", create_session_id()));
        let store = ScreenshotSessionStore::default();
        for id in ["old", "new"] {
            let directory = root.join(format!("session-1-{id}"));
            fs::create_dir_all(&directory).expect("session directory");
            let path = directory.join("capture.png");
            fs::write(&path, b"x").expect("session media");
            store.set_active(ScreenshotSession {
                id: id.into(),
                initial_action: "copy".into(),
                media: media(path, &format!("media-{id}")),
                targets: Vec::new(),
                revealed: false,
                expires_at_ms: u64::MAX,
                restore_window_label: None,
            });
        }
        store
            .insert_lease(ScreenshotCommitLease {
                token: "upload-new".into(),
                session_id: "new".into(),
                action: "copy".into(),
                save_path: None,
                expires_at_ms: u64::MAX,
            })
            .expect("lease should insert");
        store.clear_active_if("old");
        assert_eq!(store.active().expect("new session remains").id, "new");
        store.clear_active_if("new");
        assert!(store.take_lease("upload-new").is_none());
        assert_eq!(store.resource_counts(), (0, 0, 0));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn ten_store_lifecycle_cycles_return_to_zero_resources() {
        let store = ScreenshotSessionStore::default();
        let root =
            std::env::temp_dir().join(format!("zero-screenshot-cycles-{}", create_session_id()));
        for cycle in 0..10 {
            let directory = root.join(format!("session-{cycle}-fixture"));
            fs::create_dir_all(&directory).expect("cycle directory");
            let path = directory.join("capture.png");
            fs::write(&path, b"x").expect("cycle media");
            store.set_active(ScreenshotSession {
                id: format!("session-{cycle}"),
                initial_action: "copy".into(),
                media: media(path, &format!("media-{cycle}")),
                targets: Vec::new(),
                revealed: false,
                expires_at_ms: u64::MAX,
                restore_window_label: None,
            });
            store.clear_active();
            assert_eq!(store.resource_counts(), (0, 0, 0));
            assert!(!directory.exists());
        }
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn pin_labels_are_unique() {
        let first = create_pin_label();
        let second = create_pin_label();
        assert!(first.starts_with("pin-"));
        assert_ne!(first, second);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_png_clipboard_round_trips_on_private_pasteboard() {
        unsafe {
            let pool: *mut Object = msg_send![class!(NSAutoreleasePool), new];
            let pasteboard: *mut Object = msg_send![class!(NSPasteboard), pasteboardWithUniqueName];
            let png = b"\x89PNG\r\n\x1a\nclipboard-test";
            copy_png_to_pasteboard(png, pasteboard).expect("native pasteboard write");
            let png_type: *mut Object =
                msg_send![class!(NSString), stringWithUTF8String:c"public.png".as_ptr()];
            let data: *mut Object = msg_send![pasteboard, dataForType:png_type];
            assert!(!data.is_null());
            let data_bytes: *const u8 = msg_send![data, bytes];
            let data_length: usize = msg_send![data, length];
            assert_eq!(std::slice::from_raw_parts(data_bytes, data_length), png);
            let _: () = msg_send![pasteboard, releaseGlobally];
            let _: () = msg_send![pool, drain];
        }
    }
}

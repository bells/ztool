use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use base64::Engine;
#[cfg(target_os = "macos")]
use objc::{
    class, msg_send,
    runtime::{Object, BOOL, YES},
    sel, sel_impl,
};
use serde::{Deserialize, Serialize};
use tauri::{Manager, WebviewUrl};

const CAPTURE_WINDOW_LABEL: &str = "capture";
const TRAY_WINDOW_LABEL: &str = "tray";
const MAIN_WINDOW_LABEL: &str = "main";
const PIN_WINDOW_LABEL: &str = "pin";
const DEFAULT_SAVE_FILE_NAME: &str = "zero-snap.png";
const PIN_TITLEBAR_HEIGHT: f64 = 30.0;
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
pub struct CaptureSessionPayload {
    pub session_id: String,
    pub image_base64: String,
    pub initial_action: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScreenshotCommitResult {
    pub copied: bool,
    pub saved_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScreenshotCancelResult {
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PinScreenshotResult {
    pub pin_window_label: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PinPayload {
    pub image_base64: String,
}

#[derive(Debug, Clone)]
struct ScreenshotSession {
    id: String,
    initial_action: String,
    image_base64: String,
    width: u32,
    height: u32,
    restore_window_label: Option<String>,
}

#[derive(Debug, Default)]
pub struct ScreenshotSessionStore {
    active: Mutex<Option<ScreenshotSession>>,
    pins: Mutex<HashMap<String, String>>,
}

impl ScreenshotSessionStore {
    fn set_active(&self, session: ScreenshotSession) {
        if let Ok(mut active) = self.active.lock() {
            *active = Some(session);
        }
    }

    fn active(&self) -> Option<ScreenshotSession> {
        self.active.lock().ok().and_then(|active| active.clone())
    }

    fn clear_active(&self) {
        if let Ok(mut active) = self.active.lock() {
            *active = None;
        }
    }

    fn set_pin_image(&self, label: String, image_base64: String) {
        if let Ok(mut pins) = self.pins.lock() {
            pins.insert(label, image_base64);
        }
    }

    fn pin_image(&self, label: &str) -> Option<String> {
        self.pins
            .lock()
            .ok()
            .and_then(|pins| pins.get(label).cloned())
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
) -> Result<ScreenshotStartResult, String> {
    let normalized_action = normalize_action(action);
    let restore_window_label = hide_visible_shell_windows(&app);

    #[cfg(target_os = "macos")]
    {
        let (bytes, width, height) = match capture_fullscreen_png() {
            Ok(capture) => capture,
            Err(err) => {
                restore_shell_window(&app, restore_window_label.as_deref());
                return Err(err);
            }
        };
        let session_id = create_session_id();
        let image_base64 = base64::engine::general_purpose::STANDARD.encode(bytes);

        let store = app.state::<ScreenshotSessionStore>();
        let restore_window_label_for_error = restore_window_label.clone();
        store.set_active(ScreenshotSession {
            id: session_id.clone(),
            initial_action: normalized_action.clone(),
            image_base64,
            width,
            height,
            restore_window_label,
        });

        if let Err(err) = open_capture_window(&app) {
            store.clear_active();
            restore_shell_window(&app, restore_window_label_for_error.as_deref());
            return Err(err);
        }

        return Ok(ScreenshotStartResult {
            mode: "custom-overlay".into(),
            platform: "macOS".into(),
            action: normalized_action,
            message: "截图编辑器已打开".into(),
            session_id: Some(session_id),
            capture_window_label: Some(CAPTURE_WINDOW_LABEL.into()),
        });
    }

    #[cfg(not(target_os = "macos"))]
    {
        if let Err(err) = launch_system_screenshot(&normalized_action) {
            restore_shell_window(&app, restore_window_label.as_deref());
            return Err(err);
        }

        Ok(ScreenshotStartResult {
            mode: "system-launcher".into(),
            platform: platform_name().into(),
            action: normalized_action.clone(),
            message: match normalized_action.as_str() {
                "save" => "截图工具已打开，完成后会保存图片".into(),
                _ => "截图工具已打开，完成后会复制到剪贴板".into(),
            },
            session_id: None,
            capture_window_label: None,
        })
    }
}

pub fn init_screenshot_session(
    app: tauri::AppHandle,
    session_id: Option<String>,
) -> Result<CaptureSessionPayload, String> {
    let store = app.state::<ScreenshotSessionStore>();
    let active = store
        .active()
        .ok_or_else(|| "截图会话不存在或已结束".to_string())?;

    if let Some(requested) = session_id {
        if requested != active.id {
            return Err("截图会话已更新，请重新开始截图".into());
        }
    }

    Ok(CaptureSessionPayload {
        session_id: active.id,
        image_base64: active.image_base64,
        initial_action: active.initial_action,
        width: active.width,
        height: active.height,
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommitScreenshotInput {
    pub session_id: String,
    pub action: String,
    pub png_base64: String,
    pub save_path: Option<String>,
}

pub fn commit_screenshot(
    app: tauri::AppHandle,
    input: CommitScreenshotInput,
) -> Result<ScreenshotCommitResult, String> {
    let active = validate_session(&app, &input.session_id)?;

    let action = normalize_action(input.action);
    let bytes = decode_png_base64(&input.png_base64)?;

    let mut copied = false;
    let mut saved_path = None;

    if action == "copy" {
        copy_png_to_clipboard(&bytes)?;
        copied = true;
    } else {
        let resolved_path = if let Some(path) = input.save_path {
            PathBuf::from(path)
        } else {
            rfd::FileDialog::new()
                .set_file_name(DEFAULT_SAVE_FILE_NAME)
                .save_file()
                .ok_or_else(|| "已取消保存".to_string())?
        };

        std::fs::write(&resolved_path, bytes).map_err(|e| format!("保存截图失败: {e}"))?;
        saved_path = Some(resolved_path.to_string_lossy().into_owned());
    }

    if let Some(capture) = app.get_webview_window(CAPTURE_WINDOW_LABEL) {
        let _ = capture.close();
    }
    restore_shell_window(&app, active.restore_window_label.as_deref());

    let store = app.state::<ScreenshotSessionStore>();
    store.clear_active();

    Ok(ScreenshotCommitResult { copied, saved_path })
}

pub fn cancel_screenshot_session(
    app: tauri::AppHandle,
    session_id: String,
) -> Result<ScreenshotCancelResult, String> {
    let active = validate_session(&app, &session_id)?;

    if let Some(capture) = app.get_webview_window(CAPTURE_WINDOW_LABEL) {
        let _ = capture.close();
    }
    restore_shell_window(&app, active.restore_window_label.as_deref());

    let store = app.state::<ScreenshotSessionStore>();
    store.clear_active();

    Ok(ScreenshotCancelResult { ok: true })
}

#[derive(Debug, Clone, Deserialize)]
pub struct PinScreenshotInput {
    pub session_id: String,
    pub png_base64: String,
}

pub fn pin_screenshot(
    app: tauri::AppHandle,
    input: PinScreenshotInput,
) -> Result<PinScreenshotResult, String> {
    let _active = validate_session(&app, &input.session_id)?;

    let bytes = decode_png_base64(&input.png_base64)?;
    let (width, height) = png_dimensions(&bytes)?;
    let label = create_pin_label();
    let store = app.state::<ScreenshotSessionStore>();
    store.set_pin_image(label.clone(), input.png_base64);

    let pin_window = tauri::WebviewWindowBuilder::new(&app, &label, WebviewUrl::App("".into()))
        .title("Pinned Image")
        .decorations(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(false)
        .inner_size(width as f64, height as f64 + PIN_TITLEBAR_HEIGHT)
        .build()
        .map_err(|e| format!("打开钉图窗口失败: {e}"))?;

    let _ = pin_window.show();
    let _ = pin_window.set_focus();

    Ok(PinScreenshotResult {
        pin_window_label: label,
    })
}

pub fn init_pin_window(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<PinPayload, String> {
    let label = window.label().to_string();
    let store = app.state::<ScreenshotSessionStore>();
    let image_base64 = store
        .pin_image(&label)
        .ok_or_else(|| "钉图内容不存在".to_string())?;

    Ok(PinPayload { image_base64 })
}

fn validate_session(app: &tauri::AppHandle, session_id: &str) -> Result<ScreenshotSession, String> {
    let store = app.state::<ScreenshotSessionStore>();
    let active = store
        .active()
        .ok_or_else(|| "截图会话不存在或已结束".to_string())?;

    if active.id != session_id {
        return Err("截图会话已失效，请重新截图".into());
    }

    Ok(active)
}

fn hide_visible_shell_windows(app: &tauri::AppHandle) -> Option<String> {
    let mut restore_window_label = None;

    for label in [TRAY_WINDOW_LABEL, MAIN_WINDOW_LABEL] {
        if let Some(window) = app.get_webview_window(label) {
            if window.is_visible().unwrap_or(false) {
                if restore_window_label.is_none() {
                    restore_window_label = Some(label.to_string());
                }
                let _ = window.hide();
            }
        }
    }

    restore_window_label
}

fn restore_shell_window(app: &tauri::AppHandle, label: Option<&str>) {
    if let Some(label) = label {
        if let Some(window) = app.get_webview_window(label) {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

fn normalize_action(action: String) -> String {
    match action.as_str() {
        "copy" | "save" => action,
        _ => "copy".into(),
    }
}

fn create_session_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
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

fn decode_png_base64(input: &str) -> Result<Vec<u8>, String> {
    let base64_part = input
        .strip_prefix("data:image/png;base64,")
        .unwrap_or(input);

    base64::engine::general_purpose::STANDARD
        .decode(base64_part)
        .map_err(|e| format!("解析图片失败: {e}"))
}

fn png_dimensions(bytes: &[u8]) -> Result<(u32, u32), String> {
    let image = image::load_from_memory(bytes).map_err(|e| format!("解析图片尺寸失败: {e}"))?;
    Ok((image.width(), image.height()))
}

#[cfg(target_os = "macos")]
fn capture_fullscreen_png() -> Result<(Vec<u8>, u32, u32), String> {
    let temp_path =
        std::env::temp_dir().join(format!("zero-snap-capture-{}.png", create_session_id()));

    std::process::Command::new("screencapture")
        .args(["-x", "-t", "png"])
        .arg(&temp_path)
        .status()
        .map_err(|e| format!("调用系统截图失败: {e}"))
        .and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                Err("系统截图命令执行失败".to_string())
            }
        })?;

    let bytes = std::fs::read(&temp_path).map_err(|e| format!("读取截图文件失败: {e}"))?;
    let _ = std::fs::remove_file(&temp_path);

    let dimensions = image::load_from_memory(&bytes)
        .map_err(|e| format!("解析截图尺寸失败: {e}"))?
        .to_rgba8()
        .dimensions();

    Ok((bytes, dimensions.0, dimensions.1))
}

fn open_capture_window(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(existing) = app.get_webview_window(CAPTURE_WINDOW_LABEL) {
        let _ = existing.close();
    }

    let monitor = app
        .primary_monitor()
        .map_err(|e| format!("读取主显示器失败: {e}"))?
        .ok_or_else(|| "未找到可用于截图的主显示器".to_string())?;
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
            .map_err(|e| format!("打开截图编辑窗口失败: {e}"))?;

    let prepare_result = capture_window
        .set_position(monitor_position)
        .and_then(|_| capture_window.set_size(monitor_size))
        .and_then(|_| capture_window.show())
        .and_then(|_| capture_window.set_focus());

    if let Err(err) = prepare_result {
        let _ = capture_window.close();
        return Err(format!("显示截图编辑窗口失败: {err}"));
    }

    Ok(())
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

    let png_type: *mut Object = msg_send![
        class!(NSString),
        stringWithUTF8String:b"public.png\0".as_ptr()
    ];
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
        .map_err(|e| format!("打开截图工具失败: {e}"))?;

    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn launch_system_screenshot(_action: &str) -> Result<(), String> {
    Err("当前平台暂不支持系统截图入口".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_unknown_action_to_copy() {
        assert_eq!(normalize_action("unknown".into()), "copy");
        assert_eq!(normalize_action("copy".into()), "copy");
        assert_eq!(normalize_action("save".into()), "save");
    }

    #[test]
    fn decode_data_url_png() {
        let payload = "data:image/png;base64,aGVsbG8=";
        let decoded = decode_png_base64(payload).expect("decode should succeed");
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn reads_png_dimensions() {
        let image = image::RgbaImage::new(2, 1);
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .expect("fixture should encode");

        assert_eq!(
            png_dimensions(&png).expect("dimensions should parse"),
            (2, 1)
        );
    }

    #[test]
    fn store_tracks_and_clears_active_session() {
        let store = ScreenshotSessionStore::default();
        store.set_active(ScreenshotSession {
            id: "session-1".into(),
            initial_action: "copy".into(),
            image_base64: "abc".into(),
            width: 100,
            height: 80,
            restore_window_label: Some("tray".into()),
        });

        let active = store.active().expect("session should exist");
        assert_eq!(active.id, "session-1");
        assert_eq!(active.width, 100);

        store.clear_active();
        assert!(store.active().is_none());
    }

    #[test]
    fn create_pin_label_is_unique_and_prefixed() {
        let first = create_pin_label();
        let second = create_pin_label();
        assert!(first.starts_with("pin-"));
        assert!(second.starts_with("pin-"));
        assert_ne!(first, second);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_png_clipboard_round_trips_on_private_pasteboard() {
        unsafe {
            let pool: *mut Object = msg_send![class!(NSAutoreleasePool), new];
            let pasteboard: *mut Object = msg_send![class!(NSPasteboard), pasteboardWithUniqueName];
            let png = b"\x89PNG\r\n\x1a\nclipboard-test";

            copy_png_to_pasteboard(png, pasteboard)
                .expect("native pasteboard write should succeed");

            let png_type: *mut Object = msg_send![
                class!(NSString),
                stringWithUTF8String:b"public.png\0".as_ptr()
            ];
            let data: *mut Object = msg_send![pasteboard, dataForType:png_type];
            assert!(!data.is_null());
            let data_bytes: *const u8 = msg_send![data, bytes];
            let data_length: usize = msg_send![data, length];
            let copied = std::slice::from_raw_parts(data_bytes, data_length);
            assert_eq!(copied, png);

            let _: () = msg_send![pasteboard, releaseGlobally];
            let _: () = msg_send![pool, drain];
        }
    }
}

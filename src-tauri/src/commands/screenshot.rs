use crate::services::screenshot::{
    cancel_screenshot_session as cancel_screenshot_session_service,
    init_pin_window as init_pin_window_service,
    init_screenshot_session as init_screenshot_session_service,
    prepare_screenshot_commit as prepare_screenshot_commit_service,
    read_screenshot_media as read_screenshot_media_service, screenshot_capabilities,
    start_screenshot_session, upload_screenshot_commit as upload_screenshot_commit_service,
    validate_screenshot_commit_request, CaptureSessionPayload, PinPayload,
    PrepareScreenshotCommitInput, ScreenshotCancelResult, ScreenshotCapabilities,
    ScreenshotCommitResult, ScreenshotError, ScreenshotMediaInput, ScreenshotStartResult,
    ScreenshotUploadLease, DEFAULT_SAVE_FILE_NAME, MAX_SCREENSHOT_PNG_BYTES,
};

#[tauri::command]
pub fn get_screenshot_capabilities() -> ScreenshotCapabilities {
    screenshot_capabilities()
}

#[tauri::command]
pub async fn start_screenshot(
    app: tauri::AppHandle,
    action: String,
) -> Result<ScreenshotStartResult, ScreenshotError> {
    tauri::async_runtime::spawn_blocking(move || start_screenshot_session(app, action))
        .await
        .map_err(|_| worker_error("screenshot.start_worker", "截图启动任务异常结束"))?
}

#[tauri::command]
pub fn init_screenshot_session(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    session_id: Option<String>,
) -> Result<CaptureSessionPayload, ScreenshotError> {
    let label = window.label().to_string();
    init_screenshot_session_service(app, &label, session_id)
}

#[tauri::command]
pub async fn read_screenshot_media(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    input: ScreenshotMediaInput,
) -> Result<tauri::ipc::Response, ScreenshotError> {
    let label = window.label().to_string();
    let read_app = app.clone();
    let bytes = tauri::async_runtime::spawn_blocking(move || {
        read_screenshot_media_service(read_app, &label, input)
    })
    .await
    .map_err(|_| ScreenshotError {
        code: "screenshot.read_worker".into(),
        message: "截图资源读取任务异常结束".into(),
        retryable: true,
    })??;
    Ok(tauri::ipc::Response::new(bytes))
}

#[tauri::command]
pub async fn prepare_screenshot_commit(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    input: PrepareScreenshotCommitInput,
) -> Result<ScreenshotUploadLease, ScreenshotError> {
    let label = window.label().to_string();
    validate_screenshot_commit_request(&app, &label, &input)?;
    let save_path = if input.action == "save" {
        rfd::AsyncFileDialog::new()
            .add_filter("PNG image", &["png"])
            .set_file_name(DEFAULT_SAVE_FILE_NAME)
            .save_file()
            .await
            .map(|handle| handle.path().to_path_buf())
    } else {
        None
    };
    prepare_screenshot_commit_service(app, &label, input, save_path)
}

#[tauri::command]
pub async fn upload_screenshot_commit(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    request: tauri::ipc::Request<'_>,
) -> Result<ScreenshotCommitResult, ScreenshotError> {
    let lease_token = screenshot_header(&request, "x-zero-screenshot-lease")?.to_string();
    let session_id =
        optional_screenshot_header(&request, "x-zero-screenshot-session").map(str::to_string);
    let action =
        optional_screenshot_header(&request, "x-zero-screenshot-action").map(str::to_string);
    let body = match request.body() {
        tauri::ipc::InvokeBody::Raw(bytes) if bytes.len() <= MAX_SCREENSHOT_PNG_BYTES => {
            Some(bytes.clone())
        }
        tauri::ipc::InvokeBody::Raw(_) => {
            return Err(ScreenshotError {
                code: "screenshot.png_size".into(),
                message: "PNG 数据大小超出允许范围".into(),
                retryable: false,
            });
        }
        tauri::ipc::InvokeBody::Json(_) => None,
    };
    let label = window.label().to_string();
    tauri::async_runtime::spawn_blocking(move || {
        upload_screenshot_commit_service(
            app,
            &label,
            &lease_token,
            session_id.as_deref(),
            action.as_deref(),
            body.as_deref(),
        )
    })
    .await
    .map_err(|_| worker_error("screenshot.commit_worker", "截图提交任务异常结束"))?
}

#[tauri::command]
pub async fn cancel_screenshot_session(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    session_id: String,
) -> Result<ScreenshotCancelResult, ScreenshotError> {
    let label = window.label().to_string();
    tauri::async_runtime::spawn_blocking(move || {
        cancel_screenshot_session_service(app, &label, session_id)
    })
    .await
    .map_err(|_| worker_error("screenshot.cancel_worker", "截图清理任务异常结束"))?
}

#[tauri::command]
pub fn init_pin_window(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> Result<PinPayload, ScreenshotError> {
    let label = window.label().to_string();
    init_pin_window_service(app, &label)
}

fn screenshot_header<'a>(
    request: &'a tauri::ipc::Request<'_>,
    name: &str,
) -> Result<&'a str, ScreenshotError> {
    optional_screenshot_header(request, name).ok_or_else(|| ScreenshotError {
        code: "screenshot.header".into(),
        message: format!("缺少截图提交请求头: {name}"),
        retryable: false,
    })
}

fn optional_screenshot_header<'a>(
    request: &'a tauri::ipc::Request<'_>,
    name: &str,
) -> Option<&'a str> {
    request
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty() && value.len() <= 128)
}

fn worker_error(code: &str, message: &str) -> ScreenshotError {
    ScreenshotError {
        code: code.into(),
        message: message.into(),
        retryable: true,
    }
}

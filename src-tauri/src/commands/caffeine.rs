use std::time::SystemTime;

use crate::services::caffeine::{CaffeineExpiry, CaffeineSnapshot, CaffeineState};
use tauri::Manager;

#[tauri::command]
pub fn get_caffeine_state(
    state: tauri::State<'_, CaffeineState>,
) -> Result<CaffeineSnapshot, String> {
    state.snapshot()
}

#[tauri::command]
pub async fn toggle_keep_awake(
    app: tauri::AppHandle,
    enabled: bool,
    duration_minutes: Option<u64>,
) -> Result<CaffeineSnapshot, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let transition = app
            .state::<CaffeineState>()
            .set_enabled(enabled, duration_minutes)?;
        if let Some(expiry) = transition.expiry {
            schedule_expiry(app.clone(), expiry);
        }
        let _ = crate::services::status_bar::refresh_status_bar(&app);
        Ok(transition.snapshot)
    })
    .await
    .map_err(|_| "防休眠状态任务异常结束".to_string())?
}

pub fn schedule_expiry(app: tauri::AppHandle, expiry: CaffeineExpiry) {
    std::thread::spawn(move || {
        if let Ok(delay) = expiry.expires_at.duration_since(SystemTime::now()) {
            std::thread::sleep(delay);
        }

        let state = app.state::<CaffeineState>();
        if state
            .expire_if_current(expiry.generation, SystemTime::now())
            .unwrap_or(false)
        {
            let _ = crate::services::status_bar::refresh_status_bar(&app);
        }
    });
}

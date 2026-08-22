use serde::Serialize;
use tauri::{Emitter, Manager};

pub const SURFACE_ACTIVITY_EVENT: &str = "zero://surface-activity";

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SurfaceActivityState {
    Active,
    Hidden,
    Disposed,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SurfaceActivityPayload {
    pub label: String,
    pub state: SurfaceActivityState,
}

impl SurfaceActivityPayload {
    fn new(label: impl Into<String>, state: SurfaceActivityState) -> Self {
        Self {
            label: label.into(),
            state,
        }
    }
}

pub fn current_surface_activity(
    window: &tauri::WebviewWindow,
) -> Result<SurfaceActivityPayload, String> {
    let state = if window
        .is_visible()
        .map_err(|error| format!("failed to read surface visibility: {error}"))?
    {
        SurfaceActivityState::Active
    } else {
        SurfaceActivityState::Hidden
    };
    Ok(SurfaceActivityPayload::new(window.label(), state))
}

pub fn show_surface(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    let phase = format!("surface_reveal:{}", window.label());
    let trace = window
        .app_handle()
        .try_state::<crate::services::performance::PerformanceTrace>();
    if let Some(trace) = &trace {
        trace.begin_pending(phase.clone());
    }
    let result = window
        .show()
        .and_then(|_| emit_to_surface(window, SurfaceActivityState::Active));
    if result.is_err() {
        if let Some(trace) = trace {
            trace.cancel_pending(&phase);
        }
    }
    result
}

pub fn hide_surface(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    window.hide()?;
    emit_to_surface(window, SurfaceActivityState::Hidden)
}

pub fn close_surface(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    let app = window.app_handle().clone();
    let payload = SurfaceActivityPayload::new(window.label(), SurfaceActivityState::Disposed);
    window.close()?;
    app.emit(SURFACE_ACTIVITY_EVENT, payload)
}

pub fn destroy_surface(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    let app = window.app_handle().clone();
    let payload = SurfaceActivityPayload::new(window.label(), SurfaceActivityState::Disposed);
    window.destroy()?;
    app.emit(SURFACE_ACTIVITY_EVENT, payload)
}

fn emit_to_surface(
    window: &tauri::WebviewWindow,
    state: SurfaceActivityState,
) -> tauri::Result<()> {
    window.emit(
        SURFACE_ACTIVITY_EVENT,
        SurfaceActivityPayload::new(window.label(), state),
    )
}

#[tauri::command]
pub fn get_surface_activity(
    window: tauri::WebviewWindow,
) -> Result<SurfaceActivityPayload, String> {
    current_surface_activity(&window)
}

#[tauri::command]
pub fn hide_current_surface(window: tauri::WebviewWindow) -> Result<(), String> {
    hide_surface(&window).map_err(|error| format!("failed to hide surface: {error}"))
}

#[tauri::command]
pub fn close_current_surface(window: tauri::WebviewWindow) -> Result<(), String> {
    close_surface(&window).map_err(|error| format!("failed to close surface: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_activity_contract_is_camel_case_and_lowercase() {
        let payload = SurfaceActivityPayload::new("tray", SurfaceActivityState::Active);
        assert_eq!(
            serde_json::to_value(payload).unwrap(),
            serde_json::json!({ "label": "tray", "state": "active" })
        );
    }

    #[test]
    fn surface_activity_event_name_is_host_owned() {
        assert_eq!(SURFACE_ACTIVITY_EVENT, "zero://surface-activity");
    }
}

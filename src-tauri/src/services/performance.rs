use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformancePhaseEvent {
    pub sequence: u64,
    pub phase: String,
    pub outcome: String,
    pub started_at_us: u64,
    pub duration_us: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

#[derive(Clone)]
pub struct PerformanceTrace {
    inner: Arc<PerformanceTraceInner>,
}

struct PerformanceTraceInner {
    origin: Instant,
    events: Mutex<Vec<PerformancePhaseEvent>>,
    pending: Mutex<HashMap<String, Instant>>,
    emit_logs: bool,
}

impl Default for PerformanceTrace {
    fn default() -> Self {
        Self::new(std::env::var("ZERO_PERFORMANCE_TRACE").is_ok_and(|value| value == "1"))
    }
}

impl PerformanceTrace {
    pub fn new(emit_logs: bool) -> Self {
        Self {
            inner: Arc::new(PerformanceTraceInner {
                origin: Instant::now(),
                events: Mutex::new(Vec::new()),
                pending: Mutex::new(HashMap::new()),
                emit_logs,
            }),
        }
    }

    pub fn begin(&self) -> Instant {
        Instant::now()
    }

    pub fn finish(&self, phase: &str, outcome: &str, started: Instant) {
        self.push(phase, outcome, started, started.elapsed(), None, None);
    }

    pub fn mark(&self, phase: &str, outcome: &str) {
        self.push(phase, outcome, Instant::now(), Duration::ZERO, None, None);
    }

    pub fn finish_since_origin(&self, phase: &str, outcome: &str) {
        self.push(
            phase,
            outcome,
            self.inner.origin,
            self.inner.origin.elapsed(),
            None,
            None,
        );
    }

    pub fn record_duration(&self, phase: &str, outcome: &str, duration: Duration) {
        let started = Instant::now()
            .checked_sub(duration)
            .unwrap_or(self.inner.origin);
        self.push(phase, outcome, started, duration, None, None);
    }

    pub fn measure(&self, phase: &str, value: u64, unit: &str) {
        self.push(
            phase,
            "ok",
            Instant::now(),
            Duration::ZERO,
            Some(value),
            Some(unit.into()),
        );
    }

    pub fn begin_pending(&self, phase: impl Into<String>) {
        if let Ok(mut pending) = self.inner.pending.lock() {
            pending.insert(phase.into(), Instant::now());
        }
    }

    pub fn finish_pending(&self, phase: &str, outcome: &str) -> bool {
        let started = self
            .inner
            .pending
            .lock()
            .ok()
            .and_then(|mut pending| pending.remove(phase));
        if let Some(started) = started {
            self.finish(phase, outcome, started);
            true
        } else {
            false
        }
    }

    pub fn cancel_pending(&self, phase: &str) {
        if let Ok(mut pending) = self.inner.pending.lock() {
            pending.remove(phase);
        }
    }

    pub fn snapshot(&self) -> Vec<PerformancePhaseEvent> {
        self.inner
            .events
            .lock()
            .map(|events| events.clone())
            .unwrap_or_default()
    }

    pub fn emits_logs(&self) -> bool {
        self.inner.emit_logs
    }

    fn push(
        &self,
        phase: &str,
        outcome: &str,
        started: Instant,
        duration: Duration,
        value: Option<u64>,
        unit: Option<String>,
    ) {
        let event = {
            let Ok(mut events) = self.inner.events.lock() else {
                return;
            };
            let event = PerformancePhaseEvent {
                sequence: events.len() as u64 + 1,
                phase: phase.into(),
                outcome: outcome.into(),
                started_at_us: duration_us(started.saturating_duration_since(self.inner.origin)),
                duration_us: duration_us(duration),
                value,
                unit,
            };
            events.push(event.clone());
            event
        };
        if self.inner.emit_logs {
            if let Ok(json) = serde_json::to_string(&event) {
                eprintln!("ZERO_PERF {json}");
            }
        }
    }
}

fn duration_us(duration: Duration) -> u64 {
    duration.as_micros().try_into().unwrap_or(u64::MAX)
}

pub fn record_media_transfer(app: &tauri::AppHandle, channel: &str, bytes: usize) {
    use tauri::Manager;

    if let Some(trace) = app.try_state::<PerformanceTrace>() {
        trace.measure(
            &format!("media_transfer:{channel}"),
            bytes.try_into().unwrap_or(u64::MAX),
            "bytes",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_events_are_monotonic_and_keep_error_outcomes() {
        let trace = PerformanceTrace::new(false);
        let started = trace.begin();
        trace.finish("migration", "ok", started);
        trace.mark("registry_load_write", "error");

        let events = trace.snapshot();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].sequence, 1);
        assert_eq!(events[1].sequence, 2);
        assert!(events[1].started_at_us >= events[0].started_at_us);
        assert_eq!(events[1].outcome, "error");
    }

    #[test]
    fn pending_reveals_measure_once_and_missing_acknowledgements_are_safe() {
        let trace = PerformanceTrace::new(false);
        trace.begin_pending("surface_reveal:tray");
        assert!(trace.finish_pending("surface_reveal:tray", "ok"));
        assert!(!trace.finish_pending("surface_reveal:tray", "late"));
        assert_eq!(trace.snapshot().len(), 1);
    }

    #[test]
    fn byte_measurements_retain_value_and_unit() {
        let trace = PerformanceTrace::new(false);
        trace.measure("media_transfer:screenshot_upload", 4096, "bytes");
        let event = trace.snapshot().pop().expect("measurement should exist");
        assert_eq!(event.value, Some(4096));
        assert_eq!(event.unit.as_deref(), Some("bytes"));
    }
}

use super::{ScreenshotSourceBounds, ScreenshotTargetCandidate, ScreenshotTargetKind};

const MINIMUM_TARGET_DIMENSION: u32 = 8;

#[derive(Debug, Clone, Copy, PartialEq)]
struct NativeRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Clone, PartialEq)]
struct NativeWindowSnapshot {
    native_id: u32,
    process_id: u32,
    app_name: String,
    title: String,
    bounds: NativeRect,
    minimized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct NativeCaptureGeometry {
    monitor: NativeRect,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct CaptureTargetSnapshot {
    geometry: NativeCaptureGeometry,
    windows: Vec<NativeWindowSnapshot>,
}

pub(super) fn prepare_capture_target_snapshot() -> Result<CaptureTargetSnapshot, String> {
    let monitor = xcap::Monitor::all()
        .map_err(|error| format!("list monitors: {error}"))?
        .into_iter()
        .find(|monitor| monitor.is_primary().unwrap_or(false))
        .ok_or_else(|| "primary monitor is unavailable".to_string())?;
    let geometry = NativeCaptureGeometry {
        monitor: NativeRect {
            x: monitor.x().map_err(|error| format!("monitor x: {error}"))? as f64,
            y: monitor.y().map_err(|error| format!("monitor y: {error}"))? as f64,
            width: monitor
                .width()
                .map_err(|error| format!("monitor width: {error}"))? as f64,
            height: monitor
                .height()
                .map_err(|error| format!("monitor height: {error}"))? as f64,
        },
    };

    let windows = xcap::Window::all()
        .map_err(|error| format!("list windows: {error}"))?
        .into_iter()
        .filter_map(|window| {
            let snapshot = (|| {
                Ok::<_, xcap::XCapError>(NativeWindowSnapshot {
                    native_id: window.id()?,
                    process_id: window.pid()?,
                    app_name: window.app_name()?,
                    title: window.title()?,
                    bounds: NativeRect {
                        x: window.x()? as f64,
                        y: window.y()? as f64,
                        width: window.width()? as f64,
                        height: window.height()? as f64,
                    },
                    minimized: window.is_minimized()?,
                })
            })();
            match snapshot {
                Ok(snapshot) => Some(snapshot),
                Err(error) => {
                    eprintln!("Zero Snap skipped unreadable window candidate: {error}");
                    None
                }
            }
        })
        .collect();
    Ok(CaptureTargetSnapshot { geometry, windows })
}

pub(super) fn resolve_capture_targets(
    snapshot: &CaptureTargetSnapshot,
    image_width: u32,
    image_height: u32,
    current_process_id: u32,
) -> Vec<ScreenshotTargetCandidate> {
    snapshot
        .windows
        .iter()
        .filter(|window| is_window_candidate(window, snapshot.geometry, current_process_id))
        .filter_map(|window| {
            source_bounds(window.bounds, snapshot.geometry, image_width, image_height)
        })
        .enumerate()
        .map(|(index, bounds)| ScreenshotTargetCandidate {
            id: format!("target-{index}"),
            kind: ScreenshotTargetKind::Window,
            bounds,
        })
        .collect()
}

fn is_window_candidate(
    window: &NativeWindowSnapshot,
    geometry: NativeCaptureGeometry,
    current_process_id: u32,
) -> bool {
    if window.process_id == current_process_id
        || window.minimized
        || window.bounds.width <= 0.0
        || window.bounds.height <= 0.0
        || intersection(window.bounds, geometry.monitor).is_none()
    {
        return false;
    }
    let app_name = window.app_name.trim();
    let is_desktop =
        (app_name == "Finder" && window.title.trim().is_empty()) || app_name == "Window Server";
    !is_desktop
}

fn source_bounds(
    window: NativeRect,
    geometry: NativeCaptureGeometry,
    image_width: u32,
    image_height: u32,
) -> Option<ScreenshotSourceBounds> {
    if image_width == 0
        || image_height == 0
        || geometry.monitor.width <= 0.0
        || geometry.monitor.height <= 0.0
    {
        return None;
    }
    let clipped = intersection(window, geometry.monitor)?;
    let scale_x = image_width as f64 / geometry.monitor.width;
    let scale_y = image_height as f64 / geometry.monitor.height;
    let left = ((clipped.x - geometry.monitor.x) * scale_x)
        .floor()
        .clamp(0.0, image_width as f64) as u32;
    let top = ((clipped.y - geometry.monitor.y) * scale_y)
        .floor()
        .clamp(0.0, image_height as f64) as u32;
    let right = ((clipped.x + clipped.width - geometry.monitor.x) * scale_x)
        .ceil()
        .clamp(0.0, image_width as f64) as u32;
    let bottom = ((clipped.y + clipped.height - geometry.monitor.y) * scale_y)
        .ceil()
        .clamp(0.0, image_height as f64) as u32;
    let width = right.saturating_sub(left);
    let height = bottom.saturating_sub(top);
    (width >= MINIMUM_TARGET_DIMENSION && height >= MINIMUM_TARGET_DIMENSION).then_some(
        ScreenshotSourceBounds {
            x: left,
            y: top,
            width,
            height,
        },
    )
}

fn intersection(left: NativeRect, right: NativeRect) -> Option<NativeRect> {
    let x = left.x.max(right.x);
    let y = left.y.max(right.y);
    let far_x = (left.x + left.width).min(right.x + right.width);
    let far_y = (left.y + left.height).min(right.y + right.height);
    (far_x > x && far_y > y).then_some(NativeRect {
        x,
        y,
        width: far_x - x,
        height: far_y - y,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geometry() -> NativeCaptureGeometry {
        NativeCaptureGeometry {
            monitor: NativeRect {
                x: -1440.0,
                y: 0.0,
                width: 1440.0,
                height: 900.0,
            },
        }
    }

    fn window(native_id: u32, bounds: NativeRect) -> NativeWindowSnapshot {
        NativeWindowSnapshot {
            native_id,
            process_id: native_id + 100,
            app_name: "Example".into(),
            title: format!("Window {native_id}"),
            bounds,
            minimized: false,
        }
    }

    #[test]
    fn converts_negative_monitor_coordinates_with_outside_rounding() {
        let bounds = source_bounds(
            NativeRect {
                x: -1340.25,
                y: 100.25,
                width: 400.5,
                height: 300.5,
            },
            geometry(),
            2880,
            1800,
        )
        .expect("source bounds");
        assert_eq!(
            bounds,
            ScreenshotSourceBounds {
                x: 199,
                y: 200,
                width: 802,
                height: 602,
            }
        );
    }

    #[test]
    fn normalizes_vertically_stacked_monitor_origins_to_the_capture_image() {
        for monitor_y in [-900.0, 900.0] {
            let stacked_geometry = NativeCaptureGeometry {
                monitor: NativeRect {
                    x: 0.0,
                    y: monitor_y,
                    width: 1440.0,
                    height: 900.0,
                },
            };
            assert_eq!(
                source_bounds(
                    NativeRect {
                        x: 100.0,
                        y: monitor_y + 100.0,
                        width: 400.0,
                        height: 300.0,
                    },
                    stacked_geometry,
                    2880,
                    1800,
                ),
                Some(ScreenshotSourceBounds {
                    x: 200,
                    y: 200,
                    width: 800,
                    height: 600,
                })
            );
        }
    }

    #[test]
    fn clips_partial_windows_and_rejects_small_or_outside_rectangles() {
        assert_eq!(
            source_bounds(
                NativeRect {
                    x: -1540.0,
                    y: 850.0,
                    width: 300.0,
                    height: 200.0,
                },
                geometry(),
                1440,
                900,
            ),
            Some(ScreenshotSourceBounds {
                x: 0,
                y: 850,
                width: 200,
                height: 50,
            })
        );
        assert!(source_bounds(
            NativeRect {
                x: 10.0,
                y: 10.0,
                width: 100.0,
                height: 100.0,
            },
            geometry(),
            1440,
            900,
        )
        .is_none());
    }

    #[test]
    fn filters_current_process_minimized_desktop_and_invalid_windows() {
        let bounds = NativeRect {
            x: -1000.0,
            y: 100.0,
            width: 300.0,
            height: 200.0,
        };
        let mut own = window(1, bounds);
        own.process_id = 42;
        assert!(!is_window_candidate(&own, geometry(), 42));
        let mut minimized = window(2, bounds);
        minimized.minimized = true;
        assert!(!is_window_candidate(&minimized, geometry(), 42));
        let mut desktop = window(3, geometry().monitor);
        desktop.app_name = "Finder".into();
        desktop.title.clear();
        assert!(!is_window_candidate(&desktop, geometry(), 42));
        assert!(is_window_candidate(&window(4, bounds), geometry(), 42));
    }

    #[test]
    fn preserves_front_to_back_order_and_assigns_opaque_ids() {
        let snapshot = CaptureTargetSnapshot {
            geometry: geometry(),
            windows: vec![
                window(
                    99,
                    NativeRect {
                        x: -1200.0,
                        y: 100.0,
                        width: 400.0,
                        height: 300.0,
                    },
                ),
                window(
                    7,
                    NativeRect {
                        x: -1000.0,
                        y: 150.0,
                        width: 500.0,
                        height: 400.0,
                    },
                ),
            ],
        };
        let targets = resolve_capture_targets(&snapshot, 1440, 900, 42);
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].id, "target-0");
        assert_eq!(targets[1].id, "target-1");
        assert!(!targets[0].id.contains("99"));
    }
}

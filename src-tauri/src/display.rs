use crate::policy::{render_size, target_fullscreen, RESOLUTIONS};
use serde::{Deserialize, Serialize};
use tauri::{PhysicalPosition, PhysicalSize, WebviewWindow};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayState {
    mode: &'static str,
    size: String,
    sizes: Vec<String>,
    monitor: usize,
    monitors: Vec<MonitorInfo>,
    vsync: bool,
    vsync_next: bool,
    angle: &'static str,
    angle_next: &'static str,
    angles: [&'static str; 1],
    discord: bool,
}

#[derive(Serialize)]
struct MonitorInfo {
    label: String,
    primary: bool,
}

#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct DisplayPatch {
    mode: Option<String>,
    size: Option<String>,
    monitor: Option<usize>,
    vsync: Option<bool>,
    angle: Option<String>,
    discord: Option<bool>,
}

pub fn read(window: &WebviewWindow) -> Result<DisplayState, String> {
    let mut monitors = window.available_monitors().map_err(|e| e.to_string())?;
    monitors.sort_by_key(|m| (m.position().x, m.position().y));
    let current = window.current_monitor().map_err(|e| e.to_string())?;
    let primary = window.primary_monitor().map_err(|e| e.to_string())?;
    let monitor = current
        .as_ref()
        .and_then(|m| monitors.iter().position(|v| v.position() == m.position()))
        .unwrap_or(0);
    let size = window.inner_size().map_err(|e| e.to_string())?;
    let sizes = monitors
        .get(monitor)
        .map(|m| {
            RESOLUTIONS
                .iter()
                .filter(|(w, h)| {
                    let scale = m.scale_factor();
                    *w as f64 >= 960.0 * scale
                        && *h as f64 >= 540.0 * scale
                        && *w <= m.size().width
                        && *h as f64 <= m.size().height as f64 - 60.0 * scale
                })
                .map(|(w, h)| format!("{w}x{h}"))
                .collect()
        })
        .unwrap_or_default();
    Ok(DisplayState {
        mode: if window.is_fullscreen().map_err(|e| e.to_string())? {
            "fullscreen"
        } else {
            "window"
        },
        size: if window.is_maximized().map_err(|e| e.to_string())? {
            "max".into()
        } else {
            format!("{}x{}", size.width, size.height)
        },
        sizes,
        monitor,
        monitors: monitors
            .iter()
            .map(|m| MonitorInfo {
                label: format!("{}×{}", m.size().width, m.size().height),
                primary: primary
                    .as_ref()
                    .is_some_and(|p| p.position() == m.position()),
            })
            .collect(),
        vsync: true,
        vsync_next: true,
        angle: "auto",
        angle_next: "auto",
        angles: ["auto"],
        discord: false,
    })
}

pub fn apply(window: &WebviewWindow, patch: DisplayPatch) -> Result<DisplayState, String> {
    if patch
        .mode
        .as_deref()
        .is_some_and(|m| !matches!(m, "window" | "fullscreen"))
    {
        return Err("Unknown window mode".into());
    }
    if patch.vsync == Some(false) || patch.angle.as_deref().is_some_and(|a| a != "auto") {
        return Err("Graphics and V-Sync are managed by macOS WebKit".into());
    }
    if patch.discord == Some(true) {
        return Err("Discord integration is not included in this Rust client".into());
    }
    let requested_size = match patch.size.as_deref() {
        None | Some("max") => None,
        Some(value) => Some(render_size(value).ok_or("Unsupported resolution")?),
    };
    let mut monitors = window.available_monitors().map_err(|e| e.to_string())?;
    monitors.sort_by_key(|m| (m.position().x, m.position().y));
    if patch.monitor.is_some_and(|i| i >= monitors.len()) {
        return Err("Monitor is no longer available".into());
    }
    let fullscreen = target_fullscreen(
        patch.mode.as_deref(),
        patch.size.is_some(),
        window.is_fullscreen().map_err(|e| e.to_string())?,
    );
    if let Some(index) = patch.monitor {
        window.set_fullscreen(false).map_err(|e| e.to_string())?;
        let m = &monitors[index];
        window
            .set_position(PhysicalPosition::new(
                m.position().x + 40,
                m.position().y + 60,
            ))
            .map_err(|e| e.to_string())?;
    }
    if patch.size.is_some() || patch.mode.as_deref() == Some("window") {
        window.set_fullscreen(false).map_err(|e| e.to_string())?;
    }
    if patch.size.as_deref() == Some("max") {
        window.maximize().map_err(|e| e.to_string())?;
    } else if let Some((width, height)) = requested_size {
        window.unmaximize().map_err(|e| e.to_string())?;
        window
            .set_size(PhysicalSize::new(width, height))
            .map_err(|e| e.to_string())?;
        window.center().map_err(|e| e.to_string())?;
    }
    if fullscreen {
        window.set_fullscreen(true).map_err(|e| e.to_string())?;
    }
    read(window)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_patch_rejects_unrelated_commands_and_invalid_monitor_types() {
        for value in [
            r#"{"path":"/tmp/game"}"#,
            r#"{"monitor":-1}"#,
            r#"{"size":0}"#,
        ] {
            assert!(serde_json::from_str::<DisplayPatch>(value).is_err());
        }
        assert!(serde_json::from_str::<DisplayPatch>(r#"{"mode":"window","monitor":0}"#).is_ok());
    }
}

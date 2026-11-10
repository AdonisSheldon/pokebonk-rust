pub const RESOLUTIONS: [(u32, u32); 6] = [
    (1280, 720),
    (1600, 900),
    (1920, 1080),
    (2560, 1440),
    (3200, 1800),
    (3840, 2160),
];

pub fn game_origin(scheme: &str, host: &str, port: Option<u16>) -> bool {
    scheme == "https"
        && matches!(host, "pokebonk.com" | "www.pokebonk.com")
        && matches!(port, None | Some(443))
}

pub fn render_size(value: &str) -> Option<(u32, u32)> {
    let (width, height) = value.split_once('x')?;
    let size = (width.parse().ok()?, height.parse().ok()?);
    RESOLUTIONS.contains(&size).then_some(size)
}

pub fn target_fullscreen(mode: Option<&str>, resizing: bool, current: bool) -> bool {
    match mode {
        Some("fullscreen") => true,
        Some("window") => false,
        _ => current && !resizing,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_official_https_origins_receive_desktop_access() {
        assert!(game_origin("https", "pokebonk.com", None));
        assert!(game_origin("https", "www.pokebonk.com", Some(443)));
        for (scheme, host, port) in [
            ("http", "pokebonk.com", None),
            ("https", "pokebonk.com.attacker.test", None),
            ("https", "fakepokebonk.com", None),
            ("https", "api.pokebonk.com", None),
            ("https", "pokebonk.com", Some(8443)),
        ] {
            assert!(!game_origin(scheme, host, port), "{scheme}://{host}");
        }
    }

    #[test]
    fn window_size_accepts_only_supported_render_resolutions() {
        assert_eq!(render_size("1280x720"), Some((1280, 720)));
        assert_eq!(render_size("3840x2160"), Some((3840, 2160)));
        for value in [
            "0x0",
            "-1x720",
            "999999x999999",
            "1280x720;open",
            "NaN",
            "max",
        ] {
            assert_eq!(render_size(value), None, "{value}");
        }
    }

    #[test]
    fn monitor_changes_preserve_fullscreen_but_window_sizes_leave_it() {
        assert!(target_fullscreen(None, false, true));
        assert!(!target_fullscreen(None, true, true));
        assert!(!target_fullscreen(Some("window"), false, true));
        assert!(target_fullscreen(Some("fullscreen"), false, false));
    }
}

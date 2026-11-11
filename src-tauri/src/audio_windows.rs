use super::mute::FocusMute;
use std::sync::Mutex;
use tauri::Manager;
use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_8;
use windows_core::{Interface, BOOL};

#[derive(Default)]
pub struct FocusAudio(Mutex<FocusMute>);

pub fn sync(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    let focused_window = window.clone();
    let app = window.app_handle().clone();
    window.with_webview(move |platform| {
        let update = || -> Result<(), Box<dyn std::error::Error>> {
            let focused = focused_window.is_focused()?;
            // Tauri keeps the controller live and runs this callback on its UI thread.
            let view: ICoreWebView2_8 = unsafe { platform.controller().CoreWebView2()? }.cast()?;
            let mut current = BOOL::default();
            unsafe { view.IsMuted(&mut current)? };
            let state = app.state::<FocusAudio>();
            state
                .0
                .lock()
                .map_err(|_| "Audio mute state lock poisoned")?
                .sync(
                    focused,
                    current.as_bool(),
                    // IsMuted gates all output without altering page volume or playback state.
                    // https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2_8
                    |muted| unsafe { view.SetIsMuted(muted) },
                )?;
            Ok(())
        };
        if let Err(error) = update() {
            eprintln!("Could not update WebView2 focus audio: {error}");
        }
    })
}

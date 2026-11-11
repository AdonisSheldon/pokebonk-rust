#[cfg(any(windows, test))]
#[path = "audio_mute.rs"]
mod mute;
#[cfg(windows)]
#[path = "audio_windows.rs"]
mod windows;
#[cfg(windows)]
pub use windows::{sync, FocusAudio};

#[cfg(target_os = "macos")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "macos")]
use tauri::Manager;

#[cfg(target_os = "macos")]
#[derive(Default)]
pub struct FocusAudio {
    suspended: AtomicBool,
}

#[cfg(target_os = "macos")]
pub fn sync(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    let handle = window.app_handle().clone();
    window.with_webview(move |platform| {
        let main_thread = objc2::MainThreadMarker::new().expect("Tauri main-thread callback");
        let active = objc2_app_kit::NSApplication::sharedApplication(main_thread).isActive();
        // Read native state here so queued focus changes cannot apply stale values.
        let window: &objc2_app_kit::NSWindow = unsafe { &*platform.ns_window().cast() };
        let suspended = !window.isKeyWindow() || !active;
        let state = handle.state::<FocusAudio>();
        if state.suspended.swap(suspended, Ordering::SeqCst) == suspended {
            return;
        }
        // Tauri provides a live WKWebView and runs this closure on the main thread.
        // This public API covers HTML media and Web Audio, without changing volumes.
        // https://developer.apple.com/documentation/webkit/wkwebview/setallmediaplaybacksuspended(_:completionhandler:)
        unsafe {
            let view: &objc2_web_kit::WKWebView = &*platform.inner().cast();
            view.setAllMediaPlaybackSuspended_completionHandler(suspended, None);
        }
    })
}

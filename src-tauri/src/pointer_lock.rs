use block2::RcBlock;
use core_graphics::display::CGDisplay;
use objc2::{rc::Retained, runtime::AnyObject};
use objc2_app_kit::{
    NSApplication, NSCursor, NSEvent, NSEventMask, NSEventModifierFlags, NSEventType,
};
use std::{
    cell::RefCell,
    ptr::NonNull,
    sync::atomic::{AtomicBool, Ordering},
};
use tauri::{Manager, WebviewWindow, WindowEvent};

#[derive(Default)]
struct PointerLock(AtomicBool);

thread_local! {
    static MONITOR: RefCell<Option<Retained<AnyObject>>> = const { RefCell::new(None) };
}

// All cursor operations and local monitor callbacks run on AppKit's main thread.
fn release_now(window: &WebviewWindow) {
    if !window
        .state::<PointerLock>()
        .0
        .swap(false, Ordering::SeqCst)
    {
        return;
    }
    if let Err(error) = CGDisplay::associate_mouse_and_mouse_cursor_position(true) {
        eprintln!("Could not restore mouse association: {error:?}");
    }
    NSCursor::unhide();
    let _ = window.eval("window.__pokebonkPointerLock?.release()");
}

pub fn release(window: &WebviewWindow) {
    let handle = window.clone();
    let _ = window
        .app_handle()
        .run_on_main_thread(move || release_now(&handle));
}

pub async fn set(window: &WebviewWindow, locked: bool) -> Result<(), String> {
    let (send, receive) = tokio::sync::oneshot::channel();
    let handle = window.clone();
    window
        .with_webview(move |platform| {
            let result = (|| {
                if !locked {
                    release_now(&handle);
                    return Ok(());
                }
                let thread = objc2::MainThreadMarker::new().expect("Tauri main-thread callback");
                let native = unsafe { &*platform.ns_window().cast::<objc2_app_kit::NSWindow>() };
                if !native.isKeyWindow() || !NSApplication::sharedApplication(thread).isActive() {
                    return Err("Pointer lock requires the active game window".into());
                }
                if MONITOR.with(|stored| stored.borrow().is_none()) {
                    return Err("Native pointer event monitor is unavailable".into());
                }
                let state = handle.state::<PointerLock>();
                if state.0.load(Ordering::SeqCst) {
                    return Ok(());
                }
                let size = handle.inner_size().map_err(|error| error.to_string())?;
                handle
                    .set_cursor_position(tauri::PhysicalPosition::new(
                        size.width / 2,
                        size.height / 2,
                    ))
                    .map_err(|error| error.to_string())?;
                CGDisplay::associate_mouse_and_mouse_cursor_position(false)
                    .map_err(|error| format!("Could not capture mouse: {error:?}"))?;
                NSCursor::hide();
                state.0.store(true, Ordering::SeqCst);
                Ok(())
            })();
            let _ = send.send(result);
        })
        .map_err(|error| error.to_string())?;
    receive.await.map_err(|error| error.to_string())?
}

pub fn install(window: &WebviewWindow) -> tauri::Result<()> {
    window.app_handle().manage(PointerLock::default());
    let handle = window.clone();
    window.with_webview(move |platform| {
        let native = unsafe { &*platform.ns_window().cast::<objc2_app_kit::NSWindow>() };
        let number = native.windowNumber();
        let block = RcBlock::new(move |event: NonNull<NSEvent>| {
            let value = unsafe { event.as_ref() };
            if !handle.state::<PointerLock>().0.load(Ordering::SeqCst) {
                return event.as_ptr();
            }
            let thread = objc2::MainThreadMarker::new().expect("AppKit event callback");
            let app = NSApplication::sharedApplication(thread);
            if !app.isActive() || !app.keyWindow().is_some_and(|window| window.windowNumber() == number) {
                release_now(&handle);
                return event.as_ptr();
            }
            // Relative mouse events may not carry an NSWindow; the active key window still scopes them.
            if value.windowNumber() != 0 && value.windowNumber() != number {
                return event.as_ptr();
            }
            if value.r#type() == NSEventType::KeyDown {
                if value.keyCode() == 53 {
                    release_now(&handle);
                    return std::ptr::null_mut();
                }
                return event.as_ptr();
            }
            let dx = value.deltaX();
            let dy = value.deltaY();
            if dx.is_finite() && dy.is_finite() && (dx != 0.0 || dy != 0.0) {
                let buttons = NSEvent::pressedMouseButtons();
                let flags = value.modifierFlags();
                let shift = flags.contains(NSEventModifierFlags::Shift);
                let control = flags.contains(NSEventModifierFlags::Control);
                let alt = flags.contains(NSEventModifierFlags::Option);
                let meta = flags.contains(NSEventModifierFlags::Command);
                let _ = handle.eval(format!(
                    "window.__pokebonkPointerLock?.move({dx},{dy},{buttons},{{shiftKey:{shift},ctrlKey:{control},altKey:{alt},metaKey:{meta}}})"
                ));
            }
            // Deliver one relative event through the bridge, not a second fixed-position event.
            std::ptr::null_mut()
        });
        let mask = NSEventMask::MouseMoved
            | NSEventMask::LeftMouseDragged
            | NSEventMask::RightMouseDragged
            | NSEventMask::OtherMouseDragged
            | NSEventMask::KeyDown;
        let monitor =
            unsafe { NSEvent::addLocalMonitorForEventsMatchingMask_handler(mask, &block) };
        MONITOR.with(|stored| *stored.borrow_mut() = monitor);
    })?;
    let handle = window.clone();
    window.on_window_event(move |event| {
        if matches!(event, WindowEvent::Focused(false) | WindowEvent::Destroyed) {
            release(&handle);
        }
        if matches!(event, WindowEvent::Destroyed) {
            let _ = handle.app_handle().run_on_main_thread(|| {
                MONITOR.with(|stored| {
                    if let Some(monitor) = stored.borrow_mut().take() {
                        unsafe { NSEvent::removeMonitor(&monitor) };
                    }
                });
            });
        }
    });
    Ok(())
}

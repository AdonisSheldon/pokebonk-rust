#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod display;
mod locale;
#[cfg(target_os = "macos")]
mod pointer_lock;
mod policy;
mod update_progress;
mod updater;

use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    menu::{MenuBuilder, MenuItem, SubmenuBuilder},
    webview::NewWindowResponse,
    AppHandle, Manager, WebviewUrl, WebviewWindow, WindowEvent,
};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tauri_plugin_opener::OpenerExt;

const GAME: &str = "https://pokebonk.com";

#[derive(Default)]
struct ExitState {
    allowed: AtomicBool,
    confirming: AtomicBool,
}

fn is_game(url: &tauri::Url) -> bool {
    policy::game_origin(url.scheme(), url.host_str().unwrap_or(""), url.port())
}

fn check_sender(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == "main" && is_game(&window.url().map_err(|e| e.to_string())?) {
        Ok(())
    } else {
        Err("Desktop commands are restricted to the official game".into())
    }
}

#[tauri::command]
async fn pointer_lock(window: WebviewWindow, locked: bool) -> Result<(), String> {
    check_sender(&window)?;
    #[cfg(target_os = "macos")]
    return pointer_lock::set(&window, locked).await;
    #[cfg(not(target_os = "macos"))]
    {
        let _ = locked;
        Err("Native pointer lock is only needed by macOS WKWebView".into())
    }
}

#[tauri::command]
async fn display(window: WebviewWindow) -> Result<display::DisplayState, String> {
    check_sender(&window)?;
    display::read(&window)
}

#[tauri::command]
async fn set_display(
    window: WebviewWindow,
    patch: display::DisplayPatch,
) -> Result<display::DisplayState, String> {
    check_sender(&window)?;
    display::apply(&window, patch)
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Action {
    Quit,
    Reload,
    Reconnect,
    Relaunch,
}

fn request_action(app: &AppHandle, action: Action) {
    let language = *app.state::<locale::Language>();
    let state = app.state::<ExitState>();
    if state.confirming.swap(true, Ordering::SeqCst) {
        return;
    }
    let message = match action {
        Action::Quit => language.text(
            "退出游戏？请先在游戏中保存当前对局。",
            "Quit the game? Save your current run first.",
        ),
        Action::Relaunch => language.text(
            "重新启动？请先保存当前对局。",
            "Restart the client? Save your current run first.",
        ),
        _ => language.text(
            "重新加载游戏？未保存的对局可能中断。",
            "Reload the game? Unsaved play may be interrupted.",
        ),
    };
    let handle = app.clone();
    let mut dialog = app
        .dialog()
        .message(message)
        .title("PokeBonk Rust")
        .buttons(MessageDialogButtons::OkCancelCustom(
            language.text("继续", "Continue").into(),
            language.text("取消", "Cancel").into(),
        ));
    if let Some(window) = app.get_webview_window("main") {
        dialog = dialog.parent(&window);
    }
    dialog.show(move |confirmed| {
        handle
            .state::<ExitState>()
            .confirming
            .store(false, Ordering::SeqCst);
        if !confirmed {
            return;
        }
        match action {
            Action::Quit => {
                handle
                    .state::<ExitState>()
                    .allowed
                    .store(true, Ordering::SeqCst);
                handle.exit(0);
            }
            Action::Relaunch => {
                handle
                    .state::<ExitState>()
                    .allowed
                    .store(true, Ordering::SeqCst);
                handle.restart();
            }
            Action::Reload | Action::Reconnect => {
                if let Some(window) = handle.get_webview_window("main") {
                    if let Err(error) = window.navigate(GAME.parse().expect("static game URL")) {
                        eprintln!("Could not reconnect: {error}");
                    }
                }
            }
        }
    });
}

#[tauri::command]
async fn native_action(window: WebviewWindow, action: Action) -> Result<(), String> {
    check_sender(&window)?;
    request_action(window.app_handle(), action);
    Ok(())
}

fn open_external(app: &AppHandle, url: &tauri::Url) {
    if matches!(url.scheme(), "https" | "http" | "mailto") {
        if let Err(error) = app.opener().open_url(url.as_str(), None::<&str>) {
            eprintln!("Could not open external link: {error}");
        }
    }
}

fn main() {
    let app = tauri::Builder::default()
        // Native-only plugins: none of their commands is granted to the website.
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_denylist(&[update_progress::LABEL])
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::SIZE
                        | tauri_plugin_window_state::StateFlags::POSITION
                        | tauri_plugin_window_state::StateFlags::MAXIMIZED
                        | tauri_plugin_window_state::StateFlags::FULLSCREEN,
                )
                .build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(
            tauri_plugin_opener::Builder::new()
                .open_js_links_on_click(false)
                .build(),
        )
        .manage(ExitState::default())
        .manage(locale::Language::system())
        .manage(audio::FocusAudio::default())
        .invoke_handler(tauri::generate_handler![
            display,
            set_display,
            native_action,
            pointer_lock
        ])
        .setup(|app| {
            let language = *app.state::<locale::Language>();
            let check_updates = MenuItem::with_id(
                app,
                "check-updates",
                language.text("检查更新…", "Check for Updates…"),
                true,
                None::<&str>,
            )?;
            app.manage(updater::UpdaterState::new(check_updates.clone()));
            let app_menu = SubmenuBuilder::new(app, "PokeBonk Rust")
                .about_with_text(language.text("关于 PokéBonk", "About PokéBonk"), None)
                .item(&check_updates)
                .separator()
                .hide_with_text(language.text("隐藏 PokéBonk", "Hide PokéBonk"))
                .hide_others_with_text(language.text("隐藏其他应用", "Hide Others"))
                .show_all_with_text(language.text("显示全部", "Show All"))
                .separator()
                .quit_with_text(language.text("退出 PokéBonk", "Quit PokéBonk"))
                .build()?;
            let game_menu = SubmenuBuilder::new(app, language.text("游戏", "Game"))
                .text(
                    "fullscreen",
                    language.text("切换全屏 (F11)", "Toggle Fullscreen (F11)"),
                )
                .text("reload", language.text("重新加载", "Reload"))
                .text("reconnect", language.text("重新连接", "Reconnect"))
                .separator()
                .text("website", language.text("在浏览器打开", "Open in Browser"))
                .build()?;
            let edit_menu = SubmenuBuilder::new(app, language.text("编辑", "Edit"))
                .undo_with_text(language.text("撤销", "Undo"))
                .redo_with_text(language.text("重做", "Redo"))
                .separator()
                .cut_with_text(language.text("剪切", "Cut"))
                .copy_with_text(language.text("复制", "Copy"))
                .paste_with_text(language.text("粘贴", "Paste"))
                .select_all_with_text(language.text("全选", "Select All"))
                .build()?;
            let window_menu = SubmenuBuilder::new(app, language.text("窗口", "Window"))
                .minimize_with_text(language.text("最小化", "Minimize"))
                .maximize_with_text(language.text("最大化", "Maximize"))
                .separator()
                .close_window_with_text(language.text("关闭窗口", "Close Window"))
                .build()?;
            app.set_menu(
                MenuBuilder::new(app)
                    .items(&[&app_menu, &game_menu, &edit_menu, &window_menu])
                    .build()?,
            )?;
            let navigation_app = app.handle().clone();
            let popup_app = app.handle().clone();
            let initialization = if cfg!(target_os = "macos") {
                format!(
                    "{}\n{}",
                    include_str!("../pointer-lock.js"),
                    include_str!("../bridge.js")
                )
            } else {
                include_str!("../bridge.js").to_owned()
            };
            // https://docs.rs/tauri/2.12.5/tauri/webview/struct.WebviewWindowBuilder.html
            let window =
                tauri::WebviewWindowBuilder::new(app, "main", WebviewUrl::External(GAME.parse()?))
                    .title("PokéBonk — Rust")
                    .inner_size(1280.0, 800.0)
                    .min_inner_size(960.0, 540.0)
                    .center()
                    .initialization_script(initialization)
                    .background_color(tauri::webview::Color(26, 22, 48, 255))
                    .disable_drag_drop_handler()
                    .background_throttling(
                        tauri::utils::config::BackgroundThrottlingPolicy::Disabled,
                    )
                    .on_navigation(move |url| {
                        #[cfg(target_os = "macos")]
                        if let Some(window) = navigation_app.get_webview_window("main") {
                            pointer_lock::release(&window);
                        }
                        if is_game(url) {
                            true
                        } else {
                            open_external(&navigation_app, url);
                            false
                        }
                    })
                    .on_new_window(move |url, _| {
                        open_external(&popup_app, &url);
                        NewWindowResponse::Deny
                    })
                    .build()?;
            #[cfg(target_os = "macos")]
            pointer_lock::install(&window)?;
            let focused_window = window.clone();
            window.on_window_event(move |event| {
                if let WindowEvent::Focused(_) = event {
                    if let Err(error) = audio::sync(&focused_window) {
                        eprintln!("Could not update background audio: {error}");
                    }
                }
            });
            audio::sync(&window)?;
            updater::start(app.handle());
            Ok(())
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            "fullscreen" => {
                if let Some(w) = app.get_webview_window("main") {
                    if let Ok(on) = w.is_fullscreen() {
                        let _ = w.set_fullscreen(!on);
                    }
                }
            }
            "reload" => request_action(app, Action::Reload),
            "reconnect" => request_action(app, Action::Reconnect),
            "website" => open_external(app, &GAME.parse().expect("static game URL")),
            "check-updates" => updater::check(app, true),
            _ => {}
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                }
                return;
            }
            match event {
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    request_action(window.app_handle(), Action::Quit);
                }
                WindowEvent::Resized(_) => {
                    if let Some(w) = window.app_handle().get_webview_window("main") {
                        let _ = w.eval("window.dispatchEvent(new Event('pb:window-change'))");
                    }
                }
                _ => {}
            }
        })
        .build(tauri::generate_context!())
        .expect("Could not start PokeBonk Rust");
    app.run(|handle, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            if !handle.state::<ExitState>().allowed.load(Ordering::SeqCst) {
                api.prevent_exit();
                request_action(handle, Action::Quit);
            }
        }
    });
}

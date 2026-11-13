use super::{
    locale::Language,
    update_progress::{percent, ProgressWindow, Stage},
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use std::time::{Duration, Instant};
use tauri::{menu::MenuItem, AppHandle, Manager, Wry};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};
use tauri_plugin_updater::UpdaterExt;
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

pub struct UpdaterState {
    menu: MenuItem<Wry>,
    gate: CheckGate,
}

impl UpdaterState {
    pub fn new(menu: MenuItem<Wry>) -> Self {
        Self {
            menu,
            gate: CheckGate::default(),
        }
    }
}

#[derive(Default)]
struct CheckGate {
    busy: AtomicBool,
    dismissed: Mutex<Option<String>>,
}

impl CheckGate {
    fn begin(&self) -> bool {
        !self.busy.swap(true, Ordering::SeqCst)
    }

    fn finish(&self) {
        self.busy.store(false, Ordering::SeqCst);
    }

    fn should_offer(&self, version: &str, manual: bool) -> bool {
        manual
            || self
                .dismissed
                .lock()
                .expect("update prompt state")
                .as_deref()
                != Some(version)
    }

    fn dismiss(&self, version: String) {
        *self.dismissed.lock().expect("update prompt state") = Some(version);
    }
}

fn status(app: &AppHandle, text: &str, enabled: bool) {
    let state = app.state::<UpdaterState>();
    let _ = state.menu.set_text(text);
    let _ = state.menu.set_enabled(enabled);
}

fn message(app: &AppHandle, text: &str) {
    let mut dialog = app.dialog().message(text).title("PokéBonk");
    if let Some(window) = app.get_webview_window("main") {
        dialog = dialog.parent(&window);
    }
    dialog.show(|_| {});
}

async fn confirm(app: &AppHandle, version: &str) -> Option<bool> {
    let language = *app.state::<Language>();
    if app
        .state::<super::ExitState>()
        .confirming
        .swap(true, Ordering::SeqCst)
    {
        return None;
    }
    let mut dialog = app
        .dialog()
        .message(format!(
            "{} v{version}\n{}",
            language.text("发现新版本", "New version available:"),
            language.text(
                "请先保存当前对局，升级后将重新启动。",
                "Save your current run first. Updating will restart the app."
            )
        ))
        .title("PokéBonk")
        .buttons(MessageDialogButtons::OkCancelCustom(
            language.text("升级并重启", "Update and Restart").into(),
            language.text("稍后", "Later").into(),
        ));
    if let Some(window) = app.get_webview_window("main") {
        dialog = dialog.parent(&window);
    }
    // Native dialogs must not block the event loop or the async executor.
    let result = tauri::async_runtime::spawn_blocking(move || dialog.blocking_show()).await;
    app.state::<super::ExitState>()
        .confirming
        .store(false, Ordering::SeqCst);
    result.ok()
}

// https://v2.tauri.app/plugin/updater/#checking-for-updates
async fn check_and_update(app: &AppHandle, manual: bool) {
    let language = *app.state::<Language>();
    let checked = async {
        app.updater_builder()
            .timeout(Duration::from_secs(20))
            .build()?
            .check()
            .await
    }
    .await;
    let mut update = match checked {
        Ok(Some(update)) => update,
        Ok(None) => {
            if manual {
                message(
                    app,
                    &format!(
                        "{} ({}).",
                        language.text("已经是最新版本", "You're up to date"),
                        app.package_info().version
                    ),
                );
            }
            return;
        }
        Err(error) => {
            eprintln!("Could not check for updates: {error}");
            if manual {
                message(
                    app,
                    language.text(
                        "暂时无法检查更新，请稍后重试。",
                        "Could not check for updates. Please try again later.",
                    ),
                );
            }
            return;
        }
    };
    if !app
        .state::<UpdaterState>()
        .gate
        .should_offer(&update.version, manual)
    {
        return;
    }
    match confirm(app, &update.version).await {
        Some(true) => {}
        Some(false) => {
            app.state::<UpdaterState>().gate.dismiss(update.version);
            return;
        }
        None => return,
    }
    let progress = match ProgressWindow::open(app, language, &update.version) {
        Ok(progress) => progress,
        Err(error) => {
            eprintln!("Could not open update progress: {error}");
            message(
                app,
                language.text(
                    "无法打开更新窗口，请稍后重试。",
                    "Could not open the update window. Please try again.",
                ),
            );
            return;
        }
    };
    status(
        app,
        language.text("正在下载更新…", "Downloading update…"),
        false,
    );
    update.timeout = Some(Duration::from_secs(600));
    let mut downloaded = 0;
    let mut last_percent = None;
    let mut last_render = Instant::now();
    let bytes = match update
        .download(
            |length, total| {
                downloaded += length as u64;
                let next = percent(downloaded, total);
                if next != last_percent || last_render.elapsed() >= Duration::from_millis(100) {
                    progress.set(Stage::Download, downloaded, total);
                    if let Some(next) = next {
                        status(
                            app,
                            &format!(
                                "{} {next}%",
                                language.text("正在下载更新…", "Downloading update…")
                            ),
                            false,
                        );
                    }
                    last_percent = next;
                    last_render = Instant::now();
                }
            },
            || progress.set(Stage::Verify, 0, None),
        )
        .await
    {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("Could not download or verify update: {error}");
            drop(progress);
            message(
                app,
                language.text(
                    "更新下载或校验失败，请稍后重试。",
                    "Download or verification failed. Please try again.",
                ),
            );
            return;
        }
    };
    progress.set(Stage::Install, 0, None);
    status(
        app,
        language.text("正在安装更新…", "Installing update…"),
        false,
    );
    // Windows exits directly after launching NSIS, before normal shutdown saves state.
    if let Err(error) = app.save_window_state(StateFlags::all()) {
        eprintln!("Could not save window state before update: {error}");
    }
    app.state::<super::ExitState>()
        .allowed
        .store(true, Ordering::SeqCst);
    let installed = tauri::async_runtime::spawn_blocking(move || update.install(bytes)).await;
    match installed {
        Ok(Ok(())) => {
            progress.set(Stage::Restart, 0, None);
            app.restart()
        }
        result => {
            app.state::<super::ExitState>()
                .allowed
                .store(false, Ordering::SeqCst);
            eprintln!("Could not install update: {result:?}");
            drop(progress);
            message(
                app,
                language.text(
                    "未能完成安装，请稍后重试或重新下载安装包。",
                    "Could not finish installing. Please try again or download the installer.",
                ),
            );
        }
    }
}

pub fn check(app: &AppHandle, manual: bool) {
    if !app.state::<UpdaterState>().gate.begin() {
        return;
    }
    status(
        app,
        app.state::<Language>()
            .text("正在检查更新…", "Checking for updates…"),
        false,
    );
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        check_and_update(&app, manual).await;
        status(
            &app,
            app.state::<Language>()
                .text("检查更新…", "Check for Updates…"),
            true,
        );
        app.state::<UpdaterState>().gate.finish();
    });
}

pub fn start(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(10)).await;
        loop {
            check(&app, false);
            tokio::time::sleep(Duration::from_secs(4 * 60 * 60)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_check_can_run_until_it_finishes() {
        let gate = CheckGate::default();
        assert!(gate.begin());
        assert!(!gate.begin());
        gate.finish();
        assert!(gate.begin());
    }

    #[test]
    fn later_skips_only_automatic_prompts_for_the_same_version() {
        let gate = CheckGate::default();
        assert!(gate.should_offer("0.1.4", false));
        gate.dismiss("0.1.4".into());
        assert!(!gate.should_offer("0.1.4", false));
        assert!(gate.should_offer("0.1.4", true));
        assert!(gate.should_offer("0.1.5", false));
    }

    #[test]
    fn progress_handles_unknown_lengths_and_overruns() {
        assert_eq!(percent(20, None), None);
        assert_eq!(percent(20, Some(0)), None);
        assert_eq!(percent(20, Some(80)), Some(25));
        assert_eq!(percent(81, Some(80)), Some(100));
        assert_eq!(percent(u64::MAX, Some(u64::MAX)), Some(100));
    }
}

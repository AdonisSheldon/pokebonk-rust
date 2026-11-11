use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use std::time::Duration;
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

fn percent(downloaded: u64, total: Option<u64>) -> Option<u8> {
    let total = total.filter(|total| *total > 0)?;
    Some((u128::from(downloaded) * 100 / u128::from(total)).min(100) as u8)
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
            "发现新版本 v{version}\n请先保存当前对局，升级后将重新启动。"
        ))
        .title("PokéBonk")
        .buttons(MessageDialogButtons::OkCancelCustom(
            "升级并重启".into(),
            "稍后".into(),
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
                    &format!("已经是最新版本（{}）。", app.package_info().version),
                );
            }
            return;
        }
        Err(error) => {
            eprintln!("Could not check for updates: {error}");
            if manual {
                message(app, "暂时无法检查更新，请稍后重试。");
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
    status(app, "正在下载更新…", false);
    update.timeout = Some(Duration::from_secs(600));
    let mut downloaded = 0;
    let mut last_percent = None;
    let bytes = match update
        .download(
            |length, total| {
                downloaded += length as u64;
                let next = percent(downloaded, total);
                if next != last_percent {
                    if let Some(next) = next {
                        status(app, &format!("正在下载更新… {next}%"), false);
                    }
                    last_percent = next;
                }
            },
            || {},
        )
        .await
    {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("Could not download or verify update: {error}");
            message(app, "更新下载或校验失败，请稍后重试。");
            return;
        }
    };
    status(app, "正在安装更新…", false);
    // Windows exits directly after launching NSIS, before normal shutdown saves state.
    if let Err(error) = app.save_window_state(StateFlags::all()) {
        eprintln!("Could not save window state before update: {error}");
    }
    app.state::<super::ExitState>()
        .allowed
        .store(true, Ordering::SeqCst);
    let installed = tauri::async_runtime::spawn_blocking(move || update.install(bytes)).await;
    match installed {
        Ok(Ok(())) => app.restart(),
        result => {
            app.state::<super::ExitState>()
                .allowed
                .store(false, Ordering::SeqCst);
            eprintln!("Could not install update: {result:?}");
            message(app, "未能完成安装，请稍后重试或重新下载安装包。");
        }
    }
}

pub fn check(app: &AppHandle, manual: bool) {
    if !app.state::<UpdaterState>().gate.begin() {
        return;
    }
    status(app, "正在检查更新…", false);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        check_and_update(&app, manual).await;
        status(&app, "检查更新 / Check for Updates…", true);
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

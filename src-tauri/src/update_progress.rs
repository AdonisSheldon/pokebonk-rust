use super::locale::Language;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tauri::{webview::PageLoadEvent, AppHandle, Manager, WebviewUrl, WebviewWindow};

pub const LABEL: &str = "updater-progress";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Download,
    Verify,
    Install,
    Restart,
}

pub fn percent(downloaded: u64, total: Option<u64>) -> Option<u8> {
    let total = total.filter(|total| *total > 0)?;
    Some((u128::from(downloaded) * 100 / u128::from(total)).min(100) as u8)
}

fn view(
    language: Language,
    version: &str,
    stage: Stage,
    downloaded: u64,
    total: Option<u64>,
) -> Value {
    let text = match stage {
        Stage::Download => language.text("正在下载更新…", "Downloading update…"),
        Stage::Verify => language.text("正在校验更新…", "Verifying update…"),
        Stage::Install => language.text("正在安装更新…", "Installing update…"),
        Stage::Restart => language.text("正在重新启动…", "Restarting…"),
    };
    json!({
        "language": language.text("zh-CN", "en"), "version": version, "stage": text,
        "percentage": match stage { Stage::Download => percent(downloaded, total), Stage::Restart => Some(100), _ => None },
        "downloaded": if stage == Stage::Download { Some(downloaded) } else { None },
        "total": if stage == Stage::Download { total.filter(|value| *value > 0) } else { None },
    })
}

struct ProgressState(Mutex<Value>);

impl ProgressState {
    fn new(initial: Value) -> Self {
        Self(Mutex::new(initial))
    }

    fn publish(&self, value: Value, render: impl FnOnce(&Value)) {
        // shortcut: eval must stay nonblocking; review this lock before enabling tauri/tracing.
        let mut latest = self.0.lock().expect("update progress state");
        *latest = value;
        render(&latest);
    }

    fn replay(&self, render: impl FnOnce(&Value)) {
        let latest = self.0.lock().expect("update progress state");
        render(&latest);
    }
}

fn render(window: &WebviewWindow, data: &Value) {
    let script = format!("window.__pokebonkUpdateProgress = {data}; window.__pokebonkRenderUpdate?.(window.__pokebonkUpdateProgress);");
    if let Err(error) = window.eval(script) {
        eprintln!("Could not render update progress: {error}");
    }
}

pub struct ProgressWindow {
    window: WebviewWindow,
    language: Language,
    version: String,
    latest: Arc<ProgressState>,
}

impl ProgressWindow {
    pub fn open(app: &AppHandle, language: Language, version: &str) -> tauri::Result<Self> {
        let initial = view(language, version, Stage::Download, 0, None);
        let latest = Arc::new(ProgressState::new(initial.clone()));
        let loaded_state = latest.clone();
        let mut builder = tauri::WebviewWindowBuilder::new(
            app,
            LABEL,
            WebviewUrl::App("update-progress.html".into()),
        )
        .title(language.text("PokéBonk · 更新", "PokéBonk · Update"))
        .inner_size(420.0, 230.0)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .closable(false)
        .skip_taskbar(true)
        .center()
        .focused(true)
        .theme(Some(tauri::Theme::Light))
        .background_color(tauri::webview::Color(245, 241, 232, 255))
        .initialization_script(format!("window.__pokebonkUpdateProgress = {initial};"))
        .on_page_load(move |window, payload| {
            // Windows may initialize the document after an early ExecuteScript call.
            if matches!(payload.event(), PageLoadEvent::Finished) {
                loaded_state.replay(|data| render(&window, data));
            }
        });
        if let Some(main) = app.get_webview_window("main") {
            builder = builder.parent(&main)?;
        }
        Ok(Self {
            window: builder.build()?,
            language,
            version: version.to_owned(),
            latest,
        })
    }

    pub fn set(&self, stage: Stage, downloaded: u64, total: Option<u64>) {
        let data = view(self.language, &self.version, stage, downloaded, total);
        self.latest.publish(data, |data| render(&self.window, data));
    }
}

impl Drop for ProgressWindow {
    fn drop(&mut self) {
        let _ = self.window.destroy();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downloading_reports_real_bytes_and_percent() {
        let data = view(Language::English, "0.1.6", Stage::Download, 20, Some(80));
        assert_eq!(data["percentage"], 25);
        assert_eq!(data["downloaded"], 20);
        assert_eq!(data["stage"], "Downloading update…");
        assert_eq!(
            view(Language::Chinese, "0.1.6", Stage::Download, 0, None)["percentage"],
            Value::Null
        );
    }

    #[test]
    fn verification_and_installation_never_claim_a_fake_percentage() {
        for stage in [Stage::Verify, Stage::Install] {
            let data = view(Language::Chinese, "0.1.6", stage, 80, Some(80));
            assert_eq!(data["percentage"], Value::Null);
            assert_eq!(data["downloaded"], Value::Null);
        }
        assert_eq!(
            view(Language::English, "0.1.6", Stage::Restart, 0, None)["percentage"],
            100
        );
    }

    #[test]
    fn page_load_replays_latest_stage_after_initial_download_state() {
        for stage in [Stage::Verify, Stage::Install] {
            let initial = view(Language::English, "0.1.6", Stage::Download, 0, None);
            let latest = view(Language::English, "0.1.6", stage, 80, Some(80));
            let progress = ProgressState::new(initial.clone());
            let mut page = initial.clone();

            progress.publish(latest.clone(), |value| page = value.clone());
            assert_eq!(page, latest);

            // Simulate page initialization finishing after an early Windows script.
            page = initial;
            progress.replay(|value| page = value.clone());

            assert_eq!(page, latest);
        }
    }
}

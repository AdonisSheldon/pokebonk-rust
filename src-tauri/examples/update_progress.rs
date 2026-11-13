//! Read-only UI fixture: verifies a real download, then previews install/restart; never installs.
#[path = "../src/locale.rs"]
mod locale;
#[path = "../src/update_progress.rs"]
mod update_progress;

use locale::Language;
use std::time::Duration;
use tauri_plugin_updater::UpdaterExt;
use update_progress::{ProgressWindow, Stage};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut context = tauri::generate_context!();
    context.package_info_mut().version = "0.1.4".parse()?;
    context.config_mut().identifier = "local.pokebonk.progress-preview".into();
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let language = match std::env::var("POKEBONK_PREVIEW_LANGUAGE").as_deref() {
                    Ok("en") => Language::English,
                    Ok("zh") => Language::Chinese,
                    _ => Language::system(),
                };
                let progress = ProgressWindow::open(&handle, language, "0.1.5");
                let result: Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> = async {
                    let progress = progress.as_ref().map_err(|error| error.to_string())?;
                    let endpoint = "https://github.com/AdonisSheldon/pokebonk-rust/releases/download/v0.1.5/latest.json".parse()?;
                    let mut update = handle.updater_builder().timeout(Duration::from_secs(30)).endpoints(vec![endpoint])?.build()?.check().await?.ok_or("missing fixture release")?;
                    update.timeout = Some(Duration::from_secs(600));
                    let mut received = 0;
                    let bytes = update.download(|length, total| {
                        received += length as u64;
                        progress.set(Stage::Download, received, total);
                        // Keep cached downloads visible long enough to inspect this test window.
                        std::thread::sleep(Duration::from_millis(40));
                    }, || progress.set(Stage::Verify, 0, None)).await?;
                    progress.set(Stage::Install, 0, None);
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    progress.set(Stage::Restart, 0, None);
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    Ok(serde_json::json!({"outcome":"verified", "version":update.version, "bytes":bytes.len(), "language":language.text("zh-CN", "en"), "installed":false}))
                }.await;
                let report = match result {
                    Ok(data) => data,
                    Err(error) => serde_json::json!({"outcome":"error", "error":error.to_string()}),
                };
                let path = std::env::var_os("POKEBONK_PREVIEW_REPORT").map(std::path::PathBuf::from)
                    .unwrap_or_else(|| std::env::temp_dir().join("pokebonk-progress-preview.json"));
                std::fs::write(path, serde_json::to_vec_pretty(&report).expect("preview JSON")).expect("preview report");
                drop(progress);
                handle.exit(if report["outcome"] == "verified" { 0 } else { 1 });
            });
            Ok(())
        })
        .run(context)?;
    Ok(())
}

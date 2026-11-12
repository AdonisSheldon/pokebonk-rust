//! Read-only updater integration probe; it never calls install or opens the game.
use std::{env, fs, path::PathBuf, time::Duration};
use tauri_plugin_updater::UpdaterExt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let endpoint: tauri::Url = args.next().ok_or("missing endpoint")?.parse()?;
    let report = PathBuf::from(args.next().ok_or("missing report path")?);
    let target = args.next();
    let version = env::var("POKEBONK_PROBE_VERSION").unwrap_or_else(|_| "0.1.2".into());
    let loopback = endpoint.scheme() == "http" && endpoint.host_str() == Some("127.0.0.1");
    if !loopback && endpoint.scheme() != "https" {
        return Err("probe endpoints must use HTTPS or loopback HTTP".into());
    }
    let mut context = tauri::generate_context!();
    context.package_info_mut().version = version.parse()?;
    context.config_mut().identifier = "local.pokebonk.updater-probe".into();
    let settings = context
        .config_mut()
        .plugins
        .0
        .get_mut("updater")
        .ok_or("missing updater config")?;
    settings["endpoints"] = serde_json::json!([endpoint]);
    // Restricted to this no-install test executable; production config enforces TLS.
    settings["dangerousInsecureTransportProtocol"] = serde_json::json!(loopback);
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            let handle = app.handle().clone();
            let report = report.clone();
            let target = target.clone();
            tauri::async_runtime::spawn(async move {
                let result: Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> =
                    async {
                        let mut builder = handle.updater_builder().timeout(Duration::from_secs(30));
                        if loopback {
                            builder = builder.no_proxy();
                        }
                        if let Some(target) = target {
                            builder = builder.target(target);
                        }
                        if let Some(update) = builder.build()?.check().await? {
                            let bytes = update.download(|_, _| {}, || {}).await?;
                            fs::write(report.with_extension("download"), &bytes)?;
                            Ok(
                                serde_json::json!({"outcome":"verified", "version":update.version,
                            "current_version":update.current_version, "target":update.target,
                            "url":update.download_url, "bytes":bytes.len()}),
                            )
                        } else {
                            Ok(serde_json::json!({"outcome":"current"}))
                        }
                    }
                    .await;
                let (data, code) = match result {
                    Ok(data) => (data, 0),
                    Err(error) => (
                        serde_json::json!({"outcome":"error", "error":error.to_string()}),
                        1,
                    ),
                };
                fs::write(
                    &report,
                    serde_json::to_vec_pretty(&data).expect("probe JSON"),
                )
                .expect("probe report");
                handle.exit(code);
            });
            Ok(())
        })
        .build(context)?;
    app.run(|_, _| {});
    Ok(())
}

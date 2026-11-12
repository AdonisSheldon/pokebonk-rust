use tauri::Manager;
#[cfg(target_os = "macos")]
#[path = "../src/pointer_lock.rs"]
mod native;

#[tauri::command]
async fn pointer_lock(window: tauri::WebviewWindow, locked: bool) -> Result<(), String> {
    let url = window.url().map_err(|error| error.to_string())?;
    if url.host_str() != Some("127.0.0.1") || url.port() != Some(18766) {
        return Err("Only the loopback fixture may use this test command".into());
    }
    #[cfg(target_os = "macos")]
    return native::set(&window, locked).await;
    #[cfg(not(target_os = "macos"))]
    {
        let _ = locked;
        Err("Windows uses its own Pointer Lock implementation".into())
    }
}

fn main() {
    let mut context = tauri::generate_context!();
    context.config_mut().identifier = "local.pokebonk.pointer-lock-test".into();
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![pointer_lock])
        .setup(|app| {
            app.add_capability(serde_json::json!({
                "identifier":"pointer-lock-test-runtime", "windows":["main"], "local":false,
                "remote":{"urls":["http://127.0.0.1:18766"]}, "permissions":["allow-pointer-lock"]
            }).to_string())?;
            let builder = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::External("http://127.0.0.1:18766/fixture.html".parse()?),
            )
            .title("Pointer Lock Test")
            .inner_size(1080.0, 900.0);
            #[cfg(target_os = "macos")]
            let builder = builder
                .initialization_script(include_str!("../pointer-lock.js"))
                .initialization_script("window.__pokebonkInstallPointerLock();");
            let window = builder.build()?;
            #[cfg(target_os = "macos")]
            native::install(&window)?;
            let _ = window;
            Ok(())
        })
        .run(context)
        .expect("Pointer Lock fixture failed");
}

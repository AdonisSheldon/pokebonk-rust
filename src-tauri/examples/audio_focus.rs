// Native regression fixture: cargo run --example audio_focus. Switch to another
// app and back; audio must suspend and resume without changing the volume or
// resuming the intentionally paused context. No game or network data is used.
#[path = "../src/audio.rs"]
mod audio;

fn main() {
    tauri::Builder::default()
        .manage(audio::FocusAudio::default())
        .setup(|app| {
            let window = tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("Audio Focus Test — local fixture")
            .inner_size(780.0, 480.0)
            .initialization_script(include_str!("audio_focus.js"))
            // Allow only this local fixture's generated silent WAV sample.
            .on_web_resource_request(|_, response| {
                response.headers_mut().insert(
                    "Content-Security-Policy",
                    "default-src 'self'; style-src 'self' 'unsafe-inline'; media-src blob:"
                        .parse()
                        .unwrap(),
                );
            })
            .build()?;
            let focused_window = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::Focused(_) = event {
                    audio::sync(&focused_window).unwrap();
                }
            });
            audio::sync(&window)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Audio fixture could not start");
}

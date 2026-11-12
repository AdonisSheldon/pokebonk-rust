fn main() {
    // https://docs.rs/tauri-build/2.7.4/tauri_build/struct.AppManifest.html
    let manifest = tauri_build::AppManifest::new().commands(&[
        "display",
        "set_display",
        "native_action",
        "pointer_lock",
    ]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
        .expect("Could not build Tauri resources");
}

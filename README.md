# PokeBonk Rust for macOS

Independent Rust / Tauri 2 / WKWebView desktop client. Based on review of
`CharcherGit/pokebonk-app` main at `219162945987c3b8c95699c3f84bdc758926f1a3`
(2026-11-03), checked on 2026-11-10. The game is loaded from https://pokebonk.com;
the game itself is not rewritten or bundled. This is a personal alternative client,
not an official release. Upstream artwork and game rights remain with their owners.

## Development scope and acceptance

- Apple Silicon macOS 13+ app and DMG; ASCII package/executable name.
- Persistent, independent game data; no import or alteration of Electron data.
- Native menu, fullscreen, reload/reconnect, window size/monitor settings.
- Compatible `pokebonkDesktop` bridge for these features, restricted to the official
  HTTPS game origin. External links open in the default browser.
- System-managed WebKit graphics; Chromium ANGLE/V-Sync overrides are not offered.
- Window state and single instance. Confirm before quit/reload to protect a run.
- No Discord integration or automatic installation of upstream Electron updates.
- Verify Rust policy tests, JS bridge tests, lint, release build, native runtime,
  WebGL2 rendering, package signatures and DMG contents. No FPS gain is assumed.

## Build

Install Rust stable and Xcode command line tools, then Node 22+:

```sh
npm ci
npm run test:bridge
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --locked -- -D warnings
npm run build
npm run dev
```

Rust code is in `src-tauri/src`, the injected compatibility bridge is
`src-tauri/bridge.js`, and bridge tests are in `tests`. `Cargo.lock` and
`package-lock.json` pin dependencies. Use `cargo fmt` and small direct functions.

Boundary: only official game pages may call the enumerated window commands. No
generic file, process, shell or network API is exposed to web content. Never copy
session cookies or authentication data between clients. Publishing to a third-party
service and Apple Developer signing require separately supplied credentials.

## Framework references

- https://docs.rs/tauri/2.12.5/tauri/webview/struct.WebviewWindowBuilder.html
- https://v2.tauri.app/security/capabilities/#remote-api-access
- https://docs.rs/tauri-build/2.7.4/tauri_build/struct.AppManifest.html
- https://v2.tauri.app/plugin/window-state/
- https://v2.tauri.app/plugin/single-instance/
- https://v2.tauri.app/distribute/macos-application-bundle/

## Implementation checkpoints

1. Compile the Rust WKWebView shell and lock dependencies.
2. Test URL/input boundaries; add the desktop bridge, native menus and persistence.
3. Test and build; verify the loaded game and window controls in the native app.
4. Package an ad-hoc signed app, source archive and verified DMG.

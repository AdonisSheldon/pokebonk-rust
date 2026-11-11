# PokeBonk Rust for macOS and Windows

Independent Rust / Tauri 2 desktop client using WKWebView on macOS and WebView2 on
Windows. Based on review of
`CharcherGit/pokebonk-app` main at `219162945987c3b8c95699c3f84bdc758926f1a3`
(2026-11-03), checked on 2026-11-10. The game is loaded from https://pokebonk.com;
the game itself is not rewritten or bundled. This is a personal alternative client,
not an official release. Upstream artwork and game rights remain with their owners.

## Development scope and acceptance

- Intel and Apple Silicon macOS 13+ apps and separate DMGs; ASCII package/executable name.
- Windows 10/11 x64 app and NSIS `.exe` installer, with native mute while unfocused.
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

For both architectures on the same Mac:

```sh
rustup target add aarch64-apple-darwin x86_64-apple-darwin
npm run build:apple -- -- --locked
npm run build:intel -- -- --locked
```

`build:apple` targets `aarch64-apple-darwin` (M-series Macs); `build:intel` targets
`x86_64-apple-darwin` (Intel Macs). Each target has its own signed app in
`src-tauri/target/<target>/release/bundle/macos/PokeBonk Rust.app`. Both retain
the same app version and features. Rust cross-compilation uses Xcode's macOS SDK:
https://doc.rust-lang.org/rustc/platform-support/apple-darwin.html

Rust code is in `src-tauri/src`, the injected compatibility bridge is
`src-tauri/bridge.js`, and bridge tests are in `tests`. `Cargo.lock` and
`package-lock.json` pin dependencies. Use `cargo fmt` and small direct functions.

### Drag-to-install DMG

With Python 3.11+, install the packaging tool and package the signed app:

```sh
python3 -m pip install dmgbuild==1.6.7
python3 packaging/build-dmg.py \
  "src-tauri/target/aarch64-apple-darwin/release/bundle/macos/PokeBonk Rust.app" \
  "PokeBonk-Rust-0.1.2-macOS-arm64-Installer.dmg"
python3 packaging/build-dmg.py \
  "src-tauri/target/x86_64-apple-darwin/release/bundle/macos/PokeBonk Rust.app" \
  "PokeBonk-Rust-0.1.2-macOS-x86_64-Installer.dmg"
```

The image opens in a minimal 640 × 360 icon view with a warm Retina background,
pixel frames, a red arrow and a small Poké Ball; there is no background text.
App and Applications positions are fixed. The script verifies
the binary architecture, mounted layout, app bytes, Applications link and code signature before publishing
the image. It refuses to overwrite an existing image. It does not rebuild or re-sign
the app. To regenerate the checked-in background with macOS AppKit:

```sh
swift packaging/dmg-background.swift packaging/dmg-background
```

Packaging reference: https://dmgbuild.readthedocs.io/en/latest/settings.html

### Windows installer

On Windows, install Rust MSVC, Microsoft C++ Build Tools, Node and WebView2, then:

```sh
npm ci
npm run test:bridge
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run build:windows -- -- --locked
```

The installer is written to `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/`.
It installs for the current user, uses Simplified Chinese or English according to
the OS language, and downloads the Evergreen WebView2 bootstrapper if the runtime
is missing or older than the configured minimum. Game access requires the internet.

The installer uses the macOS package's warm cream, dark pixel frame and red Poké
Ball palette. The welcome page and explanatory paragraphs are removed; installation
location, progress, launch and shortcut choices remain. The small
`packaging/windows-theme.nsh` hook retains Tauri's standard update, uninstall and
WebView2 handling. The checked-in 24-bit BMP artwork has no baked-in text. To
regenerate it on macOS (not required for a Windows build):

```sh
swift packaging/windows-art.swift packaging
```

For cross-compilation from macOS, install LLVM, LLD, NSIS, the Rust
`x86_64-pc-windows-msvc` target and cargo-xwin, add LLVM and cargo-xwin to PATH, then:

```sh
npm run build:windows -- --runner cargo-xwin -- --locked
```

Windows focus muting uses WebView2 `ICoreWebView2_8::SetIsMuted` for all audio output.
It preserves the initial mute value across repeated blur events and restores it on
focus. Failed setters retain the previous state for retry. Page volume values and
intentional playback pauses are untouched. macOS retains its WKWebView handler.
The same local `audio_focus` fixture is available on Windows with
`cargo run --manifest-path src-tauri/Cargo.toml --example audio_focus`.
On Windows, playback remains running while native output is muted; check audible
silence and restoration when switching focus rather than expecting a paused state.

- https://v2.tauri.app/distribute/windows-installer/
- https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/icorewebview2_8?view=webview2-1.0.3537.50

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


## Release verification (2026-11-10)

- Upstream main: `219162945987c3b8c95699c3f84bdc758926f1a3`.
- Rust 1.99.1; Tauri 2.12.5; Tauri CLI 2.12.4; target `aarch64-apple-darwin`.
- 4 Rust tests and 4 JS bridge tests passed; Clippy passed with warnings denied.
- npm audit: no vulnerabilities. RustSec audit: no vulnerabilities; five advisory
  warnings in cross-platform lockfile dependencies were checked against the
  Apple Silicon build graph and none of those packages is included on macOS.
- Native runtime: official online game v0.9.23 loaded, guest entry reached starter
  selection, and F11 fullscreen toggling worked. The running test window was left
  available for use. No FPS improvement or long-session stability is claimed.
- Monitor selection on a second physical screen, account login with a real account,
  co-op, gamepad and long-session gameplay have not been manually verified.
- App and DMG verification results are recorded with the delivered build metadata.
- Ad-hoc local signature, without Apple Developer ID notarization. On a Mac that
  blocks first launch, use System Settings → Privacy & Security → Open Anyway.
- The app is a separate client: existing Electron guest sessions are not imported.
  Registered users can sign into the same game service themselves.

The source archive contains this entire standalone `rust-macos` project, including
both dependency lockfiles and icons; it does not require the Electron project to build.


## 0.1.1 — mute while unfocused

The native macOS focus handler suspends all WKWebView media playback while the
application or game window is unfocused, and removes that suspension on focus.
Both HTML media and Web Audio are covered. Game volume values and intentional
pauses are preserved. Playback remains gated during page navigation and new audio
creation because WebKit owns the media suspension, rather than a page script.

Native regression fixture (local audio only; does not touch game data):

```sh
cargo run --manifest-path src-tauri/Cargo.toml --example audio_focus
```

Click Start, minimize/switch away, restore the window, then repeat after clicking
Pause intentionally. Web Audio should go from running to interrupted and back;
the held context stays suspended. HTML media pauses and resumes, with volume 0.37
and Web Audio gain 0.0001 unchanged. This was verified on macOS 27.0 with the native
fixture. The original four Rust tests, four JS tests and all-target Clippy also pass.

Framework API: https://developer.apple.com/documentation/webkit/wkwebview/setallmediaplaybacksuspended(_:completionhandler:)

## Dual-architecture verification (2026-11-11)

- Locked release builds passed for `aarch64-apple-darwin` and `x86_64-apple-darwin`.
- Four Rust tests passed on each target, and all four JavaScript bridge tests passed.
- The Apple Silicon app started natively; the Intel app started through Rosetta on
  Apple Silicon. Both loaded and rendered the official game's main menu.
- Each DMG's app matches its current build byte for byte. Architecture, macOS 13.0
  deployment target, Applications link, approved background and strict signature
  checks passed. The existing Apple Silicon DMG is unchanged.
- Runtime source and focus muting are unchanged. Intel hardware, macOS 13 hardware
  and long-session gameplay were not tested in this release. Both apps remain
  ad-hoc signed without Apple notarization.

## Windows 0.1.2 verification (2026-11-11)

- Windows x64 release and Unicode NSIS installer cross-compiled on macOS.
- Eight Rust tests passed natively on macOS and as Windows executables in an
  isolated CrossOver/Wine Windows 10 container. Four tests cover focus mute
  restoration, existing mute state, duplicate events and failed native setters.
- All four JavaScript bridge tests and all-target Clippy checks on both macOS and
  Windows passed with warnings denied. Cross-linking reports missing optional
  Microsoft CRT debug symbols; release linking succeeds.
- Installer archive integrity passed. Its x64 payload matches the compiled app
  after Tauri's expected NSIS bundle marker patch. A silent installation in the
  isolated container exited successfully and installed identical program bytes.
- WebView2 Evergreen 157.0.4341.46 was installed by the normal installer flow.
- Windows hardware gameplay, real output silence during focus changes and
  Windows 11 have not been manually verified. The installer and executable are
  unsigned; Windows may display a SmartScreen warning.

### Minimal installer theme (2026-11-11)

- NSIS theme compilation and archive integrity passed. Both embedded 24-bit BMPs
  match the checked-in artwork. The packaged application is byte-identical to the
  previous Windows installer, including its focus muting implementation.
- Fresh silent installation and same-version reinstallation in an isolated
  CrossOver/Wine Windows 10 container succeeded, installed WebView2 and produced
  the expected application bytes.
- CrossOver did not expose the interactive installer window for visual inspection.
  Interactive layout and display scaling still require verification on Windows.

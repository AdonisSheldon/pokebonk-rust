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
  "PokeBonk-Rust-0.1.6-macOS-arm64-Installer.dmg"
python3 packaging/build-dmg.py \
  "src-tauri/target/x86_64-apple-darwin/release/bundle/macos/PokeBonk Rust.app" \
  "PokeBonk-Rust-0.1.6-macOS-x86_64-Installer.dmg"
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
WebView2 handling. The directory page and footer use the same warm background;
its directory group, input and browse button are raised by 50 dialog units, with
Windows font and DPI conversion. The checked-in 24-bit BMP artwork has no
baked-in text. To regenerate it on macOS (not required for a Windows build):

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

## Automatic Rust client updates

Version 0.1.3 checks the dedicated
https://github.com/AdonisSheldon/pokebonk-rust/releases/latest/download/latest.json
ten seconds after startup and every four hours. The application menu also has
Check for Updates. A short native prompt offers upgrade/restart or Later and
reminds the player to save the current run. Later suppresses the same automatic
prompt for this session; a manual check can still offer it.

The Rust updater downloads and verifies the package before changing any program
files. Both its signature and the version bound into that signature must match.
Only after user confirmation does installation run; Windows uses the existing
NSIS installer in passive mode. The identifier, profile locations, installer
artwork and native focus-audio handlers are retained. No updater capability is
granted to remote game pages; the compatibility Electron update API stays off.

The public update key is embedded in the configuration. The private key must stay
outside the repository, be backed up by the maintainer and be stored as the
`TAURI_SIGNING_PRIVATE_KEY` release secret. Updater signatures are separate from
Apple notarization and Windows code signing. Existing 0.1.2 clients need one
manual installation of 0.1.3 to acquire this update mechanism.

API: https://v2.tauri.app/plugin/updater/

### Publishing a Rust release

Use the standalone repository https://github.com/AdonisSheldon/pokebonk-rust.
The workflow checks all three targets on pushes and pull requests. A version tag
also builds signed updater artifacts and the existing themed installers, then
automatically publishes a Latest release after all three targets and the updater
verification succeed. Assets are uploaded to a draft first; a failed build,
verification or upload prevents publication. Published assets are never overwritten.
The clients see the new `latest.json` on their next automatic or manual check.
The signing secret is supplied only to the release packaging step, not ordinary
pull-request checks. The release tag must equal `v` plus the application version.

Keep the version in `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`,
`src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json` and the bridge's `clientVersion`
consistent; the bridge tests enforce this. For local signed packaging, set
`TAURI_SIGNING_PRIVATE_KEY` to the private key file path and use the normal build
commands. Collect each target's unchanged signed bytes with:

```sh
python packaging/release.py collect aarch64-apple-darwin dist
python packaging/release.py collect x86_64-apple-darwin dist
python packaging/release.py collect x86_64-pc-windows-msvc dist
python packaging/release.py manifest dist AdonisSheldon/pokebonk-rust
python tests/release_test.py
```

The manifest embeds signature content, uses fixed architecture keys and refuses
to generate a partial three-platform feed. Updater packages are `.exe` on Windows
and `.app.tar.gz` on macOS; the themed DMGs remain for manual installation.

Before creating a draft, the release workflow runs a read-only Rust updater probe
against all three real signed packages. The local HTTP fixture also checks an
already-current client, a tampered payload, a signed-version mismatch and a
missing platform. It never installs or opens the game. To run it locally after
collecting the three signed packages:

```sh
cargo build --manifest-path src-tauri/Cargo.toml --locked --example updater_probe
python tests/updater_integration.py src-tauri/target/debug/examples/updater_probe dist updater-verification.json
```

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

The second installer revision also colors the directory page and footer warm
cream and raises its three directory controls by 50 dialog units. An isolated
NSIS fixture running the production callback passed in English and Simplified
Chinese: all controls moved upward together, their native sizes and horizontal
positions stayed unchanged, and the removed paragraph was hidden. The callback
also preserved caller registers; both the outer window and directory page
returned the expected warm cream background brushes. Windows hardware visual
checks remain pending.

## 0.1.4 — macOS game pointer lock

WKWebView does not expose the browser Pointer Lock host delegate used by the game.
The macOS bridge now captures relative mouse input only when the official game
calls `requestPointerLock()`; opening the app or showing a menu never captures it.
Native confirmation precedes `pointerlockchange`, and movement carries the current
mouse buttons and Shift/Control/Alt/Command state. Cocoa supplies accelerated input,
so raw/unadjusted requests explicitly reject with `NotSupportedError`, allowing the
game to fall back to the normal request. Windows retains WebView2's native API.

Esc, loss of focus, navigation and removal of the locked element release capture.
Returning to the foreground does not capture again until the game requests it.
Only the active official game window can invoke the new native command. Input is
monitored within this application; no global keyboard/mouse monitor or Accessibility
permission is used. Audio muting and installer artwork are unchanged.

Local regression fixture (separate identifier; no game account or save data):

```sh
python3 -m http.server 18766 --bind 127.0.0.1 --directory src-tauri/examples/pointer_lock
# In another terminal:
cargo run --manifest-path src-tauri/Cargo.toml --example pointer_lock
```

Verify idle/menu state stays unlocked; request lock, release the mouse buttons and
move; check nonzero relative movement with buttons=0. Then test X, Esc and switching
away, including a new request after returning. Esc may be consumed by the native
host before a DOM key event; verify that the locked element clears and change
count increases. JavaScript tests run with `npm run test:bridge`.

## 0.1.5 — automatic GitHub release publication

Version tags publish as Latest after all three signed builds and the genuine
updater verification succeed. This release uses that automated publication path
and the existing in-app updater. Game input, focus muting and installer artwork
retain the behavior of 0.1.4.

## 0.1.6 — visible update progress and system language

Native menus, confirmation dialogs and update messages use Chinese when the
system display language is Chinese, otherwise English. The language is selected
at startup; no application language setting is required.

After confirming an upgrade, a compact PokéBonk window displays downloaded bytes
and the actual percentage. Unknown download sizes, signature verification and
installation use an indeterminate bar. Errors close the progress window before
showing a native message. The local page has no granted IPC commands, and this
temporary window is excluded from saved window state.

The progress window is provided by the client initiating an update: clients up
to 0.1.5 still show their existing menu percentage while installing this version.
Updates initiated by 0.1.6 use the new window.

Read-only native preview (downloads and verifies the public 0.1.5 package, then
simulates the installation/restart display; never installs or loads the game):

```sh
cargo run --manifest-path src-tauri/Cargo.toml --locked --features custom-protocol --example update_progress
```

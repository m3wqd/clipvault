# Validation — 2026-11-24

This file records performed checks, not target-platform certification.

## Completed

- Installed local Rust 1.100.0, rustfmt, Clippy, Node.js 22.16.0 and npm in work/; no system package installation.
- Resolved Rust and npm dependencies; generated Cargo.lock and package-lock.json.
- Production frontend build: strict TypeScript and Vite 6.4.6 succeeded.
- Vitest: 4 tests passed (IPC browser rejection, history/loading/actions/events, debounced search/type filtering, backend error rendering).
- Rust core: 11 tests passed, covering persistent reopen, migration v1→latest/newer-version rejection, FTS search/deletion, deduplication, pins, retention, size/errors, favorites/tags/collections/FK cascade, snippets/settings/clear, image retention/pagination, PNG round-trip/thumbnail/path validation/reconciliation and settings/pattern checks.
- Linux desktop source checked with cargo check and Clippy --all-targets -- -D warnings using a locally extracted WebKitGTK package. This is not a GUI run.
- Windows x64 release EXE cross-compiled through cargo-xwin, Windows MSVC target and Microsoft SDK/CRT. Linker emitted missing Microsoft debug-PDB warnings; the release binary was generated. No successful runtime test is inferred from this.
- NSIS 3.13 compiled a real Windows setup executable containing the app and Microsoft's WebView2 bootstrapper.
- PE import inspection: Windows x64 GUI app; ordinary Windows system imports; no separate VCRUNTIME140 DLL import observed. NSIS itself uses a 32-bit installer stub containing the 64-bit app.
- JSON/TOML parsing and Python migration tests also run.
- wl-paste --list-types accessed the actual Wayland session successfully outside the sandbox; only format names were inspected, no clipboard text was printed. No clipboard write or restore test was performed.

## Not completed

- Windows install/uninstall/upgrade and application run, real copy/capture/restore, tray, hotkey, popup focus and autostart.
- X11 clipboard ownership and round trips.
- Wayland/Hyprland text/image round trips, compositor compatibility and background behavior. Format enumeration alone is not validation of these features.
- Native Rust desktop test binaries (core tests run without desktop dependency), Linux release bundles, MSI or Arch package builds.
- Public CI runs, release signatures, publication, updater, measured resource profiling, desktop screenshots/GIFs.

## Acceptance procedure

Use disposable non-sensitive text and images in a target desktop session. Copy A/B/A, verify one A item with updated count, restart, restore A and paste into another application. Repeat with Unicode and an image. Search, pin/favorite, tag and assign collection, remove and restart. Test snippets and popup keyboard navigation. Pause/incognito must prevent recording. Test unavailable clipboard owner, oversized data, disk-full handling, retention with pins and explicit clear confirmation. Verify autostart off by default, close-to-tray and Quit, installer update/data preservation, normal uninstall and deliberate full data removal. Record OS/compositor versions and actual results before declaring production readiness.

## Startup repair — 0.2.1

The 0.2.0 Windows artifact was incorrectly built without Tauri's custom-protocol feature, so it used devUrl (localhost:1420) rather than embedded UI assets. This was an actual packaging defect, not a user configuration issue.

For 0.2.1, the cross-build enables custom-protocol and release builds without it are rejected by build.rs. Verified by invoking the compiled build script with PROFILE=release and no feature flag: it rejected the build with the intended diagnostic.

Performed for this patch:

- Strict TypeScript/Vite production build passed; four frontend tests passed.
- Eleven Rust core tests and two Python SQLite tests passed.
- New production-context regression test ran successfully on Linux without starting a dev server: tauri::is_dev() was false, and index.html and its referenced JavaScript were resolved from embedded assets.
- Clippy with custom-protocol and all targets passed with warnings denied.
- Windows x64 EXE rebuilt with custom-protocol; static PE verification confirmed that the current Vite JavaScript and CSS asset keys are present in that exact executable.

These checks establish production asset packaging. They do not constitute a full Windows installation or GUI/clipboard test; those remain unverified. The 0.2.0 binaries should not be used.

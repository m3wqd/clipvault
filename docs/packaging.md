# Packaging provenance

Delivered artifacts: Windows x64 application EXE and a per-user NSIS Setup containing that executable plus the official Microsoft WebView2 bootstrapper. SHA256SUMS.txt identifies the exact delivered files.

The custom installer source is packaging/scripts/installer.nsi. Rebuild with makensis, passing APP_EXE, SETUP_OUT and WEBVIEW_BOOTSTRAPPER as absolute file paths. Download the bootstrapper from https://go.microsoft.com/fwlink/?LinkId=2124703 and verify Microsoft's signature on Windows before public release. The installer checks the WebView2 runtime registry version and runs the bundled bootstrapper only if missing. The runtime download requires internet. The installer does not enable autostart or delete history.

Cross-build app command used: cargo xwin build --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc --release --features custom-protocol. Build npm assets before Cargo so the correct frontend is embedded. SDK/cache/tooling were kept outside source deliverables in work/.

Native Windows packaging is also prepared in build-windows.ps1 and Tauri configuration (NSIS + WiX MSI). That route uses Tauri's templates, whereas the delivered cross-built Setup uses our NSIS script. Switching installer routes needs upgrade testing; do not assume registry/uninstall identity equivalence. MSI's configured upgrade GUID is stable, but no MSI was generated in this environment.

No installer/application signature is configured; expect an unknown publisher warning. Do not publish this preview as a signed or verified release. No automated updater artifacts or keys exist. Linux AppImage/DEB configuration and a local-source Arch PKGBUILD are provided, not built packages.

## 0.2.1 repair

The original direct Cargo build omitted tauri/custom-protocol. Cargo release optimization does not select Tauri production mode: without that feature Tauri uses devUrl and does not embed the frontend. The corrected cross-build script explicitly enables custom-protocol. build.rs rejects release desktop builds without the feature. A production-context regression test checks embedded HTML and its referenced JavaScript without starting a development server.

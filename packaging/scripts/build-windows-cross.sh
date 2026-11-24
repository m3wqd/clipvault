#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
npm ci
npm run build
# cargo --release alone does not select Tauri production mode.
cargo xwin build --manifest-path src-tauri/Cargo.toml --target x86_64-pc-windows-msvc --release --features custom-protocol --locked
python3 tests/verify_windows_assets.py src-tauri/target/x86_64-pc-windows-msvc/release/clipvault.exe

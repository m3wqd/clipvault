fn main() {
    #[cfg(feature = "desktop")]
    {
        // Direct Cargo cross-builds must re-embed Vite assets after frontend changes.
        println!("cargo:rerun-if-changed=../dist");
        tauri_build::build();
    }
}

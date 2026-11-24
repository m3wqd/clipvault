fn main() {
    #[cfg(feature = "desktop")]
    {
        if std::env::var("PROFILE").as_deref() == Ok("release")
            && std::env::var_os("CARGO_FEATURE_CUSTOM_PROTOCOL").is_none()
        {
            panic!("Release builds require --features custom-protocol to embed the frontend; use the Tauri CLI or the checked-in cross-build script.");
        }
        // Direct Cargo cross-builds must re-embed Vite assets after frontend changes.
        println!("cargo:rerun-if-changed=../dist");
        tauri_build::build();
    }
}

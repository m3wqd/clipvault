use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
pub struct DesktopState {
    pub close_to_tray: AtomicBool,
    pub shortcut: Mutex<String>,
}
pub fn popup(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("popup") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}
pub fn update(app: &tauri::AppHandle, s: &crate::settings::Settings) -> crate::error::Result<()> {
    let state = app.state::<DesktopState>();
    let mut current = state
        .shortcut
        .lock()
        .map_err(|_| crate::error::Error::Invalid("Shortcut state locked".into()))?;
    let wayland = cfg!(target_os = "linux") && std::env::var_os("WAYLAND_DISPLAY").is_some();
    if !wayland && *current != s.shortcut {
        app.global_shortcut().register(s.shortcut.as_str()).map_err(|_|crate::error::Error::Invalid("Shortcut unavailable. On Wayland bind the compositor to launch ClipVault instead.".into()))?;
        if !current.is_empty() {
            let _ = app.global_shortcut().unregister(current.as_str());
        }
        *current = s.shortcut.clone();
    }
    state
        .close_to_tray
        .store(s.close_to_tray, Ordering::Relaxed);
    Ok(())
}
pub fn setup(
    app: &mut tauri::App,
    s: &crate::settings::Settings,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    tauri::WebviewWindowBuilder::new(
        app,
        "popup",
        tauri::WebviewUrl::App("index.html?popup=1".into()),
    )
    .title("ClipVault Quick Access")
    .inner_size(620., 520.)
    .visible(false)
    .always_on_top(true)
    .decorations(false)
    .build()?;
    let open = MenuItem::with_id(app, "open", "Open ClipVault", true, None::<&str>)?;
    let quick = MenuItem::with_id(app, "quick", "Quick Popup", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pause / Resume tracking", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quick, &pause, &quit])?;
    let image = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))?;
    let tray = TrayIconBuilder::new()
        .icon(image)
        .tooltip("ClipVault")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            "quick" => popup(app),
            "pause" => {
                let handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let state = handle.state::<super::Service>();
                    if let Ok(super::Reply::Status(status)) =
                        super::request(state, super::Request::Status).await
                    {
                        let state = handle.state::<super::Service>();
                        let _ = super::request(
                            state,
                            super::Request::Pause {
                                value: !status.paused,
                            },
                        )
                        .await;
                    }
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app);
    match tray {
        Ok(_) => {
            if let Err(e) = update(app.handle(), s) {
                tracing::warn!("Desktop shortcut registration unavailable: {e}");
            }
        }
        Err(_) => {
            tracing::warn!("System tray unavailable; closing the main window will exit");
        }
    }
    Ok(())
}

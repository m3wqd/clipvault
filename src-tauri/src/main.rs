#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod database;
mod desktop;
mod error;
mod images;
mod platform;
mod settings;
use database::{Database, Item};
use serde::{Deserialize, Serialize};
use std::{
    sync::{mpsc, Mutex},
    time::Duration,
};
use tauri::{Emitter, Manager};
#[derive(Clone, Serialize)]
struct Status {
    paused: bool,
    incognito: bool,
    backend: String,
    error: Option<String>,
}
#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum Request {
    List {
        query: String,
        pinned: bool,
        kind: String,
        #[serde(default)]
        favorite: bool,
        #[serde(default)]
        collection: Option<i64>,
        #[serde(default)]
        tag: String,
        #[serde(default)]
        since: i64,
        #[serde(default)]
        frequent: bool,
        #[serde(default)]
        offset: u32,
    },
    Copy {
        id: i64,
    },
    Pin {
        id: i64,
        value: bool,
    },
    Favorite {
        id: i64,
        value: bool,
    },
    Delete {
        id: i64,
    },
    Pause {
        value: bool,
    },
    Status,
    Settings,
    SaveSettings {
        settings: settings::Settings,
    },
    Snippets {
        query: String,
    },
    SaveSnippet {
        id: Option<i64>,
        title: String,
        body: String,
        category: String,
    },
    DeleteSnippet {
        id: i64,
    },
    CopyText {
        text: String,
    },
    Collections,
    RenameCollection {
        id: i64,
        name: String,
    },
    DeleteCollection {
        id: i64,
    },
    CreateCollection {
        name: String,
    },
    Assign {
        id: i64,
        collection: i64,
        value: bool,
    },
    Tags {
        id: i64,
    },
    SetTags {
        id: i64,
        names: Vec<String>,
    },
    Clear {
        all: bool,
        confirmation: String,
    },
    Thumbnail {
        id: i64,
    },
}
#[derive(Serialize)]
#[serde(untagged)]
enum Reply {
    Items(Vec<Item>),
    Status(Status),
    Settings(settings::Settings),
    Snippets(Vec<database::Snippet>),
    Collections(Vec<database::Named>),
    Tags(Vec<String>),
    Text(String),
    Done(()),
}
struct Message {
    request: Request,
    reply: tauri::async_runtime::Sender<std::result::Result<Reply, String>>,
}
struct Service {
    sender: Mutex<mpsc::SyncSender<Message>>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    worker: Mutex<Option<std::thread::JoinHandle<()>>>,
}
#[tauri::command]
async fn request(
    state: tauri::State<'_, Service>,
    request: Request,
) -> std::result::Result<Reply, String> {
    let (tx, mut rx) = tauri::async_runtime::channel(1);
    state
        .sender
        .lock()
        .map_err(|_| "Service lock unavailable")?
        .try_send(Message { request, reply: tx })
        .map_err(|_| "Clipboard service busy or unavailable")?;
    rx.recv()
        .await
        .ok_or_else(|| "Clipboard service stopped".to_string())?
}
fn main() {
    tracing_subscriber::fmt().with_target(false).init();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            if args.iter().any(|a| a == "--popup") {
                desktop::popup(app);
                return;
            }
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        desktop::popup(app);
                    }
                })
                .build(),
        )
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "popup"
                    || window
                        .app_handle()
                        .state::<desktop::DesktopState>()
                        .close_to_tray
                        .load(std::sync::atomic::Ordering::Relaxed)
                {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
            }
            let db = Database::open(&dir.join("history.db"))?;
            let preferences = db.settings()?;
            db.prune()?;
            let image_dir = dir.join("images");
            std::fs::create_dir_all(&image_dir)?;
            images::reconcile(&image_dir, db.image_paths()?)?;
            app.manage(desktop::DesktopState {
                close_to_tray: std::sync::atomic::AtomicBool::new(false),
                shortcut: Mutex::new(String::new()),
            });
            desktop::setup(app, &preferences)?;
            let (sender, receiver) = mpsc::sync_channel::<Message>(32);
            let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let worker_stop = stop.clone();
            let handle = app.handle().clone();
            let worker = std::thread::Builder::new()
                .name("clipboard-service".into())
                .spawn(move || run(db, receiver, handle, image_dir, worker_stop))?;
            app.manage(Service {
                sender: Mutex::new(sender),
                stop,
                worker: Mutex::new(Some(worker)),
            });
            if std::env::args().any(|a| a == "--popup") {
                desktop::popup(app.handle());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![request])
        .build(tauri::generate_context!())
        .expect("ClipVault failed to start")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                let service = app.state::<Service>();
                service
                    .stop
                    .store(true, std::sync::atomic::Ordering::Release);
                if let Ok(mut slot) = service.worker.lock() {
                    if let Some(worker) = slot.take() {
                        let _ = worker.join();
                    }
                };
            }
        });
}
fn run(
    mut db: Database,
    rx: mpsc::Receiver<Message>,
    app: tauri::AppHandle,
    image_dir: std::path::PathBuf,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    let mut status = Status {
        paused: false,
        incognito: db.settings().map(|s| s.incognito).unwrap_or(false),
        backend: "Unavailable".into(),
        error: None,
    };
    let mut backend: Option<platform::Backend> = None;
    let mut last = None;
    let mut next = std::time::Instant::now();
    while !stop.load(std::sync::atomic::Ordering::Acquire) {
        let timeout = next.saturating_duration_since(std::time::Instant::now());
        match rx.recv_timeout(timeout) {
            Ok(msg) => {
                let result = (|| -> error::Result<Reply> {
                    match msg.request {
                        Request::List {
                            query,
                            pinned,
                            kind,
                            favorite,
                            collection,
                            tag,
                            since,
                            frequent,
                            offset,
                        } => Ok(Reply::Items(db.list(&database::Search {
                            query: &query,
                            pinned,
                            kind: &kind,
                            favorite,
                            collection,
                            tag: &tag,
                            since,
                            frequent,
                            offset,
                        })?)),
                        Request::Status => {
                            status.incognito = db.settings()?.incognito;
                            Ok(Reply::Status(status.clone()))
                        }
                        Request::Pause { value } => {
                            status.paused = value;
                            let _ = app.emit("tracking-status", &status);
                            Ok(Reply::Status(status.clone()))
                        }
                        Request::Pin { id, value } => {
                            db.pin(id, value)?;
                            let _ = app.emit("history-changed", ());
                            Ok(Reply::Done(()))
                        }
                        Request::Delete { id } => {
                            db.delete(id)?;
                            images::reconcile(&image_dir, db.image_paths()?)?;
                            let _ = app.emit("history-changed", ());
                            Ok(Reply::Done(()))
                        }
                        Request::Settings => Ok(Reply::Settings(db.settings()?)),
                        Request::SaveSettings { settings } => {
                            settings.validate()?;
                            desktop::update(&app, &settings)?;
                            db.save_settings(&settings)?;
                            status.incognito = settings.incognito;
                            let _ = app.emit("tracking-status", &status);
                            db.prune()?;
                            images::reconcile(&image_dir, db.image_paths()?)?;
                            let _ = app.emit("preferences-changed", ());
                            Ok(Reply::Done(()))
                        }
                        Request::Snippets { query } => Ok(Reply::Snippets(db.snippets(&query)?)),
                        Request::SaveSnippet {
                            id,
                            title,
                            body,
                            category,
                        } => {
                            db.save_snippet(id, &title, &body, &category)?;
                            let _ = app.emit("history-changed", ());
                            Ok(Reply::Done(()))
                        }
                        Request::DeleteSnippet { id } => {
                            db.delete_snippet(id)?;
                            let _ = app.emit("history-changed", ());
                            Ok(Reply::Done(()))
                        }
                        Request::Collections => Ok(Reply::Collections(db.collections()?)),
                        Request::RenameCollection { id, name } => {
                            db.rename_collection(id, &name)?;
                            let _ = app.emit("history-changed", ());
                            Ok(Reply::Done(()))
                        }
                        Request::DeleteCollection { id } => {
                            db.delete_collection(id)?;
                            let _ = app.emit("history-changed", ());
                            Ok(Reply::Done(()))
                        }
                        Request::CreateCollection { name } => {
                            db.collection(&name)?;
                            let _ = app.emit("history-changed", ());
                            Ok(Reply::Done(()))
                        }
                        Request::Assign {
                            id,
                            collection,
                            value,
                        } => {
                            db.assign(id, collection, value)?;
                            let _ = app.emit("history-changed", ());
                            Ok(Reply::Done(()))
                        }
                        Request::Tags { id } => Ok(Reply::Tags(db.get_tags(id)?)),
                        Request::SetTags { id, names } => {
                            db.tags(id, names)?;
                            let _ = app.emit("history-changed", ());
                            Ok(Reply::Done(()))
                        }
                        Request::Clear { all, confirmation } => {
                            if confirmation != "DELETE" {
                                return Err(error::Error::Invalid("Type DELETE to confirm".into()));
                            }
                            db.clear(all)?;
                            images::reconcile(&image_dir, db.image_paths()?)?;
                            let _ = app.emit("history-changed", ());
                            Ok(Reply::Done(()))
                        }
                        Request::Thumbnail { id } => {
                            let name = db
                                .image_path(id)?
                                .ok_or_else(|| error::Error::Invalid("Not an image".into()))?;
                            Ok(Reply::Text(images::thumbnail(&image_dir, &name)?))
                        }
                        Request::Favorite { id, value } => {
                            db.favorite(id, value)?;
                            let _ = app.emit("history-changed", ());
                            Ok(Reply::Done(()))
                        }
                        Request::CopyText { text } => {
                            if text.len() > db.settings()?.max_bytes {
                                return Err(error::Error::Invalid("Text too large".into()));
                            }
                            backend
                                .as_mut()
                                .ok_or_else(|| {
                                    error::Error::Invalid("Clipboard unavailable".into())
                                })?
                                .write(text.clone())?;
                            last = Some(blake3::hash(text.as_bytes()));
                            Ok(Reply::Done(()))
                        }
                        Request::Copy { id } => {
                            if let Some(name) = db.image_path(id)? {
                                let data = images::load(&image_dir, &name)?;
                                backend
                                    .as_mut()
                                    .ok_or_else(|| {
                                        error::Error::Invalid("Clipboard unavailable".into())
                                    })?
                                    .write_image(data)?;
                                last = Some(blake3::hash(name.trim_end_matches(".png").as_bytes()));
                                return Ok(Reply::Done(()));
                            }
                            let text = db.text(id)?;
                            backend
                                .as_mut()
                                .ok_or_else(|| {
                                    error::Error::Invalid("Clipboard backend unavailable".into())
                                })?
                                .write(text.clone())?;
                            last = Some(blake3::hash(text.as_bytes()));
                            Ok(Reply::Done(()))
                        }
                    }
                })();
                let _ = msg.reply.blocking_send(result.map_err(|e| e.to_string()));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
        if std::time::Instant::now() < next {
            continue;
        }
        next = std::time::Instant::now() + Duration::from_millis(750);
        let preferences = match db.settings() {
            Ok(s) => s,
            Err(e) => {
                status.error = Some(e.to_string());
                let _ = app.emit("tracking-status", &status);
                continue;
            }
        };
        status.incognito = preferences.incognito;
        let previous = status.error.clone();
        if backend.is_none() {
            match platform::Backend::new() {
                Ok(b) => {
                    status.backend = b.name().into();
                    backend = Some(b)
                }
                Err(e) => {
                    status.error = Some(e.to_string());
                    next += Duration::from_secs(4);
                }
            }
        }
        if status.paused || preferences.incognito {
            continue;
        }
        let source = platform::source_app();
        if source.as_ref().is_some_and(|name| {
            preferences
                .excluded_apps
                .iter()
                .any(|excluded| excluded.eq_ignore_ascii_case(name))
        }) {
            continue;
        }
        if let Some(b) = backend.as_mut() {
            match b.read() {
                Ok(Some(text)) => {
                    status.error = None;
                    let hash = blake3::hash(text.as_bytes());
                    if last != Some(hash) {
                        if preferences.skip_sensitive && settings::potentially_sensitive(&text) {
                            last = Some(hash);
                            continue;
                        }
                        match db.add(&text) {
                            Ok(true) => {
                                if let Err(e) =
                                    db.record_source(hash.to_hex().as_str(), source.as_deref())
                                {
                                    status.error = Some(e.to_string());
                                }
                                if let Ok(paths) = db.image_paths() {
                                    if let Err(e) = images::reconcile(&image_dir, paths) {
                                        status.error = Some(e.to_string());
                                    }
                                }
                                let _ = app.emit("history-changed", ());
                                last = Some(hash);
                            }
                            Ok(false) => last = Some(hash),
                            Err(e) => {
                                status.error = Some(e.to_string());
                                next += Duration::from_secs(2)
                            }
                        }
                    }
                }
                Ok(None) => {
                    status.error = None;
                    match b.read_image() {
                        Ok(Some(data)) => {
                            match images::save(&image_dir, data, preferences.max_bytes) {
                                Ok((name, hash)) => {
                                    let h = blake3::hash(hash.as_bytes());
                                    if last != Some(h) {
                                        if let Err(e) = db.add_image(&name, &hash) {
                                            status.error = Some(e.to_string())
                                        } else {
                                            if let Err(e) =
                                                db.record_source(&hash, source.as_deref())
                                            {
                                                status.error = Some(e.to_string());
                                            }
                                            let _ = app.emit("history-changed", ());
                                            last = Some(h);
                                            if let Ok(paths) = db.image_paths() {
                                                if let Err(e) = images::reconcile(&image_dir, paths)
                                                {
                                                    status.error = Some(e.to_string());
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => status.error = Some(e.to_string()),
                            }
                        }
                        Ok(None) => last = None,
                        Err(e) => status.error = Some(e.to_string()),
                    }
                }
                Err(e) => {
                    status.error = Some(e.to_string());
                    next += Duration::from_secs(2);
                }
            }
        }
        if previous != status.error {
            let _ = app.emit("tracking-status", &status);
        }
    }
}

#[cfg(all(test, feature = "custom-protocol"))]
mod production_tests {
    #[test]
    fn production_context_embeds_html_and_javascript_without_dev_server() {
        assert!(!tauri::is_dev(), "Production must not use devUrl");
        let context: tauri::Context<tauri::Wry> = tauri::generate_context!();
        let html = context
            .assets()
            .get(&"index.html".into())
            .expect("Embedded index.html missing");
        let html = std::str::from_utf8(&html).expect("HTML must be UTF-8");
        let src = html
            .split("src=\"")
            .nth(1)
            .expect("Script reference missing")
            .split('"')
            .next()
            .expect("Script reference malformed");
        assert!(
            !src.starts_with("http"),
            "Frontend must reference a local asset"
        );
        assert!(
            context
                .assets()
                .get(&src.trim_start_matches('/').into())
                .is_some(),
            "Embedded JavaScript missing"
        );
    }
}

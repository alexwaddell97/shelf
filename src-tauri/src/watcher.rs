//! Watches folders (Downloads by default) and imports ebooks that land in them.

use crate::library::{self, format_of};
use crate::state::AppState;
use notify::event::ModifyKind;
use notify::{EventKind, RecursiveMode, Watcher};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

/// A file must keep the same size for this long before it's imported, so we don't
/// grab a download that's still being written.
const SETTLE: Duration = Duration::from_secs(2);

fn is_partial_download(path: &PathBuf) -> bool {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    [".crdownload", ".part", ".partial", ".download", ".tmp"]
        .iter()
        .any(|ext| name.ends_with(ext))
}

/// Stores selling DRM-protected books often hand out an `.acsm` file: a licence that Adobe
/// Digital Editions exchanges for the actual (protected) book.
fn is_drm_licence(path: &std::path::Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("acsm"))
}

/// (Re)starts watching the folders in settings. Call again after settings change.
pub fn restart(app: &AppHandle) {
    let state = app.state::<AppState>();
    let folders = match state.settings() {
        Ok(settings) => settings.watch_folders,
        Err(_) => return,
    };
    let mut slot = state.watcher.lock().unwrap_or_else(|p| p.into_inner());
    *slot = None; // dropping the old watcher stops it and ends its worker thread

    let (tx, rx) = mpsc::channel::<notify::Result<notify::Event>>();
    let Ok(mut watcher) = notify::recommended_watcher(tx) else {
        return;
    };
    let mut watching = false;
    for folder in &folders {
        if folder.is_dir() && watcher.watch(folder, RecursiveMode::NonRecursive).is_ok() {
            watching = true;
        }
    }
    if !watching {
        return;
    }
    *slot = Some(watcher);

    let app = app.clone();
    std::thread::spawn(move || {
        // path → (last seen size, when it last changed)
        let mut pending: HashMap<PathBuf, (u64, Instant)> = HashMap::new();
        let mut warned: HashSet<PathBuf> = HashSet::new();
        loop {
            match rx.recv_timeout(Duration::from_millis(700)) {
                Ok(Ok(event)) => {
                    let relevant = matches!(
                        event.kind,
                        EventKind::Create(_)
                            | EventKind::Modify(ModifyKind::Name(_))
                            | EventKind::Modify(ModifyKind::Data(_))
                            | EventKind::Modify(ModifyKind::Any)
                    );
                    if relevant {
                        for path in event.paths {
                            // An Adobe DRM licence, not a book: say so instead of ignoring it.
                            if is_drm_licence(&path) && warned.insert(path.clone()) {
                                let name = path
                                    .file_name()
                                    .map(|n| n.to_string_lossy().into_owned())
                                    .unwrap_or_default();
                                let _ = app.emit("drm-download", name);
                                continue;
                            }
                            if !is_partial_download(&path) && format_of(&path).is_some() {
                                let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                                pending.insert(path, (size, Instant::now()));
                            }
                        }
                    }
                }
                Ok(Err(_)) => {}
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }

            let mut ready = Vec::new();
            pending.retain(|path, (size, since)| {
                let Ok(meta) = std::fs::metadata(path) else {
                    return false;
                };
                if meta.len() != *size {
                    *size = meta.len();
                    *since = Instant::now();
                    return true;
                }
                if *size > 0 && since.elapsed() >= SETTLE {
                    ready.push(path.clone());
                    return false;
                }
                true
            });
            for path in ready {
                // Duplicates are rejected by hash inside import_file, so repeated events are harmless.
                let _ = library::import_file(&app, &path);
            }
        }
    });
}

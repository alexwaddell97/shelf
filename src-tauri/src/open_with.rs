//! Opening ebook files with Shelf: double-clicking one in the file manager (once Shelf is
//! the app for that type), dragging one onto Shelf's icon, or "Open with → Shelf".
//!
//! The file is added to the library (or found, if it's already there) and its book page
//! is shown. If Shelf is already running, the second launch hands its files to the
//! running window instead of opening another one.

use crate::db;
use crate::library::{self, format_of, hash_file};
use crate::state::AppState;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager};

/// The ebook files among command-line arguments (skipping flags and the program itself).
pub fn ebook_args(args: impl IntoIterator<Item = String>) -> Vec<PathBuf> {
    args.into_iter()
        .filter(|a| !a.starts_with('-'))
        .map(PathBuf::from)
        .filter(|p| format_of(p).is_some() && p.is_file())
        .collect()
}

fn bring_to_front(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Imports (or finds) the files and shows the last one. Runs in the background.
pub fn open_files(app: &AppHandle, files: Vec<PathBuf>) {
    if files.is_empty() {
        return;
    }
    bring_to_front(app);
    let app = app.clone();
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let mut last = None;
        for file in &files {
            let existing = hash_file(file)
                .ok()
                .and_then(|hash| db::book_by_hash(&state.db(), &hash).ok().flatten());
            let id = match existing {
                Some(id) => Some(id),
                None => library::import_file(&app, file).ok(),
            };
            last = id.or(last);
        }
        if let Some(id) = last {
            // The interface may still be starting; it collects this with take_pending_open.
            *state.pending_open.lock().unwrap_or_else(|p| p.into_inner()) = Some(id);
            let _ = app.emit("open-book", id);
        }
    });
}

/// Opens the system's default-apps settings so the user can choose Shelf for ebooks.
/// Returns false where there's no such page to open (the interface shows instructions).
pub fn open_default_apps_settings(app: &AppHandle) -> bool {
    if cfg!(windows) {
        use tauri_plugin_opener::OpenerExt;
        app.opener()
            .open_url("ms-settings:defaultapps", None::<&str>)
            .is_ok()
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_ebooks_out_of_arguments() {
        let dir = tempfile::tempdir().unwrap();
        let book = dir.path().join("a.epub");
        std::fs::write(&book, b"x").unwrap();
        let args = vec![
            "C:\\Program Files\\Shelf\\shelf.exe".to_string(),
            "--flag".to_string(),
            book.display().to_string(),
            dir.path().join("missing.epub").display().to_string(),
            dir.path().join("notes.txt").display().to_string(),
        ];
        assert_eq!(ebook_args(args), vec![book]);
    }
}

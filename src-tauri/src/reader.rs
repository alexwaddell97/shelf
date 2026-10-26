//! Reader windows: each book opens in its own window (`reader-<id>`), which loads the
//! interface's `/reader/<id>` page. Opening a book that's already open brings its window
//! forward. Windows open at the size you last left a reader window.

use crate::db;
use crate::error::{Error, Result};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent};

/// Formats the reader can display. Anything else opens in another app.
pub const READABLE: &[&str] = &["epub", "kepub", "mobi", "azw3"];

#[derive(Serialize, Deserialize, Clone, Copy)]
struct WindowSize {
    width: f64,
    height: f64,
}

const DEFAULT_SIZE: WindowSize = WindowSize {
    width: 880.0,
    height: 1040.0,
};

pub fn open(app: &AppHandle, book_id: i64) -> Result<()> {
    let label = format!("reader-{book_id}");
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
        return Ok(());
    }

    let state = app.state::<AppState>();
    let book = db::get_book(&state.db(), book_id)?;
    if !READABLE.contains(&book.format.as_str()) {
        return Err(Error::msg(format!(
            "Shelf's reader can't open {} files yet. Use “Open in another app” instead.",
            book.format.to_uppercase()
        )));
    }
    let size: WindowSize = db::kv_get(&state.db(), "reader_window")?.unwrap_or(DEFAULT_SIZE);

    let window = WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::App(format!("reader/{book_id}").into()),
    )
    .title(format!("{} — Shelf", book.title))
    .inner_size(size.width, size.height)
    .min_inner_size(420.0, 520.0)
    .build()?;

    // Remember the size for the next reader window.
    let handle = app.clone();
    let tracked = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::Resized(physical) = event {
            if physical.width == 0 || physical.height == 0 {
                return; // minimised
            }
            let scale = tracked.scale_factor().unwrap_or(1.0);
            let size = WindowSize {
                width: physical.width as f64 / scale,
                height: physical.height as f64 / scale,
            };
            let state = handle.state::<AppState>();
            let _ = db::kv_set(&state.db(), "reader_window", &size);
        }
    });
    Ok(())
}

//! Commands the frontend calls through `invoke` (see `src/lib/api.ts`).

use crate::calibre;
use crate::catalog;
use crate::covers;
use crate::db;
use crate::devices::{self, kobo};
use crate::error::{Error, Result};
use crate::goodreads;
use crate::library;
use crate::model::*;
use crate::state::AppState;
use crate::tools;
use crate::watcher;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| Error::msg(e.to_string()))?
}

// --- Library ---

#[tauri::command]
pub fn list_books(state: State<'_, AppState>) -> Result<Vec<Book>> {
    db::list_books(&state.db())
}

#[tauri::command]
pub fn get_book(state: State<'_, AppState>, id: i64) -> Result<Book> {
    db::get_book(&state.db(), id)
}

#[tauri::command]
pub async fn import_files(app: AppHandle, paths: Vec<PathBuf>) -> Result<Vec<i64>> {
    blocking(move || Ok(library::import_files(&app, &paths))).await
}

#[tauri::command]
pub fn update_book(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    patch: BookPatch,
) -> Result<Book> {
    let conn = state.db();
    if db::update_book(&conn, id, &patch)? {
        // The e-reader shows what's inside its copy, so send it again with the changes.
        db::requeue_on_devices(&conn, id)?;
    }
    let book = db::get_book(&conn, id)?;
    let _ = app.emit("library-changed", ());
    Ok(book)
}

#[tauri::command]
pub fn delete_book(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    delete_file: bool,
) -> Result<()> {
    let book = db::get_book(&state.db(), id)?;
    db::delete_book(&state.db(), id)?;
    if let Some(cover) = &book.cover_path {
        let _ = std::fs::remove_file(cover);
    }
    if delete_file {
        let _ = std::fs::remove_file(&book.file_path);
    }
    let _ = app.emit("library-changed", ());
    Ok(())
}

#[tauri::command]
pub fn reveal_book(state: State<'_, AppState>, id: i64) -> Result<()> {
    let book = db::get_book(&state.db(), id)?;
    tauri_plugin_opener::reveal_item_in_dir(&book.file_path).map_err(|e| Error::msg(e.to_string()))
}

#[tauri::command]
pub fn open_book(app: AppHandle, state: State<'_, AppState>, id: i64) -> Result<()> {
    let book = db::get_book(&state.db(), id)?;
    app.opener()
        .open_path(&book.file_path, None::<&str>)
        .map_err(|_| Error::msg("No app on this computer can open this kind of ebook."))
}

// --- Devices ---

#[tauri::command]
pub fn list_devices(state: State<'_, AppState>) -> Result<Vec<Device>> {
    devices::list(&state)
}

#[tauri::command]
pub fn refresh_devices(app: AppHandle, state: State<'_, AppState>) -> Result<Vec<Device>> {
    if devices::refresh(&app) {
        let _ = app.emit("devices-changed", ());
    }
    devices::list(&state)
}

#[tauri::command]
pub fn queue_for_device(
    app: AppHandle,
    state: State<'_, AppState>,
    device_id: String,
    book_ids: Vec<i64>,
) -> Result<()> {
    let conn = state.db();
    for id in book_ids {
        db::queue_add(&conn, &device_id, id)?;
    }
    let _ = app.emit("library-changed", ());
    Ok(())
}

#[tauri::command]
pub fn unqueue(
    app: AppHandle,
    state: State<'_, AppState>,
    device_id: String,
    book_id: i64,
) -> Result<()> {
    db::queue_remove(&state.db(), &device_id, book_id)?;
    let _ = app.emit("library-changed", ());
    Ok(())
}

fn mount_of(state: &AppState, device_id: &str) -> Result<PathBuf> {
    state.connected().get(device_id).cloned().ok_or_else(|| {
        Error::msg("That device isn't connected. Plug it in over USB and try again.")
    })
}

#[tauri::command]
pub async fn sync_device(app: AppHandle, device_id: String) -> Result<SyncReport> {
    blocking(move || {
        let state = app.state::<AppState>();
        let mount = mount_of(&state, &device_id)?;
        let report = devices::sync(
            &state,
            &device_id,
            &mount,
            &|phase, done, total, current| {
                let _ = app.emit(
                    "sync-progress",
                    SyncProgress {
                        device_id: device_id.clone(),
                        phase,
                        done,
                        total,
                        current,
                    },
                );
            },
        )?;
        let _ = app.emit("library-changed", ());
        Ok(report)
    })
    .await
}

#[tauri::command]
pub fn remove_from_device(
    app: AppHandle,
    state: State<'_, AppState>,
    device_id: String,
    book_id: i64,
) -> Result<()> {
    let mount = mount_of(&state, &device_id)?;
    let conn = state.db();
    if let Some(rel) = db::device_book_path(&conn, &device_id, book_id)? {
        kobo::remove_book(&mount, &rel)?;
    }
    db::device_book_remove(&conn, &device_id, book_id)?;
    let _ = app.emit("library-changed", ());
    Ok(())
}

#[tauri::command]
pub fn forget_device(app: AppHandle, state: State<'_, AppState>, device_id: String) -> Result<()> {
    db::forget_device(&state.db(), &device_id)?;
    let _ = app.emit("devices-changed", ());
    let _ = app.emit("library-changed", ());
    Ok(())
}

// --- Sources, catalogue and wishlist ---

#[tauri::command]
pub fn list_sources(state: State<'_, AppState>) -> Result<Vec<Source>> {
    state.sources()
}

#[tauri::command]
pub fn save_sources(state: State<'_, AppState>, sources: Vec<Source>) -> Result<()> {
    for source in &sources {
        if !source.url_template.contains("{query}") {
            return Err(Error::msg(format!(
                "{}: the URL needs {{query}} where the search text goes.",
                source.name
            )));
        }
        if !(source.url_template.starts_with("https://")
            || source.url_template.starts_with("http://"))
        {
            return Err(Error::msg(format!(
                "{}: the URL must start with https://",
                source.name
            )));
        }
    }
    state.save_sources(&sources)
}

#[tauri::command]
pub async fn search_catalog(state: State<'_, AppState>, query: String) -> Result<Vec<CatalogBook>> {
    catalog::search(&state.http, &query).await
}

#[tauri::command]
pub async fn download_free(
    app: AppHandle,
    state: State<'_, AppState>,
    book: CatalogBook,
) -> Result<i64> {
    let url = book
        .epub_url
        .clone()
        .ok_or_else(|| Error::msg("This book has no free download."))?;
    let bytes = catalog::download(&state.http, &url).await?;
    let file = state
        .paths
        .tmp_dir
        .join(format!("{}.epub", library::sanitize(&book.title)));
    std::fs::write(&file, bytes)?;
    let result = blocking({
        let app = app.clone();
        let file = file.clone();
        move || library::import_file(&app, &file)
    })
    .await;
    let _ = std::fs::remove_file(&file);
    result
}

#[tauri::command]
pub fn list_wishlist(state: State<'_, AppState>) -> Result<Vec<WishItem>> {
    db::list_wishlist(&state.db())
}

#[tauri::command]
pub fn add_to_wishlist(
    state: State<'_, AppState>,
    book: CatalogBook,
    origin: String,
) -> Result<WishItem> {
    db::add_wish(&state.db(), &book, &origin)
}

#[tauri::command]
pub fn remove_from_wishlist(state: State<'_, AppState>, id: i64) -> Result<()> {
    db::remove_wish(&state.db(), id)
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> Result<()> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(Error::msg("Only web links can be opened."));
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| Error::msg(e.to_string()))
}

// --- Settings ---

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<Settings> {
    state.settings()
}

#[tauri::command]
pub fn save_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> Result<()> {
    state.save_settings(&settings)?;
    watcher::restart(&app);
    Ok(())
}

#[tauri::command]
pub fn get_tools() -> Tools {
    tools::find()
}

// --- calibre import ---

#[tauri::command]
pub fn find_calibre_library() -> Option<String> {
    calibre::default_library().map(|p| p.display().to_string())
}

#[tauri::command]
pub async fn inspect_calibre(app: AppHandle, path: PathBuf) -> Result<calibre::CalibreSummary> {
    blocking(move || calibre::inspect(&app.state::<AppState>(), &path)).await
}

#[tauri::command]
pub async fn import_calibre(app: AppHandle, path: PathBuf) -> Result<calibre::CalibreReport> {
    blocking(move || {
        let state = app.state::<AppState>();
        let report = calibre::import(&state, &path, &|done, total, current| {
            let _ = app.emit(
                "calibre-progress",
                SyncProgress {
                    device_id: String::new(),
                    phase: "importing",
                    done,
                    total,
                    current: current.map(str::to_string),
                },
            );
            // Refresh the grid now and then so books appear as they arrive.
            if done % 25 == 0 {
                let _ = app.emit("library-changed", ());
            }
        })?;
        let _ = app.emit("library-changed", ());
        Ok(report)
    })
    .await
}

// --- Goodreads ---

#[tauri::command]
pub fn get_goodreads(state: State<'_, AppState>) -> Result<Option<goodreads::Account>> {
    goodreads::account(&state)
}

#[tauri::command]
pub async fn connect_goodreads(
    app: AppHandle,
    state: State<'_, AppState>,
    profile: String,
) -> Result<goodreads::Account> {
    let user_id = goodreads::parse_user_id(&profile).ok_or_else(|| {
        Error::msg("Paste the link to your Goodreads profile. It looks like goodreads.com/user/show/12345-yourname.")
    })?;
    let account = goodreads::check(&state, &user_id).await?;
    let _ = app.emit("library-changed", ());
    Ok(account)
}

#[tauri::command]
pub async fn check_goodreads(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<goodreads::Account> {
    let account =
        goodreads::account(&state)?.ok_or_else(|| Error::msg("Goodreads isn't connected."))?;
    let account = goodreads::check(&state, &account.user_id).await?;
    let _ = app.emit("library-changed", ());
    Ok(account)
}

#[tauri::command]
pub fn disconnect_goodreads(state: State<'_, AppState>) -> Result<()> {
    goodreads::disconnect(&state)
}

#[tauri::command]
pub async fn import_goodreads_csv(app: AppHandle, path: PathBuf) -> Result<goodreads::Applied> {
    blocking(move || {
        let state = app.state::<AppState>();
        let entries = goodreads::read_csv(&path)?;
        // An export lists every shelf, but it's a one-off: keep existing Goodreads wishes.
        let applied = goodreads::apply(&state, &entries, false)?;
        let _ = app.emit("library-changed", ());
        Ok(applied)
    })
    .await
}

// --- Tags and collections ---

fn check_kind(kind: &str) -> Result<()> {
    match kind {
        "tag" | "collection" => Ok(()),
        _ => Err(Error::msg("Unknown label kind.")),
    }
}

#[tauri::command]
pub fn list_labels(state: State<'_, AppState>, kind: String) -> Result<Vec<db::Label>> {
    check_kind(&kind)?;
    db::list_labels(&state.db(), &kind)
}

#[tauri::command]
pub fn create_label(
    app: AppHandle,
    state: State<'_, AppState>,
    kind: String,
    name: String,
) -> Result<()> {
    check_kind(&kind)?;
    db::create_label(&state.db(), &kind, &name)?;
    let _ = app.emit("library-changed", ());
    Ok(())
}

#[tauri::command]
pub fn rename_label(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    name: String,
) -> Result<()> {
    db::rename_label(&state.db(), id, &name)?;
    let _ = app.emit("library-changed", ());
    Ok(())
}

#[tauri::command]
pub fn delete_label(app: AppHandle, state: State<'_, AppState>, id: i64) -> Result<()> {
    db::delete_label(&state.db(), id)?;
    let _ = app.emit("library-changed", ());
    Ok(())
}

#[tauri::command]
pub fn label_books(
    app: AppHandle,
    state: State<'_, AppState>,
    label_id: i64,
    book_ids: Vec<i64>,
    add: bool,
) -> Result<()> {
    db::label_books(&state.db(), label_id, &book_ids, add)?;
    let _ = app.emit("library-changed", ());
    Ok(())
}

// --- Covers ---

#[tauri::command]
pub async fn set_cover_from_file(app: AppHandle, id: i64, path: PathBuf) -> Result<Book> {
    blocking(move || {
        let state = app.state::<AppState>();
        covers::from_file(&state, id, &path)?;
        let _ = app.emit("library-changed", ());
        db::get_book(&state.db(), id)
    })
    .await
}

#[tauri::command]
pub async fn set_cover_from_url(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    url: String,
) -> Result<Book> {
    let bytes = covers::download(&state.http, &url).await?;
    blocking(move || {
        let state = app.state::<AppState>();
        covers::save(&state, id, &bytes)?;
        let _ = app.emit("library-changed", ());
        db::get_book(&state.db(), id)
    })
    .await
}

#[tauri::command]
pub fn remove_cover(app: AppHandle, state: State<'_, AppState>, id: i64) -> Result<Book> {
    covers::remove(&state, id)?;
    let _ = app.emit("library-changed", ());
    db::get_book(&state.db(), id)
}

#[tauri::command]
pub async fn find_covers(
    state: State<'_, AppState>,
    id: i64,
) -> Result<Vec<covers::CoverCandidate>> {
    let book = db::get_book(&state.db(), id)?;
    covers::search(&state.http, &book.title, &book.author).await
}

// --- Opening files with Shelf ---

/// A book opened from the file manager before the interface was ready, if any.
#[tauri::command]
pub fn take_pending_open(state: State<'_, AppState>) -> Option<i64> {
    state
        .pending_open
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .take()
}

/// Opens the system's default-apps settings. Returns false where there's no such page.
#[tauri::command]
pub fn open_default_apps_settings(app: AppHandle) -> bool {
    crate::open_with::open_default_apps_settings(&app)
}

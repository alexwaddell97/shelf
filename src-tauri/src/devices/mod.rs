//! E-reader drivers and the background poller that notices them being plugged in.
//!
//! Each kind of device gets a driver module. v0.1 ships the Kobo driver; others
//! (PocketBook, Tolino, Kindle, ...) slot in beside it.

pub mod kobo;

use crate::db;
use crate::error::Result;
use crate::model::{Device, SyncReport};
use crate::state::AppState;
use crate::tools;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// A device found mounted on this computer.
pub struct Mounted {
    pub id: String,
    pub name: String,
    pub kind: &'static str,
    pub mount: PathBuf,
}

/// Places where USB storage devices show up. For testing without a real device, set
/// `SHELF_DEVICE_ROOTS` to a list of folders (separated like PATH) that look like one.
fn candidate_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = std::env::var_os("SHELF_DEVICE_ROOTS")
        .map(|v| std::env::split_paths(&v).collect())
        .unwrap_or_default();
    if cfg!(windows) {
        // C: is never an e-reader; skip it and the floppy letters.
        for letter in b'D'..=b'Z' {
            roots.push(PathBuf::from(format!("{}:\\", letter as char)));
        }
    } else {
        let user = std::env::var("USER").unwrap_or_default();
        let parents = [
            "/Volumes".to_string(),
            format!("/media/{user}"),
            format!("/run/media/{user}"),
            "/media".to_string(),
        ];
        for parent in parents {
            if let Ok(entries) = std::fs::read_dir(&parent) {
                roots.extend(entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()));
            }
        }
    }
    roots
}

pub fn detect_all() -> Vec<Mounted> {
    candidate_roots()
        .iter()
        .filter_map(|root| kobo::detect(root))
        .collect()
}

/// Stops Windows from popping up "There is no disk in the drive" dialogs while we
/// probe empty card readers.
pub fn quiet_drive_errors() {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Diagnostics::Debug::{
            SEM_FAILCRITICALERRORS, SetErrorMode,
        };
        SetErrorMode(SEM_FAILCRITICALERRORS);
    }
}

/// Re-scans for devices, records newly seen ones, and returns whether anything changed.
pub fn refresh(app: &AppHandle) -> bool {
    let state = app.state::<AppState>();
    let found = detect_all();
    let now: HashMap<String, PathBuf> = found
        .iter()
        .map(|m| (m.id.clone(), m.mount.clone()))
        .collect();
    let changed = {
        let mut connected = state.connected();
        let changed = *connected != now;
        *connected = now;
        changed
    };
    if changed {
        let conn = state.db();
        for device in &found {
            let _ = db::upsert_device(&conn, &device.id, &device.name, device.kind);
        }
    }
    changed
}

/// Polls every few seconds and emits `devices-changed` when a device appears or goes away.
pub fn start_polling(app: AppHandle) {
    std::thread::spawn(move || {
        loop {
            if refresh(&app) {
                let _ = app.emit("devices-changed", ());
                let _ = app.emit("library-changed", ());
            }
            std::thread::sleep(Duration::from_secs(3));
        }
    });
}

/// Sends a device's queued books, then reads reading progress back and forgets books
/// that were deleted on the device. `progress` is called with (done, total, current title).
pub fn sync(
    state: &AppState,
    device_id: &str,
    mount: &Path,
    progress: &dyn Fn(usize, usize, Option<String>),
) -> Result<SyncReport> {
    let settings = state.settings()?;
    let kepubify = tools::kepubify(&state.paths.data_dir);
    let queued = db::queued_books(&state.db(), device_id)?;
    let total = queued.len();
    let mut report = SyncReport {
        device_id: device_id.to_string(),
        sent: 0,
        progress_updated: 0,
        errors: Vec::new(),
    };

    for (done, book_id) in queued.into_iter().enumerate() {
        let Ok(book) = db::get_book(&state.db(), book_id) else {
            continue;
        };
        progress(done, total, Some(book.title.clone()));
        let existing = db::device_book_path(&state.db(), device_id, book_id)?;
        match kobo::send_book(
            mount,
            &book,
            existing.as_deref(),
            kepubify.as_deref(),
            settings.convert_for_device,
            &state.paths.tmp_dir,
        ) {
            Ok(rel) => {
                let conn = state.db();
                db::device_book_add(&conn, device_id, book_id, &rel)?;
                db::queue_remove(&conn, device_id, book_id)?;
                report.sent += 1;
            }
            Err(err) => report.errors.push(format!("{}: {err}", book.title)),
        }
    }
    progress(total, total, None);

    let backup = settings
        .backup_device_db
        .then(|| state.paths.data_dir.join("device-backups").join(device_id));
    let readings = kobo::read_progress(mount, &state.paths.tmp_dir, backup.as_deref())
        .unwrap_or_else(|err| {
            report
                .errors
                .push(format!("Couldn't read reading progress: {err}"));
            Vec::new()
        });
    let conn = state.db();
    for (book_id, rel) in db::device_books(&conn, device_id)? {
        if !mount.join(&rel).exists() {
            db::device_book_remove(&conn, device_id, book_id)?;
            continue;
        }
        if let Some(reading) = readings.iter().find(|r| r.rel_path == rel) {
            let (pct, status) = kobo::shelf_status(reading);
            db::set_progress(&conn, book_id, pct, status, reading.last_read.as_deref())?;
            report.progress_updated += 1;
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library;
    use crate::model::CatalogBook;
    use std::fs;

    /// The whole loop: a download is imported (matching a wishlist item and queueing
    /// itself), sent to a Kobo, read on the Kobo, and finally deleted there.
    #[test]
    fn imports_sends_and_reads_progress_back() {
        let data = tempfile::tempdir().unwrap();
        let mount = tempfile::tempdir().unwrap();
        let downloads = tempfile::tempdir().unwrap();
        fs::create_dir_all(mount.path().join(".kobo")).unwrap();
        fs::write(mount.path().join(".kobo/version"), "N0000TEST,1,2,3,4,5").unwrap();

        let state = AppState::open(data.path().to_path_buf(), true).unwrap();
        let device = kobo::detect(mount.path()).unwrap();
        db::upsert_device(&state.db(), &device.id, &device.name, device.kind).unwrap();
        let wish = CatalogBook {
            key: "/works/OL1W".into(),
            title: "The Time Machine".into(),
            author: "H. G. Wells".into(),
            year: None,
            cover_url: None,
            is_free: false,
            epub_url: None,
        };
        db::add_wish(&state.db(), &wish, "search").unwrap();

        // Import.
        let file = downloads.path().join("the_time_machine.epub");
        crate::epub::write_test_epub(&file, "The Time Machine", "H. G. Wells");
        let (id, matched) = library::import_into(&state, &file, None, &|_| {}).unwrap();
        assert!(matched, "the wishlist item should match");
        assert!(db::list_wishlist(&state.db()).unwrap().is_empty());
        let book = db::get_book(&state.db(), id).unwrap();
        assert!(Path::new(&book.file_path).is_file());
        assert!(
            book.cover_path
                .as_deref()
                .is_some_and(|p| Path::new(p).is_file())
        );
        assert_eq!(book.queued_for, vec![device.id.clone()]);
        assert!(
            library::import_into(&state, &file, None, &|_| {}).is_err(),
            "duplicates are rejected"
        );

        // First sync sends it.
        let report = sync(&state, &device.id, mount.path(), &|_, _, _| {}).unwrap();
        assert_eq!(report.sent, 1, "{:?}", report.errors);
        let rel = db::device_book_path(&state.db(), &device.id, id)
            .unwrap()
            .unwrap();
        assert!(mount.path().join(&rel).is_file());
        assert!(db::get_book(&state.db(), id).unwrap().queued_for.is_empty());

        // The Kobo records some reading; the next sync picks it up.
        let kobo_db =
            rusqlite::Connection::open(mount.path().join(".kobo/KoboReader.sqlite")).unwrap();
        kobo_db
            .execute_batch("CREATE TABLE content (ContentID TEXT, ContentType INTEGER, ___PercentRead INTEGER, ReadStatus INTEGER, DateLastRead TEXT);")
            .unwrap();
        kobo_db
            .execute(
                "INSERT INTO content VALUES (?1, 6, 37, 1, '2026-10-24T20:00:00Z')",
                [format!("file:///mnt/onboard/{rel}")],
            )
            .unwrap();
        drop(kobo_db);
        let report = sync(&state, &device.id, mount.path(), &|_, _, _| {}).unwrap();
        assert_eq!(report.progress_updated, 1, "{:?}", report.errors);
        let book = db::get_book(&state.db(), id).unwrap();
        assert_eq!((book.progress, book.status.as_str()), (37.0, "reading"));
        assert_eq!(book.on_devices, vec![device.id.clone()]);
        assert!(
            data.path()
                .join("device-backups")
                .join(&device.id)
                .join("KoboReader.sqlite")
                .is_file()
        );

        // Editing it re-queues it; the next sync rewrites the same file with the changes.
        let patch: crate::model::BookPatch = serde_json::from_str(
            r#"{"title":"The Time Machine (Annotated)","series":"Wells Classics","seriesIndex":2}"#,
        )
        .unwrap();
        assert!(db::update_book(&state.db(), id, &patch).unwrap());
        db::requeue_on_devices(&state.db(), id).unwrap();
        let report = sync(&state, &device.id, mount.path(), &|_, _, _| {}).unwrap();
        assert_eq!(report.sent, 1, "{:?}", report.errors);
        assert_eq!(
            db::device_book_path(&state.db(), &device.id, id)
                .unwrap()
                .as_deref(),
            Some(rel.as_str()),
            "same path, so the Kobo keeps the reading position"
        );
        let on_device = crate::epub::read(&mount.path().join(&rel)).unwrap();
        assert_eq!(
            on_device.title.as_deref(),
            Some("The Time Machine (Annotated)")
        );
        assert_eq!(on_device.series.as_deref(), Some("Wells Classics"));
        assert_eq!(on_device.series_index, Some(2.0));
        let library_copy =
            crate::epub::read(Path::new(&db::get_book(&state.db(), id).unwrap().file_path))
                .unwrap();
        assert_eq!(
            library_copy.title.as_deref(),
            Some("The Time Machine"),
            "the library file is untouched"
        );

        // Deleted on the device: Shelf stops listing it there.
        fs::remove_file(mount.path().join(&rel)).unwrap();
        sync(&state, &device.id, mount.path(), &|_, _, _| {}).unwrap();
        assert!(db::get_book(&state.db(), id).unwrap().on_devices.is_empty());
    }
}

/// All known devices, with live details for the connected ones.
pub fn list(state: &AppState) -> crate::error::Result<Vec<Device>> {
    let known = db::known_devices(&state.db())?;
    let connected = state.connected().clone();
    Ok(known
        .into_iter()
        .map(|d| {
            let mount = connected.get(&d.id);
            let (free, total) = match mount {
                Some(path) => (fs2::available_space(path).ok(), fs2::total_space(path).ok()),
                None => (None, None),
            };
            let kobo = d.kind == kobo::KIND;
            Device {
                connected: mount.is_some(),
                mount_path: mount.map(|p| p.display().to_string()),
                last_seen: Some(d.last_seen),
                support_level: if kobo {
                    "full".into()
                } else {
                    "planned".into()
                },
                send_format: if kobo { "kepub".into() } else { "epub".into() },
                free_bytes: free,
                total_bytes: total,
                id: d.id,
                name: d.name,
                kind: d.kind,
            }
        })
        .collect())
}

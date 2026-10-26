//! E-reader drivers and the background poller that notices them being plugged in.
//!
//! Each kind of device gets a driver module. v0.1 ships the Kobo driver; others
//! (PocketBook, Tolino, Kindle, ...) slot in beside it.

pub mod kobo;

use crate::db;
use crate::error::Result;
use crate::model::{Device, SyncReport};
use crate::state::AppState;
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
        {
            let conn = state.db();
            for device in &found {
                let _ = db::upsert_device(&conn, &device.id, &device.name, device.kind);
            }
        }
        for device in &found {
            reconcile_on_connect(&state, device);
        }
    }
    changed
}

/// Brings Shelf's record of a device up to date with what's actually on it:
/// - books already there that Shelf didn't send (e.g. from calibre) are linked to the
///   matching library books by title and author, so they show as on the device and
///   later updates overwrite them in place;
/// - books deleted on the device are forgotten;
/// - reading progress is copied across (only when the device has some, so a book marked
///   finished elsewhere isn't reset by an untouched copy).
///
/// Returns (books linked, books whose progress was updated).
pub fn reconcile(
    state: &AppState,
    device_id: &str,
    mount: &Path,
    readings: &[kobo::Reading],
) -> Result<(usize, usize)> {
    let conn = state.db();
    let books = db::list_books(&conn)?;
    let mut on_device: HashMap<String, i64> = db::device_books(&conn, device_id)?
        .into_iter()
        .map(|(id, rel)| (rel, id))
        .collect();
    let mut linked: std::collections::HashSet<i64> = on_device.values().copied().collect();

    let mut adopted = 0;
    for reading in readings {
        if on_device.contains_key(&reading.rel_path) || !mount.join(&reading.rel_path).is_file() {
            continue;
        }
        let Some(title) = reading.title.as_deref().filter(|t| !t.trim().is_empty()) else {
            continue;
        };
        let author = reading.author.as_deref().unwrap_or("");
        let found = books
            .iter()
            .find(|b| !linked.contains(&b.id) && db::same_work(&b.title, &b.author, title, author));
        if let Some(book) = found {
            db::device_book_add(&conn, device_id, book.id, &reading.rel_path)?;
            // It's already there, so there's nothing to send.
            db::queue_remove(&conn, device_id, book.id)?;
            linked.insert(book.id);
            on_device.insert(reading.rel_path.clone(), book.id);
            adopted += 1;
        }
    }

    let mut updated = 0;
    for (rel, book_id) in &on_device {
        if !mount.join(rel).exists() {
            db::device_book_remove(&conn, device_id, *book_id)?;
            continue;
        }
        if let Some(reading) = readings.iter().find(|r| &r.rel_path == rel) {
            if reading.status == 0 && reading.percent <= 0.0 {
                continue;
            }
            let (pct, status) = kobo::shelf_status(reading);
            db::set_progress(&conn, *book_id, pct, status, reading.last_read.as_deref())?;
            updated += 1;
        }
    }
    Ok((adopted, updated))
}

/// Reads a newly connected device's book list and reconciles it (see `reconcile`).
fn reconcile_on_connect(state: &AppState, device: &Mounted) {
    if device.kind != kobo::KIND {
        return;
    }
    if let Ok(readings) = kobo::read_progress(&device.mount, &state.paths.tmp_dir, None) {
        let _ = reconcile(state, &device.id, &device.mount, &readings);
    }
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
    progress: &dyn Fn(&'static str, usize, usize, Option<String>),
) -> Result<SyncReport> {
    let settings = state.settings()?;
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
        progress("sending", done, total, Some(book.title.clone()));
        let existing = db::device_book_path(&state.db(), device_id, book_id)?;
        match kobo::send_book(
            mount,
            &book,
            existing.as_deref(),
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
    // Sending is done; reading the device's database and updating progress comes next.
    progress("reading", total, total, None);

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
    let (_, updated) = reconcile(state, device_id, mount, &readings)?;
    report.progress_updated = updated;
    db::kv_set(
        &state.db(),
        &last_sync_key(device_id),
        &chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    )?;
    Ok(report)
}

fn last_sync_key(device_id: &str) -> String {
    format!("last_sync:{device_id}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library;
    use crate::model::CatalogBook;
    use std::fs;

    /// Dry run of linking against a real library and a real Kobo, working on a copy of
    /// the Shelf database (the device is only read). Run with:
    /// `SHELF_TEST_DATA=<Shelf data folder> SHELF_TEST_KOBO=F:\ cargo test real_kobo_linking -- --ignored --nocapture`
    #[test]
    #[ignore = "needs SHELF_TEST_DATA and SHELF_TEST_KOBO"]
    fn real_kobo_linking() {
        let (Some(data), Some(mount)) = (
            std::env::var_os("SHELF_TEST_DATA"),
            std::env::var_os("SHELF_TEST_KOBO"),
        ) else {
            return;
        };
        let mount = PathBuf::from(mount);
        let copy = tempfile::tempdir().unwrap();
        // Copy the write-ahead log too, or the most recent changes are missing.
        for name in ["shelf.db", "shelf.db-wal"] {
            let from = Path::new(&data).join(name);
            if from.is_file() {
                fs::copy(&from, copy.path().join(name)).unwrap();
            }
        }
        let state = AppState::open(copy.path().to_path_buf(), true).unwrap();
        let device = kobo::detect(&mount).expect("a Kobo at SHELF_TEST_KOBO");
        db::upsert_device(&state.db(), &device.id, &device.name, device.kind).unwrap();
        let readings = kobo::read_progress(&mount, &state.paths.tmp_dir, None).unwrap();
        let (linked, progress) = reconcile(&state, &device.id, &mount, &readings).unwrap();
        let on_device: std::collections::HashSet<String> =
            db::device_books(&state.db(), &device.id)
                .unwrap()
                .into_iter()
                .map(|(_, rel)| rel)
                .collect();
        let unmatched: Vec<&kobo::Reading> = readings
            .iter()
            .filter(|r| !on_device.contains(&r.rel_path))
            .collect();
        println!(
            "{} books on the Kobo; linked {linked}; progress copied for {progress}; not matched {}",
            readings.len(),
            unmatched.len()
        );
        let books = db::list_books(&state.db()).unwrap();
        let links: HashMap<i64, String> = db::device_books(&state.db(), &device.id)
            .unwrap()
            .into_iter()
            .collect();
        for r in unmatched {
            let title = r.title.clone().unwrap_or_default();
            let author = r.author.clone().unwrap_or_default();
            match books
                .iter()
                .find(|b| db::same_work(&b.title, &b.author, &title, &author))
            {
                Some(b) => println!(
                    "NOT MATCHED: {title:?} by {author:?}; library book {:?} by {:?} is linked to {:?}",
                    b.title,
                    b.author,
                    links.get(&b.id)
                ),
                None => println!("NOT MATCHED: {title:?} by {author:?}; no library book matches"),
            }
        }
    }

    /// Books calibre already put on the Kobo are recognised and linked, not sent again.
    #[test]
    fn links_books_another_app_already_sent() {
        let data = tempfile::tempdir().unwrap();
        let mount = tempfile::tempdir().unwrap();
        fs::create_dir_all(mount.path().join(".kobo")).unwrap();
        fs::write(mount.path().join(".kobo/version"), "N0000TEST,1,2,3,4,5").unwrap();
        let state = AppState::open(data.path().to_path_buf(), true).unwrap();
        let device = kobo::detect(mount.path()).unwrap();
        db::upsert_device(&state.db(), &device.id, &device.name, device.kind).unwrap();

        let mut ids = Vec::new();
        for (title, author) in [
            ("Six of Crows", "Leigh Bardugo"),
            ("Dune", "Frank Herbert"),
            ("Emma", "Jane Austen"),
        ] {
            let file = data.path().join(format!("{title}.epub"));
            crate::epub::write_test_epub(&file, title, author);
            ids.push(
                library::import_into(&state, &file, None, &|_| {})
                    .unwrap()
                    .0,
            );
        }
        db::queue_add(&state.db(), &device.id, ids[1]).unwrap();

        // calibre's layout on the device, plus a store book and a stale database entry.
        for rel in [
            "Bardugo, Leigh/Six of Crows - Leigh Bardugo.kepub.epub",
            "Herbert, Frank/Dune - Frank Herbert.kepub.epub",
        ] {
            let path = mount.path().join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"kepub").unwrap();
        }
        let kobo_db =
            rusqlite::Connection::open(mount.path().join(".kobo/KoboReader.sqlite")).unwrap();
        kobo_db
            .execute_batch(
                "CREATE TABLE content (ContentID TEXT, ContentType INTEGER, ___PercentRead INTEGER, ReadStatus INTEGER, DateLastRead TEXT, Title TEXT, Attribution TEXT);
                 INSERT INTO content VALUES ('file:///mnt/onboard/Bardugo, Leigh/Six of Crows - Leigh Bardugo.kepub.epub', 6, 55, 1, '2026-10-01T09:00:00Z', 'Six of Crows', 'Leigh Bardugo');
                 INSERT INTO content VALUES ('file:///mnt/onboard/Herbert, Frank/Dune - Frank Herbert.kepub.epub', 6, 0, 0, NULL, 'Dune', 'Frank Herbert');
                 INSERT INTO content VALUES ('file:///mnt/onboard/Austen, Jane/Emma - Jane Austen.kepub.epub', 6, 10, 1, NULL, 'Emma', 'Jane Austen');
                 INSERT INTO content VALUES ('0b1b9e3d-store-book', 6, 20, 1, NULL, 'A Store Book', 'Someone');",
            )
            .unwrap();
        drop(kobo_db);

        let readings = kobo::read_progress(mount.path(), &state.paths.tmp_dir, None).unwrap();
        assert_eq!(
            reconcile(&state, &device.id, mount.path(), &readings).unwrap(),
            (2, 1)
        );

        let crows = db::get_book(&state.db(), ids[0]).unwrap();
        assert_eq!(crows.on_devices, vec![device.id.clone()]);
        assert_eq!((crows.progress, crows.status.as_str()), (55.0, "reading"));
        let dune = db::get_book(&state.db(), ids[1]).unwrap();
        assert_eq!(dune.on_devices, vec![device.id.clone()]);
        assert!(
            dune.queued_for.is_empty(),
            "already there, so not sent again"
        );
        assert_eq!(
            dune.status, "unread",
            "an untouched copy doesn't change the status"
        );
        let emma = db::get_book(&state.db(), ids[2]).unwrap();
        assert!(
            emma.on_devices.is_empty(),
            "the database lists it, but the file isn't there"
        );
        assert_eq!(
            db::device_book_path(&state.db(), &device.id, ids[0])
                .unwrap()
                .as_deref(),
            Some("Bardugo, Leigh/Six of Crows - Leigh Bardugo.kepub.epub"),
            "updates will overwrite calibre's copy in place"
        );

        assert_eq!(
            reconcile(&state, &device.id, mount.path(), &readings)
                .unwrap()
                .0,
            0,
            "linking happens once"
        );
    }

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
        let report = sync(&state, &device.id, mount.path(), &|_, _, _, _| {}).unwrap();
        assert_eq!(report.sent, 1, "{:?}", report.errors);
        let known = list(&state).unwrap();
        assert!(
            known
                .iter()
                .find(|d| d.id == device.id)
                .unwrap()
                .last_synced
                .is_some()
        );
        let rel = db::device_book_path(&state.db(), &device.id, id)
            .unwrap()
            .unwrap();
        assert!(mount.path().join(&rel).is_file());
        assert!(db::get_book(&state.db(), id).unwrap().queued_for.is_empty());

        // The Kobo records some reading; the next sync picks it up.
        let kobo_db =
            rusqlite::Connection::open(mount.path().join(".kobo/KoboReader.sqlite")).unwrap();
        kobo_db
            .execute_batch("CREATE TABLE content (ContentID TEXT, ContentType INTEGER, ___PercentRead INTEGER, ReadStatus INTEGER, DateLastRead TEXT, Title TEXT, Attribution TEXT);")
            .unwrap();
        kobo_db
            .execute(
                "INSERT INTO content VALUES (?1, 6, 37, 1, '2026-10-24T20:00:00Z', 'The Time Machine', 'H. G. Wells')",
                [format!("file:///mnt/onboard/{rel}")],
            )
            .unwrap();
        drop(kobo_db);
        let report = sync(&state, &device.id, mount.path(), &|_, _, _, _| {}).unwrap();
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
        let report = sync(&state, &device.id, mount.path(), &|_, _, _, _| {}).unwrap();
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
        sync(&state, &device.id, mount.path(), &|_, _, _, _| {}).unwrap();
        assert!(db::get_book(&state.db(), id).unwrap().on_devices.is_empty());
    }
}

/// All known devices, with live details for the connected ones.
pub fn list(state: &AppState) -> crate::error::Result<Vec<Device>> {
    let conn = state.db();
    let known = db::known_devices(&conn)?;
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
            let last_synced = db::kv_get::<String>(&conn, &last_sync_key(&d.id))
                .ok()
                .flatten();
            Device {
                connected: mount.is_some(),
                mount_path: mount.map(|p| p.display().to_string()),
                last_seen: Some(d.last_seen),
                last_synced,
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

//! Kobo e-readers. They mount as USB storage with a `.kobo` folder at the root; the
//! reader's own library database is `.kobo/KoboReader.sqlite`.

use super::Mounted;
use crate::error::{Error, Result};
use crate::library::sanitize;
use crate::model::Book;
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const KIND: &str = "kobo";

/// Folder on the device that Shelf sends books into.
const BOOKS_DIR: &str = "Shelf";

/// How the Kobo refers to files on its own storage in `content.ContentID`.
const ONBOARD_PREFIX: &str = "file:///mnt/onboard/";

/// Recognises a Kobo mounted at `root`. `.kobo/version` holds comma-separated fields,
/// the first of which is the serial number.
pub fn detect(root: &Path) -> Option<Mounted> {
    let version = fs::read_to_string(root.join(".kobo").join("version")).ok()?;
    let serial = version
        .split(',')
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())?;
    Some(Mounted {
        id: format!("kobo-{serial}"),
        name: "Kobo eReader".into(),
        kind: KIND,
        mount: root.to_path_buf(),
    })
}

/// The device-relative path for a book, with forward slashes.
fn device_rel_path(book: &Book, ext: &str) -> String {
    let stem = sanitize(&format!("{} - {}", book.author, book.title));
    format!("{BOOKS_DIR}/{stem}.{ext}")
}

/// Runs kepubify on an EPUB, returning the converted file.
fn convert_to_kepub(kepubify: &Path, input: &Path, work_dir: &Path) -> Result<PathBuf> {
    let out_dir = work_dir.join(format!("kepub-{}", std::process::id()));
    let _ = fs::remove_dir_all(&out_dir);
    fs::create_dir_all(&out_dir)?;
    let mut cmd = Command::new(kepubify);
    cmd.arg("-o").arg(&out_dir).arg(input);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let output = cmd.output()?;
    if !output.status.success() {
        return Err(Error::msg(format!(
            "kepubify couldn't convert it: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    fs::read_dir(&out_dir)?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.is_file())
        .ok_or_else(|| Error::msg("kepubify didn't produce a file."))
}

/// Copies a book onto the device, converting EPUB to kepub when kepubify is available.
/// Returns the path on the device, relative to its root.
/// Copies a book onto the device, with Shelf's metadata and cover written into the copy,
/// converting EPUB to kepub when kepubify is available. A book that's already on the
/// device (`existing`) is overwritten at the same path, so its reading position is kept.
/// Returns the path on the device, relative to its root.
pub fn send_book(
    mount: &Path,
    book: &Book,
    existing: Option<&str>,
    kepubify: Option<&Path>,
    convert: bool,
    work_dir: &Path,
) -> Result<String> {
    let source = PathBuf::from(&book.file_path);
    if !source.is_file() {
        return Err(Error::msg(
            "The book's file is missing from your library folder.",
        ));
    }
    let is_epub = matches!(book.format.as_str(), "epub" | "kepub");

    // Write metadata and cover into a temporary copy. If the file is too unusual to edit,
    // fall back to sending it exactly as it is.
    let mut temps: Vec<PathBuf> = Vec::new();
    let mut file = source.clone();
    if is_epub {
        let cover = book.cover_path.as_ref().and_then(|p| fs::read(p).ok());
        let tagged = work_dir.join(format!("send-{}.epub", book.id));
        let cover = cover.filter(|bytes| image::guess_format(bytes).is_ok());
        if crate::epub_write::write_for_device(&source, &tagged, book, cover.as_deref()).is_ok() {
            temps.push(tagged.clone());
            file = tagged;
        }
    }
    let ext = match (book.format.as_str(), kepubify) {
        ("epub", Some(tool)) if convert => {
            let converted = convert_to_kepub(tool, &file, work_dir)?;
            temps.push(converted.clone());
            file = converted;
            "kepub.epub".to_string()
        }
        ("kepub", _) => "kepub.epub".to_string(),
        (format, _) => format.to_string(),
    };

    let rel = match existing {
        Some(old) if old.ends_with(&format!(".{ext}")) => old.to_string(),
        Some(old) => {
            // Sent before in a different format: replace it.
            remove_book(mount, old)?;
            device_rel_path(book, &ext)
        }
        None => device_rel_path(book, &ext),
    };
    let dest = mount.join(&rel);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let copied = fs::copy(&file, &dest);
    for temp in temps {
        let _ = fs::remove_file(temp);
    }
    copied?;
    Ok(rel)
}

pub fn remove_book(mount: &Path, rel: &str) -> Result<()> {
    let path = mount.join(rel);
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub struct Reading {
    pub rel_path: String,
    pub percent: f64,
    /// 0 = unread, 1 = reading, 2 = finished (Kobo's ReadStatus).
    pub status: i64,
    pub last_read: Option<String>,
}

/// Reads progress for sideloaded books from the Kobo's database. Works on a copy so the
/// device's own file is never opened for writing.
pub fn read_progress(
    mount: &Path,
    work_dir: &Path,
    backup_to: Option<&Path>,
) -> Result<Vec<Reading>> {
    let db_path = mount.join(".kobo").join("KoboReader.sqlite");
    if !db_path.is_file() {
        return Ok(Vec::new());
    }
    let copy = work_dir.join("KoboReader.sqlite");
    fs::copy(&db_path, &copy)?;
    let wal = mount.join(".kobo").join("KoboReader.sqlite-wal");
    if wal.is_file() {
        fs::copy(&wal, work_dir.join("KoboReader.sqlite-wal"))?;
    }
    if let Some(dir) = backup_to {
        fs::create_dir_all(dir)?;
        fs::copy(&copy, dir.join("KoboReader.sqlite"))?;
    }

    // Opened read-write because it's our own copy and SQLite needs to replay the WAL.
    let conn = Connection::open(&copy)?;
    let mut stmt = conn.prepare(
        "SELECT ContentID, COALESCE(___PercentRead, 0), COALESCE(ReadStatus, 0), DateLastRead
         FROM content WHERE ContentType = 6 AND ContentID LIKE 'file:///mnt/onboard/%'",
    )?;
    let rows = stmt.query_map([], |r| {
        let id: String = r.get(0)?;
        Ok(Reading {
            rel_path: id.trim_start_matches(ONBOARD_PREFIX).to_string(),
            percent: r.get::<_, f64>(1)?,
            status: r.get(2)?,
            last_read: r.get(3)?,
        })
    })?;
    let readings = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    drop(stmt);
    drop(conn);
    let _ = fs::remove_file(&copy);
    let _ = fs::remove_file(work_dir.join("KoboReader.sqlite-wal"));
    Ok(readings)
}

/// Maps Kobo's reading state onto Shelf's (progress 0–100, status name).
pub fn shelf_status(reading: &Reading) -> (f64, &'static str) {
    match reading.status {
        2 => (100.0, "finished"),
        _ if reading.percent > 0.0 => (reading.percent.clamp(0.0, 99.0), "reading"),
        1 => (0.0, "reading"),
        _ => (0.0, "unread"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_a_kobo_by_its_version_file() {
        let dir = tempfile::tempdir().unwrap();
        assert!(detect(dir.path()).is_none());
        fs::create_dir_all(dir.path().join(".kobo")).unwrap();
        fs::write(
            dir.path().join(".kobo/version"),
            "N4181C1234567,4.1.15,4.38.21908,4.1.15,4.1.15,00000000-0000-0000-0000-000000000388",
        )
        .unwrap();
        let found = detect(dir.path()).unwrap();
        assert_eq!(found.id, "kobo-N4181C1234567");
    }

    #[test]
    fn reads_progress_from_a_kobo_database() {
        let mount = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        fs::create_dir_all(mount.path().join(".kobo")).unwrap();
        let conn = Connection::open(mount.path().join(".kobo/KoboReader.sqlite")).unwrap();
        conn.execute_batch(
            "CREATE TABLE content (ContentID TEXT, ContentType INTEGER, ___PercentRead INTEGER, ReadStatus INTEGER, DateLastRead TEXT);
             INSERT INTO content VALUES ('file:///mnt/onboard/Shelf/A - B.kepub.epub', 6, 42, 1, '2026-10-20T10:00:00Z');
             INSERT INTO content VALUES ('file:///mnt/onboard/Shelf/A - B.kepub.epub!!chapter1', 9, 0, 0, NULL);",
        )
        .unwrap();
        drop(conn);
        let readings = read_progress(mount.path(), work.path(), None).unwrap();
        assert_eq!(readings.len(), 1);
        assert_eq!(readings[0].rel_path, "Shelf/A - B.kepub.epub");
        assert_eq!(shelf_status(&readings[0]), (42.0, "reading"));
    }
}

//! Importing an existing calibre library. calibre's own files are never modified:
//! its `metadata.db` is copied and the copy is read.

use crate::db::BookMeta;
use crate::epub;
use crate::error::{Error, Result};
use crate::library::{self, Overrides};
use crate::state::AppState;
use rusqlite::Connection;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Formats to take from a calibre book, best first.
const PREFERRED: &[&str] = &["KEPUB", "EPUB", "AZW3", "MOBI", "PDF"];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibreSummary {
    pub path: String,
    pub book_count: usize,
    /// Books with no format Shelf can use.
    pub unsupported: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibreReport {
    pub imported: usize,
    pub skipped_duplicates: usize,
    pub failed: Vec<String>,
}

struct CalibreBook {
    meta: BookMeta,
    file: Option<PathBuf>,
    cover: Option<PathBuf>,
}

/// Where calibre keeps its current library, from calibre's own settings file.
pub fn default_library() -> Option<PathBuf> {
    let config = if cfg!(windows) {
        dirs::config_dir()?.join("calibre")
    } else if cfg!(target_os = "macos") {
        dirs::home_dir()?
            .join("Library")
            .join("Preferences")
            .join("calibre")
    } else {
        dirs::config_dir()?.join("calibre")
    };
    let text = std::fs::read_to_string(config.join("global.py.json")).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    let path = PathBuf::from(json.get("library_path")?.as_str()?);
    path.join("metadata.db").is_file().then_some(path)
}

fn read_books(library: &Path, work_dir: &Path) -> Result<Vec<CalibreBook>> {
    let source = library.join("metadata.db");
    if !source.is_file() {
        return Err(Error::msg(
            "That folder isn't a calibre library (there's no metadata.db in it).",
        ));
    }
    let copy = work_dir.join("calibre-metadata.db");
    std::fs::copy(&source, &copy)?;
    let conn = Connection::open(&copy)?;
    // Tags are joined with a unit separator, which never appears in a tag name.
    let mut stmt = conn.prepare(
        "SELECT b.id, b.title, b.series_index, b.path, b.has_cover, b.author_sort, b.pubdate,
                (SELECT group_concat(a.name, ' & ' ORDER BY l.id)
                   FROM books_authors_link l JOIN authors a ON a.id = l.author WHERE l.book = b.id),
                (SELECT s.name FROM books_series_link sl JOIN series s ON s.id = sl.series WHERE sl.book = b.id),
                (SELECT c.text FROM comments c WHERE c.book = b.id),
                (SELECT p.name FROM books_publishers_link pl JOIN publishers p ON p.id = pl.publisher WHERE pl.book = b.id),
                (SELECT i.val FROM identifiers i WHERE i.book = b.id AND i.type = 'isbn'),
                (SELECT lg.lang_code FROM books_languages_link ll JOIN languages lg ON lg.id = ll.lang_code
                  WHERE ll.book = b.id ORDER BY ll.item_order LIMIT 1),
                (SELECT r.rating FROM books_ratings_link rl JOIN ratings r ON r.id = rl.rating WHERE rl.book = b.id),
                (SELECT group_concat(t.name, char(31)) FROM books_tags_link tl JOIN tags t ON t.id = tl.tag WHERE tl.book = b.id)
         FROM books b ORDER BY b.timestamp",
    )?;
    let mut formats = conn.prepare("SELECT format, name FROM data WHERE book = ?1")?;
    let mut books = Vec::new();
    let mut rows = stmt.query([])?;
    while let Some(r) = rows.next()? {
        let id: i64 = r.get(0)?;
        let folder = library.join(r.get::<_, String>(3)?);
        let series: Option<String> = r.get(8)?;
        let available: Vec<(String, String)> = formats
            .query_map([id], |f| {
                Ok((f.get::<_, String>(0)?, f.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let file = PREFERRED.iter().find_map(|wanted| {
            available
                .iter()
                .find(|(format, _)| format == wanted)
                .map(|(format, name)| {
                    let ext = if format == "KEPUB" {
                        "kepub".to_string()
                    } else {
                        format.to_lowercase()
                    };
                    folder.join(format!("{name}.{ext}"))
                })
        });
        let meta = BookMeta {
            title: r.get(1)?,
            author: r
                .get::<_, Option<String>>(7)?
                .unwrap_or_else(|| "Unknown author".into()),
            author_sort: r
                .get::<_, Option<String>>(5)?
                .filter(|s| !s.trim().is_empty()),
            series_index: series.as_ref().and(r.get::<_, Option<f64>>(2)?),
            series,
            description: r
                .get::<_, Option<String>>(9)?
                .map(|c| epub::plain_text(&c))
                .filter(|d| !d.is_empty()),
            publisher: r.get(10)?,
            // calibre stores "0101-01-01" for an unknown date; date_part drops it.
            published: r
                .get::<_, Option<String>>(6)?
                .and_then(|d| epub::date_part(&d)),
            isbn: r
                .get::<_, Option<String>>(11)?
                .and_then(|i| epub::isbn_from(&i)),
            language: r.get(12)?,
            // calibre rates 0–10 (half stars); Shelf uses whole stars.
            rating: r
                .get::<_, Option<i64>>(13)?
                .map(|r| ((r + 1) / 2) as u8)
                .filter(|r| *r > 0),
            tags: r
                .get::<_, Option<String>>(14)?
                .map(|t| t.split('\u{1f}').map(str::to_string).collect())
                .unwrap_or_default(),
            collections: Vec::new(),
        };
        books.push(CalibreBook {
            meta,
            file: file.filter(|f| f.is_file()),
            cover: r
                .get::<_, bool>(4)?
                .then(|| folder.join("cover.jpg"))
                .filter(|c| c.is_file()),
        });
    }
    drop(rows);
    drop(formats);
    drop(stmt);
    drop(conn);
    let _ = std::fs::remove_file(&copy);
    Ok(books)
}

pub fn inspect(state: &AppState, library: &Path) -> Result<CalibreSummary> {
    let books = read_books(library, &state.paths.tmp_dir)?;
    Ok(CalibreSummary {
        path: library.display().to_string(),
        book_count: books.len(),
        unsupported: books.iter().filter(|b| b.file.is_none()).count(),
    })
}

/// Imports every calibre book Shelf can read, using calibre's metadata and covers.
/// `progress` is called with (done, total, current title).
pub fn import(
    state: &AppState,
    library: &Path,
    progress: &dyn Fn(usize, usize, Option<&str>),
) -> Result<CalibreReport> {
    let books = read_books(library, &state.paths.tmp_dir)?;
    let total = books.len();
    let mut report = CalibreReport {
        imported: 0,
        skipped_duplicates: 0,
        failed: Vec::new(),
    };
    for (done, book) in books.iter().enumerate() {
        progress(done, total, Some(&book.meta.title));
        let Some(file) = &book.file else {
            report.failed.push(format!(
                "{}: no EPUB, KEPUB, AZW3, MOBI or PDF file",
                book.meta.title
            ));
            continue;
        };
        let overrides = Overrides {
            meta: Some(book.meta.clone()),
            cover_file: book.cover.clone(),
            // Importing a whole library shouldn't queue every book for the e-reader.
            no_auto_send: true,
        };
        match library::import_into(state, file, Some(&overrides), &|_| {}) {
            Ok(_) => report.imported += 1,
            Err(Error::Duplicate) => report.skipped_duplicates += 1,
            Err(err) => report.failed.push(format!("{}: {err}", book.meta.title)),
        }
    }
    progress(total, total, None);
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_library(dir: &Path) {
        let conn = Connection::open(dir.join("metadata.db")).unwrap();
        conn.execute_batch(
            "CREATE TABLE books (id INTEGER PRIMARY KEY, title TEXT, series_index REAL, path TEXT, has_cover BOOL, timestamp TEXT, author_sort TEXT, pubdate TEXT);
             CREATE TABLE publishers (id INTEGER PRIMARY KEY, name TEXT);
             CREATE TABLE books_publishers_link (id INTEGER PRIMARY KEY, book INTEGER, publisher INTEGER);
             CREATE TABLE identifiers (id INTEGER PRIMARY KEY, book INTEGER, type TEXT, val TEXT);
             CREATE TABLE languages (id INTEGER PRIMARY KEY, lang_code TEXT);
             CREATE TABLE books_languages_link (id INTEGER PRIMARY KEY, book INTEGER, lang_code INTEGER, item_order INTEGER);
             CREATE TABLE ratings (id INTEGER PRIMARY KEY, rating INTEGER);
             CREATE TABLE books_ratings_link (id INTEGER PRIMARY KEY, book INTEGER, rating INTEGER);
             CREATE TABLE tags (id INTEGER PRIMARY KEY, name TEXT);
             CREATE TABLE books_tags_link (id INTEGER PRIMARY KEY, book INTEGER, tag INTEGER);
             CREATE TABLE authors (id INTEGER PRIMARY KEY, name TEXT);
             CREATE TABLE books_authors_link (id INTEGER PRIMARY KEY, book INTEGER, author INTEGER);
             CREATE TABLE series (id INTEGER PRIMARY KEY, name TEXT);
             CREATE TABLE books_series_link (id INTEGER PRIMARY KEY, book INTEGER, series INTEGER);
             CREATE TABLE comments (id INTEGER PRIMARY KEY, book INTEGER, text TEXT);
             CREATE TABLE data (id INTEGER PRIMARY KEY, book INTEGER, format TEXT, name TEXT);
             INSERT INTO books VALUES (1, 'Leviathan Wakes', 1.0, 'James S. A. Corey/Leviathan Wakes (1)', 1, '2026-01-01', 'Abraham, Daniel & Franck, Ty', '2011-06-15 00:00:00+00:00');
             INSERT INTO publishers VALUES (1, 'Orbit');
             INSERT INTO books_publishers_link VALUES (1, 1, 1);
             INSERT INTO identifiers VALUES (1, 1, 'isbn', '9780316129084'), (2, 1, 'goodreads', '8855321');
             INSERT INTO languages VALUES (1, 'eng');
             INSERT INTO books_languages_link VALUES (1, 1, 1, 0);
             INSERT INTO ratings VALUES (1, 8);
             INSERT INTO books_ratings_link VALUES (1, 1, 1);
             INSERT INTO tags VALUES (1, 'Science Fiction'), (2, 'Space Opera');
             INSERT INTO books_tags_link VALUES (1, 1, 1), (2, 1, 2);
             INSERT INTO authors VALUES (1, 'Daniel Abraham'), (2, 'Ty Franck');
             INSERT INTO books_authors_link VALUES (1, 1, 1), (2, 1, 2);
             INSERT INTO series VALUES (1, 'The Expanse');
             INSERT INTO books_series_link VALUES (1, 1, 1);
             INSERT INTO comments VALUES (1, 1, '<p>Space <b>noir</b>.</p>');
             INSERT INTO data VALUES (1, 1, 'MOBI', 'Leviathan Wakes - James S. A. Corey'), (2, 1, 'EPUB', 'Leviathan Wakes - James S. A. Corey');
             INSERT INTO books VALUES (2, 'No Files', NULL, 'Someone/No Files (2)', 0, '2026-01-02', 'Someone', '0101-01-01 00:00:00+00:00');",
        )
        .unwrap();
        let folder = dir.join("James S. A. Corey/Leviathan Wakes (1)");
        std::fs::create_dir_all(&folder).unwrap();
        epub::write_test_epub(
            &folder.join("Leviathan Wakes - James S. A. Corey.epub"),
            "Wrong Title In File",
            "Nobody",
        );
        std::fs::write(folder.join("cover.jpg"), b"\xFF\xD8calibre cover").unwrap();
    }

    /// Imports a real calibre library into a temporary folder and checks calibre's own
    /// database is left untouched. Run with:
    /// `SHELF_TEST_CALIBRE_LIBRARY=/path/to/library cargo test real_library -- --ignored --nocapture`
    #[test]
    #[ignore = "needs a real calibre library in SHELF_TEST_CALIBRE_LIBRARY"]
    fn imports_a_real_calibre_library() {
        let Some(library) = std::env::var_os("SHELF_TEST_CALIBRE_LIBRARY").map(PathBuf::from)
        else {
            return;
        };
        let before = std::fs::read(library.join("metadata.db")).unwrap();
        let data = tempfile::tempdir().unwrap();
        let state = AppState::open(data.path().to_path_buf(), true).unwrap();
        let started = std::time::Instant::now();
        let report = import(&state, &library, &|_, _, _| {}).unwrap();
        let books = crate::db::list_books(&state.db()).unwrap();
        let with_cover = books.iter().filter(|b| b.cover_path.is_some()).count();
        let with_series = books.iter().filter(|b| b.series.is_some()).count();
        let with_description = books.iter().filter(|b| b.description.is_some()).count();
        println!(
            "imported {} (duplicates skipped: {}) in {:.1?}; covers {with_cover}, series {with_series}, descriptions {with_description}",
            report.imported,
            report.skipped_duplicates,
            started.elapsed()
        );
        for failure in &report.failed {
            println!("failed: {failure}");
        }
        assert!(report.imported > 0);
        assert!(
            books.iter().all(|b| b.queued_for.is_empty()),
            "bulk imports are not queued"
        );
        assert_eq!(
            std::fs::read(library.join("metadata.db")).unwrap(),
            before,
            "calibre's database is untouched"
        );
    }

    #[test]
    fn imports_with_calibre_metadata_and_cover() {
        let calibre_dir = tempfile::tempdir().unwrap();
        let data = tempfile::tempdir().unwrap();
        make_library(calibre_dir.path());
        let before = std::fs::read(calibre_dir.path().join("metadata.db")).unwrap();
        let state = AppState::open(data.path().to_path_buf(), true).unwrap();

        let summary = inspect(&state, calibre_dir.path()).unwrap();
        assert_eq!((summary.book_count, summary.unsupported), (2, 1));

        let report = import(&state, calibre_dir.path(), &|_, _, _| {}).unwrap();
        assert_eq!(report.imported, 1);
        assert_eq!(report.failed.len(), 1);
        let books = crate::db::list_books(&state.db()).unwrap();
        let book = &books[0];
        assert_eq!(book.title, "Leviathan Wakes");
        assert_eq!(book.author, "Daniel Abraham & Ty Franck");
        assert_eq!(book.series.as_deref(), Some("The Expanse"));
        assert_eq!(book.series_index, Some(1.0));
        assert_eq!(book.description.as_deref(), Some("Space noir."));
        assert_eq!(book.format, "epub");
        assert_eq!(
            book.author_sort.as_deref(),
            Some("Abraham, Daniel & Franck, Ty")
        );
        assert_eq!(book.publisher.as_deref(), Some("Orbit"));
        assert_eq!(book.published.as_deref(), Some("2011-06-15"));
        assert_eq!(book.isbn.as_deref(), Some("9780316129084"));
        assert_eq!(book.language.as_deref(), Some("eng"));
        assert_eq!(book.rating, Some(4));
        assert_eq!(book.tags, vec!["Science Fiction", "Space Opera"]);
        let cover = std::fs::read(book.cover_path.as_ref().unwrap()).unwrap();
        assert_eq!(cover, b"\xFF\xD8calibre cover");

        let again = import(&state, calibre_dir.path(), &|_, _, _| {}).unwrap();
        assert_eq!((again.imported, again.skipped_duplicates), (0, 1));
        assert_eq!(
            std::fs::read(calibre_dir.path().join("metadata.db")).unwrap(),
            before,
            "calibre's database is untouched"
        );
    }
}

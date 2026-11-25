//! One-way import from Goodreads. Goodreads closed its API to new apps in 2020 and retired
//! its shelf RSS feeds in November 2026, and shelf pages now need signing in, so the way in
//! is the library export Goodreads gives you (My Books → Import and export → Export
//! library). Importing it again brings Shelf up to date. Nothing is sent to Goodreads.
//!
//! - "to-read" fills the wishlist (and Goodreads-sourced wishes that left the shelf are removed),
//! - "currently-reading" marks matching unread library books as reading,
//! - "read" marks matching library books as finished.
//!
//! Goodreads only ever moves a book forward (unread → reading → finished); reading
//! progress from an e-reader is never overwritten.

use crate::db::{self, same_work};
use crate::error::{Error, Result};
use crate::model::CatalogBook;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shelf {
    ToRead,
    Reading,
    Read,
}

impl Shelf {
    fn name(self) -> &'static str {
        match self {
            Shelf::ToRead => "to-read",
            Shelf::Reading => "currently-reading",
            Shelf::Read => "read",
        }
    }

    fn from_name(name: &str) -> Option<Shelf> {
        [Shelf::ToRead, Shelf::Reading, Shelf::Read]
            .into_iter()
            .find(|s| s.name() == name)
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub book_id: String,
    pub title: String,
    pub author: String,
    pub shelf: Shelf,
    pub cover_url: Option<String>,
    /// RFC 3339.
    pub read_at: Option<String>,
}

/// What the last export import found and changed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastImport {
    pub file_name: String,
    /// RFC 3339.
    pub imported_at: String,
    pub want_to_read: usize,
    pub reading: usize,
    pub read: usize,
    pub applied: Applied,
}

/// The Goodreads card: the last import, and whether a profile link from before the feeds
/// were retired is still saved (so the app can say why it stopped updating).
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub last_import: Option<LastImport>,
    pub old_profile_link: bool,
}

/// Key of the profile-link connection used while the feeds still worked.
const OLD_ACCOUNT_KEY: &str = "goodreads";
const LAST_IMPORT_KEY: &str = "goodreads_import";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Applied {
    pub added_to_wishlist: usize,
    pub removed_from_wishlist: usize,
    pub marked_reading: usize,
    pub marked_finished: usize,
}

/// Goodreads titles carry the series: "Leviathan Wakes (The Expanse, #1)" → "Leviathan Wakes".
fn clean_title(title: &str) -> String {
    let title = title.trim();
    match title.rfind(" (") {
        Some(pos) if title.ends_with(')') && title[pos..].contains('#') => {
            title[..pos].trim().to_string()
        }
        _ => title.to_string(),
    }
}

/// Applies Goodreads shelves to the library and wishlist.
pub fn apply(state: &AppState, entries: &[Entry], full_to_read_list: bool) -> Result<Applied> {
    let conn = state.db();
    let books = db::list_books(&conn)?;
    let owned = |e: &Entry| {
        books
            .iter()
            .find(|b| same_work(&b.title, &b.author, &e.title, &e.author))
    };
    let mut applied = Applied::default();

    let wishes = db::list_wishlist(&conn)?;
    let to_read: Vec<&Entry> = entries
        .iter()
        .filter(|e| e.shelf == Shelf::ToRead)
        .collect();
    for entry in &to_read {
        if owned(entry).is_some() {
            continue;
        }
        let key = format!("goodreads:{}", entry.book_id);
        let already = wishes.iter().any(|w| {
            w.catalog_key.as_deref() == Some(key.as_str())
                || same_work(&w.title, &w.author, &entry.title, &entry.author)
        });
        if already {
            continue;
        }
        let book = CatalogBook {
            key,
            title: entry.title.clone(),
            author: entry.author.clone(),
            year: None,
            cover_url: entry.cover_url.clone(),
            is_free: false,
            epub_url: None,
        };
        db::add_wish(&conn, &book, "goodreads")?;
        applied.added_to_wishlist += 1;
    }
    if full_to_read_list {
        let keys: HashSet<String> = to_read
            .iter()
            .map(|e| format!("goodreads:{}", e.book_id))
            .collect();
        for wish in wishes.iter().filter(|w| w.origin == "goodreads") {
            if !wish.catalog_key.as_ref().is_some_and(|k| keys.contains(k)) {
                db::remove_wish(&conn, wish.id)?;
                applied.removed_from_wishlist += 1;
            }
        }
    }

    for entry in entries.iter().filter(|e| e.shelf != Shelf::ToRead) {
        let Some(book) = owned(entry) else { continue };
        match entry.shelf {
            Shelf::Reading if book.status == "unread" => {
                db::set_progress(&conn, book.id, book.progress, "reading", None)?;
                applied.marked_reading += 1;
            }
            Shelf::Read if book.status != "finished" => {
                db::set_progress(&conn, book.id, 100.0, "finished", entry.read_at.as_deref())?;
                applied.marked_finished += 1;
            }
            _ => {}
        }
    }
    Ok(applied)
}

pub fn status(state: &AppState) -> Result<Status> {
    let conn = state.db();
    Ok(Status {
        last_import: db::kv_get(&conn, LAST_IMPORT_KEY)?,
        old_profile_link: db::kv_get::<serde_json::Value>(&conn, OLD_ACCOUNT_KEY)?.is_some(),
    })
}

/// Forgets the profile link saved before the feeds were retired.
pub fn forget_old_profile_link(state: &AppState) -> Result<()> {
    state
        .db()
        .execute("DELETE FROM kv WHERE key = ?1", [OLD_ACCOUNT_KEY])?;
    Ok(())
}

/// Imports a library export and remembers what it found. An export lists every shelf, so
/// it's the whole Want to Read list: Goodreads wishes no longer on it leave the wishlist.
pub fn import_export(state: &AppState, path: &Path) -> Result<LastImport> {
    let entries = read_csv(path)?;
    let applied = apply(state, &entries, true)?;
    let count = |shelf| entries.iter().filter(|e| e.shelf == shelf).count();
    let last = LastImport {
        file_name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        imported_at: db::now(),
        want_to_read: count(Shelf::ToRead),
        reading: count(Shelf::Reading),
        read: count(Shelf::Read),
        applied,
    };
    db::kv_set(&state.db(), LAST_IMPORT_KEY, &last)?;
    Ok(last)
}

/// Splits CSV text into rows, handling quoted fields with embedded commas, quotes and newlines.
fn parse_csv(text: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.trim_start_matches('\u{feff}').chars().peekable();
    while let Some(c) = chars.next() {
        match (quoted, c) {
            (true, '"') if chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            (true, '"') => quoted = false,
            (true, c) => field.push(c),
            (false, '"') => quoted = true,
            (false, ',') => row.push(std::mem::take(&mut field)),
            (false, '\n') => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            (false, '\r') => {}
            (false, c) => field.push(c),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

/// Reads a Goodreads library export (My Books → Import and export → Export library).
pub fn read_csv(path: &Path) -> Result<Vec<Entry>> {
    let text = std::fs::read_to_string(path)?;
    let rows = parse_csv(&text);
    let header = rows
        .first()
        .ok_or_else(|| Error::msg("That CSV file is empty."))?;
    let col = |name: &str| header.iter().position(|h| h.trim() == name);
    let (Some(id), Some(title), Some(author), Some(shelf)) = (
        col("Book Id"),
        col("Title"),
        col("Author"),
        col("Exclusive Shelf"),
    ) else {
        return Err(Error::msg(
            "That doesn't look like a Goodreads library export.",
        ));
    };
    let date_read = col("Date Read");
    Ok(rows[1..]
        .iter()
        .filter_map(|row| {
            let get = |i: usize| {
                row.get(i)
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            };
            Some(Entry {
                book_id: get(id)?,
                title: clean_title(&get(title)?),
                author: get(author)
                    .map(|a| db::collapse_spaces(&a))
                    .unwrap_or_else(|| "Unknown author".into()),
                shelf: Shelf::from_name(&get(shelf)?)?,
                cover_url: None,
                read_at: date_read
                    .and_then(get)
                    .and_then(|d| chrono::NaiveDate::parse_from_str(&d, "%Y/%m/%d").ok())
                    .map(|d| format!("{d}T00:00:00Z")),
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_csv_export() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("export.csv");
        std::fs::write(
            &path,
            "Book Id,Title,Author,ISBN,Date Read,Exclusive Shelf\n\
             1,\"Dune (Dune, #1)\",Frank Herbert,\"=\"\"0441013597\"\"\",2024/01/05,read\n\
             2,\"Babel, or the Necessity of Violence\",R.F. Kuang,,,to-read\n\
             3,Skipped,Someone,,,did-not-finish\n",
        )
        .unwrap();
        let entries = read_csv(&path).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(
            (entries[0].title.as_str(), entries[0].shelf),
            ("Dune", Shelf::Read)
        );
        assert_eq!(entries[0].read_at.as_deref(), Some("2024-01-05T00:00:00Z"));
        assert_eq!(entries[1].title, "Babel, or the Necessity of Violence");
    }

    #[test]
    fn imports_an_export_and_remembers_it() {
        let data = tempfile::tempdir().unwrap();
        let state = AppState::open(data.path().to_path_buf(), true).unwrap();
        // A profile link saved before the feeds were retired.
        db::kv_set(&state.db(), OLD_ACCOUNT_KEY, &serde_json::json!({ "userId": "1" })).unwrap();
        let path = data.path().join("goodreads_library_export.csv");
        std::fs::write(
            &path,
            "Book Id,Title,Author,Exclusive Shelf
             1,Hyperion,Dan  Simmons,to-read
             2,Piranesi,Susanna Clarke,read
",
        )
        .unwrap();

        let last = import_export(&state, &path).unwrap();
        assert_eq!((last.want_to_read, last.read, last.applied.added_to_wishlist), (1, 1, 1));
        assert_eq!(last.file_name, "goodreads_library_export.csv");
        let wish = &db::list_wishlist(&state.db()).unwrap()[0];
        assert_eq!(wish.author, "Dan Simmons", "doubled spaces are tidied");

        let status = status(&state).unwrap();
        assert!(status.old_profile_link);
        assert_eq!(status.last_import.unwrap().want_to_read, 1);
        forget_old_profile_link(&state).unwrap();
        assert!(!self::status(&state).unwrap().old_profile_link);

        // A later export without Hyperion on Want to Read takes it off the wishlist.
        std::fs::write(&path, "Book Id,Title,Author,Exclusive Shelf
2,Piranesi,Susanna Clarke,read
").unwrap();
        assert_eq!(import_export(&state, &path).unwrap().applied.removed_from_wishlist, 1);
    }

    #[test]
    fn applies_shelves_without_moving_books_backwards() {
        let data = tempfile::tempdir().unwrap();
        let state = AppState::open(data.path().to_path_buf(), true).unwrap();
        let file = data.path().join("piranesi.epub");
        crate::epub::write_test_epub(&file, "Piranesi", "Susanna Clarke");
        let (id, _) = crate::library::import_into(&state, &file, None, &|_| {}).unwrap();

        let entry = |id: &str, title: &str, author: &str, shelf| Entry {
            book_id: id.into(),
            title: title.into(),
            author: author.into(),
            shelf,
            cover_url: None,
            read_at: None,
        };
        let entries = vec![
            entry("1", "Piranesi", "Susanna Clarke", Shelf::Read),
            entry("2", "Hyperion", "Dan Simmons", Shelf::ToRead),
            entry("3", "Piranesi", "Susanna Clarke", Shelf::ToRead),
        ];
        let applied = apply(&state, &entries, true).unwrap();
        assert_eq!(
            (applied.added_to_wishlist, applied.marked_finished),
            (1, 1),
            "owned books aren't wished for"
        );
        assert_eq!(db::get_book(&state.db(), id).unwrap().status, "finished");

        // Hyperion leaves the to-read shelf, so its Goodreads wish goes away.
        let applied = apply(
            &state,
            &[entry("1", "Piranesi", "Susanna Clarke", Shelf::Reading)],
            true,
        )
        .unwrap();
        assert_eq!(applied.removed_from_wishlist, 1);
        assert_eq!(
            applied.marked_reading, 0,
            "a finished book isn't moved back to reading"
        );
        assert!(db::list_wishlist(&state.db()).unwrap().is_empty());
    }
}

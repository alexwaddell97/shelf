//! The library database: one SQLite file in the app data folder.

use crate::error::{Error, Result};
use crate::model::{Book, BookPatch, CatalogBook, WishItem};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::{Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::path::Path;

const SCHEMA_V1: &str = "
CREATE TABLE books (
    id           INTEGER PRIMARY KEY,
    title        TEXT NOT NULL,
    author       TEXT NOT NULL,
    series       TEXT,
    series_index REAL,
    description  TEXT,
    format       TEXT NOT NULL,
    size_bytes   INTEGER NOT NULL,
    added_at     TEXT NOT NULL,
    cover_path   TEXT,
    file_path    TEXT NOT NULL UNIQUE,
    content_hash TEXT NOT NULL,
    progress     REAL NOT NULL DEFAULT 0,
    status       TEXT NOT NULL DEFAULT 'unread',
    last_read    TEXT
);
CREATE INDEX books_hash ON books(content_hash);

CREATE TABLE devices (
    id        TEXT PRIMARY KEY,
    name      TEXT NOT NULL,
    kind      TEXT NOT NULL,
    last_seen TEXT NOT NULL
);

-- Books currently on a device, with their path relative to the device root.
CREATE TABLE device_books (
    device_id   TEXT NOT NULL,
    book_id     INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    device_path TEXT NOT NULL,
    sent_at     TEXT NOT NULL,
    PRIMARY KEY (device_id, book_id)
);

-- Books waiting to be sent on the next sync.
CREATE TABLE queue (
    device_id TEXT NOT NULL,
    book_id   INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    queued_at TEXT NOT NULL,
    PRIMARY KEY (device_id, book_id)
);

CREATE TABLE wishlist (
    id          INTEGER PRIMARY KEY,
    catalog_key TEXT,
    title       TEXT NOT NULL,
    author      TEXT NOT NULL,
    cover_url   TEXT,
    added_at    TEXT NOT NULL,
    origin      TEXT NOT NULL
);

CREATE TABLE kv (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
";

pub fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn open(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update_and_check(None, "journal_mode", "WAL", |_| Ok(()))?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}

// v2: the rest of the metadata calibre users expect, plus tags and collections.
const SCHEMA_V2: &str = "
ALTER TABLE books ADD COLUMN author_sort TEXT;
ALTER TABLE books ADD COLUMN publisher TEXT;
ALTER TABLE books ADD COLUMN published TEXT;
ALTER TABLE books ADD COLUMN isbn TEXT;
ALTER TABLE books ADD COLUMN language TEXT;
ALTER TABLE books ADD COLUMN rating INTEGER;

-- Tags (descriptive, often imported) and collections (curated by the user).
CREATE TABLE labels (
    id   INTEGER PRIMARY KEY,
    kind TEXT NOT NULL CHECK (kind IN ('tag', 'collection')),
    name TEXT NOT NULL COLLATE NOCASE,
    UNIQUE (kind, name)
);
CREATE TABLE book_labels (
    book_id  INTEGER NOT NULL REFERENCES books(id) ON DELETE CASCADE,
    label_id INTEGER NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
    PRIMARY KEY (book_id, label_id)
);
";

// v3: where you are in each book when reading on this computer.
const SCHEMA_V3: &str = "
CREATE TABLE reading_positions (
    book_id    INTEGER PRIMARY KEY REFERENCES books(id) ON DELETE CASCADE,
    position   TEXT NOT NULL,   -- an EPUB CFI from the reader
    fraction   REAL NOT NULL,   -- 0..1 through the book
    updated_at TEXT NOT NULL
);
";

fn migrate(conn: &Connection) -> Result<()> {
    let version: i32 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
    // Each step runs in one transaction with its version bump, so a crash part-way through
    // can't leave a half-applied schema that fails on every later start.
    let step = |to: i32, apply: &dyn Fn(&Connection) -> Result<()>| -> Result<()> {
        if version < to {
            let tx = conn.unchecked_transaction()?;
            apply(&tx)?;
            tx.pragma_update(None, "user_version", to)?;
            tx.commit()?;
        }
        Ok(())
    };
    step(1, &|c| Ok(c.execute_batch(SCHEMA_V1)?))?;
    step(2, &|c| {
        c.execute_batch(SCHEMA_V2)?;
        let authors: Vec<(i64, String)> = c
            .prepare("SELECT id, author FROM books")?
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        for (id, author) in authors {
            c.execute(
                "UPDATE books SET author_sort = ?2 WHERE id = ?1",
                params![id, author_sort(&author)],
            )?;
        }
        Ok(())
    })?;
    step(3, &|c| Ok(c.execute_batch(SCHEMA_V3)?))?;
    step(4, &|c| {
        // Some files list one author twice ("Cixin Liu" and "Liu, Cixin"); keep one.
        let books: Vec<(i64, String)> = c
            .prepare("SELECT id, author FROM books WHERE author LIKE '% & %'")?
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        for (id, author) in books {
            let cleaned = dedupe_authors(&author);
            if cleaned != author {
                c.execute(
                    "UPDATE books SET author = ?2, author_sort = ?3 WHERE id = ?1",
                    params![id, cleaned, author_sort(&cleaned)],
                )?;
            }
        }
        Ok(())
    })?;
    step(5, &|c| {
        // A series number means nothing without a series (calibre writes 1.0 regardless).
        c.execute(
            "UPDATE books SET series_index = NULL WHERE series IS NULL OR trim(series) = ''",
            [],
        )?;
        // Goodreads sometimes doubles the spaces in names ("Stephen  King").
        let wishes: Vec<(i64, String)> = c
            .prepare("SELECT id, author FROM wishlist WHERE author LIKE '%  %'")?
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        for (id, author) in wishes {
            c.execute(
                "UPDATE wishlist SET author = ?2 WHERE id = ?1",
                params![id, collapse_spaces(&author)],
            )?;
        }
        Ok(())
    })?;
    Ok(())
}

/// "Stephen  King " → "Stephen King".
pub fn collapse_spaces(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The same for "Cixin Liu" and "Liu, Cixin": the name's words, lower-cased and sorted.
pub fn person_key(name: &str) -> String {
    let mut words: Vec<String> = name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    words.sort();
    words.join(" ")
}

/// "Cixin Liu & Liu, Cixin" → "Cixin Liu": drops repeats of the same person.
pub fn dedupe_authors(author: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    author
        .split(" & ")
        .map(str::trim)
        .filter(|name| !name.is_empty() && seen.insert(person_key(name)))
        .collect::<Vec<_>>()
        .join(" & ")
}

/// calibre-style sort name: "Frank Herbert" → "Herbert, Frank";
/// "Daniel Abraham & Ty Franck" → "Abraham, Daniel & Franck, Ty".
pub fn author_sort(author: &str) -> String {
    const SUFFIXES: &[&str] = &["jr", "jr.", "sr", "sr.", "ii", "iii", "iv", "phd"];
    author
        .split(" & ")
        .map(|name| {
            let name = name.trim();
            if name.contains(',') {
                return name.to_string(); // already "Last, First"
            }
            let mut words: Vec<&str> = name.split_whitespace().collect();
            let suffix = match words.last() {
                Some(w) if words.len() > 2 && SUFFIXES.contains(&w.to_lowercase().as_str()) => {
                    words.pop()
                }
                _ => None,
            };
            match words.split_last() {
                Some((last, first)) if !first.is_empty() => {
                    let mut sorted = format!("{last}, {}", first.join(" "));
                    if let Some(suffix) = suffix {
                        sorted.push_str(&format!(" {suffix}"));
                    }
                    sorted
                }
                _ => name.to_string(),
            }
        })
        .collect::<Vec<_>>()
        .join(" & ")
}

// --- Books ---

const BOOK_COLS: &str = "id, title, author, series, series_index, description, format, size_bytes, \
                         added_at, cover_path, file_path, progress, status, last_read, \
                         author_sort, publisher, published, isbn, language, rating";

fn book_from_row(r: &Row) -> rusqlite::Result<Book> {
    Ok(Book {
        id: r.get(0)?,
        title: r.get(1)?,
        author: r.get(2)?,
        series: r.get(3)?,
        series_index: r.get(4)?,
        description: r.get(5)?,
        format: r.get(6)?,
        size_bytes: r.get(7)?,
        added_at: r.get(8)?,
        cover_path: r.get(9)?,
        file_path: r.get(10)?,
        progress: r.get(11)?,
        status: r.get(12)?,
        last_read: r.get(13)?,
        author_sort: r.get(14)?,
        publisher: r.get(15)?,
        published: r.get(16)?,
        isbn: r.get(17)?,
        language: r.get(18)?,
        rating: r.get(19)?,
        tags: Vec::new(),
        collections: Vec::new(),
        on_devices: Vec::new(),
        queued_for: Vec::new(),
    })
}

fn device_map(conn: &Connection, table: &str) -> Result<HashMap<i64, Vec<String>>> {
    let mut stmt = conn.prepare(&format!("SELECT book_id, device_id FROM {table}"))?;
    let mut map: HashMap<i64, Vec<String>> = HashMap::new();
    let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
    for row in rows {
        let (book, device) = row?;
        map.entry(book).or_default().push(device);
    }
    Ok(map)
}

/// book id → (tags, collections)
fn label_map(conn: &Connection) -> Result<HashMap<i64, (Vec<String>, Vec<String>)>> {
    let mut stmt = conn.prepare(
        "SELECT bl.book_id, l.kind, l.name FROM book_labels bl JOIN labels l ON l.id = bl.label_id ORDER BY l.name",
    )?;
    let mut map: HashMap<i64, (Vec<String>, Vec<String>)> = HashMap::new();
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (book, kind, name) = row?;
        let entry = map.entry(book).or_default();
        if kind == "tag" {
            entry.0.push(name)
        } else {
            entry.1.push(name)
        }
    }
    Ok(map)
}

fn attach_devices(conn: &Connection, books: &mut [Book]) -> Result<()> {
    let mut on = device_map(conn, "device_books")?;
    let mut queued = device_map(conn, "queue")?;
    let mut labels = label_map(conn)?;
    for book in books {
        book.on_devices = on.remove(&book.id).unwrap_or_default();
        book.queued_for = queued.remove(&book.id).unwrap_or_default();
        let (tags, collections) = labels.remove(&book.id).unwrap_or_default();
        book.tags = tags;
        book.collections = collections;
    }
    Ok(())
}

pub fn list_books(conn: &Connection) -> Result<Vec<Book>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {BOOK_COLS} FROM books ORDER BY added_at DESC, id DESC"
    ))?;
    let mut books = stmt
        .query_map([], book_from_row)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    attach_devices(conn, &mut books)?;
    Ok(books)
}

pub fn get_book(conn: &Connection, id: i64) -> Result<Book> {
    let mut book = conn
        .query_row(
            &format!("SELECT {BOOK_COLS} FROM books WHERE id = ?1"),
            [id],
            book_from_row,
        )
        .optional()?
        .ok_or_else(|| Error::msg("That book isn't in your library any more."))?;
    attach_devices(conn, std::slice::from_mut(&mut book))?;
    Ok(book)
}

pub fn book_by_hash(conn: &Connection, hash: &str) -> Result<Option<i64>> {
    Ok(conn
        .query_row(
            "SELECT id FROM books WHERE content_hash = ?1",
            [hash],
            |r| r.get(0),
        )
        .optional()?)
}

/// Everything about a book that isn't its file.
#[derive(Debug, Clone, Default)]
pub struct BookMeta {
    pub title: String,
    pub author: String,
    pub author_sort: Option<String>,
    pub series: Option<String>,
    pub series_index: Option<f64>,
    pub description: Option<String>,
    pub publisher: Option<String>,
    /// "YYYY-MM-DD" or "YYYY".
    pub published: Option<String>,
    pub isbn: Option<String>,
    pub language: Option<String>,
    /// 1–5 stars.
    pub rating: Option<u8>,
    pub tags: Vec<String>,
    pub collections: Vec<String>,
}

pub struct NewBook<'a> {
    pub meta: &'a BookMeta,
    pub format: &'a str,
    pub size_bytes: i64,
    pub file_path: &'a str,
    pub content_hash: &'a str,
}

pub fn insert_book(conn: &Connection, b: &NewBook) -> Result<i64> {
    let m = b.meta;
    let sort = m
        .author_sort
        .clone()
        .unwrap_or_else(|| author_sort(&m.author));
    conn.execute(
        "INSERT INTO books (title, author, series, series_index, description, format, size_bytes,
                            added_at, file_path, content_hash, author_sort, publisher, published,
                            isbn, language, rating)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            m.title,
            m.author,
            m.series,
            // A series number means nothing without a series.
            m.series.as_ref().and(m.series_index),
            m.description,
            b.format,
            b.size_bytes,
            now(),
            b.file_path,
            b.content_hash,
            sort,
            m.publisher,
            m.published,
            m.isbn,
            m.language,
            m.rating
        ],
    )?;
    let id = conn.last_insert_rowid();
    set_labels(conn, id, "tag", &m.tags)?;
    set_labels(conn, id, "collection", &m.collections)?;
    Ok(id)
}

// --- Tags and collections ---

/// Replaces a book's labels of one kind, creating labels as needed.
pub fn set_labels(conn: &Connection, book_id: i64, kind: &str, names: &[String]) -> Result<()> {
    conn.execute(
        "DELETE FROM book_labels WHERE book_id = ?1 AND label_id IN (SELECT id FROM labels WHERE kind = ?2)",
        params![book_id, kind],
    )?;
    for name in names.iter().map(|n| n.trim()).filter(|n| !n.is_empty()) {
        conn.execute(
            "INSERT OR IGNORE INTO labels (kind, name) VALUES (?1, ?2)",
            params![kind, name],
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO book_labels (book_id, label_id)
             SELECT ?1, id FROM labels WHERE kind = ?2 AND name = ?3",
            params![book_id, kind, name],
        )?;
    }
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    pub id: i64,
    pub kind: String,
    pub name: String,
    pub count: i64,
}

pub fn list_labels(conn: &Connection, kind: &str) -> Result<Vec<Label>> {
    let mut stmt = conn.prepare(
        "SELECT l.id, l.kind, l.name, COUNT(bl.book_id) FROM labels l
         LEFT JOIN book_labels bl ON bl.label_id = l.id
         WHERE l.kind = ?1 GROUP BY l.id ORDER BY l.name",
    )?;
    let rows = stmt.query_map([kind], |r| {
        Ok(Label {
            id: r.get(0)?,
            kind: r.get(1)?,
            name: r.get(2)?,
            count: r.get(3)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn create_label(conn: &Connection, kind: &str, name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::msg("Give it a name."));
    }
    conn.execute(
        "INSERT OR IGNORE INTO labels (kind, name) VALUES (?1, ?2)",
        params![kind, name],
    )?;
    Ok(())
}

pub fn rename_label(conn: &Connection, id: i64, name: &str) -> Result<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::msg("Give it a name."));
    }
    conn.execute(
        "UPDATE labels SET name = ?2 WHERE id = ?1",
        params![id, name],
    )
    .map_err(|_| Error::msg("There's already one with that name."))?;
    Ok(())
}

pub fn delete_label(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM labels WHERE id = ?1", [id])?;
    Ok(())
}

/// Adds books to (or removes them from) a label.
pub fn label_books(conn: &Connection, label_id: i64, book_ids: &[i64], add: bool) -> Result<()> {
    for book_id in book_ids {
        if add {
            conn.execute(
                "INSERT OR IGNORE INTO book_labels (book_id, label_id) VALUES (?1, ?2)",
                params![book_id, label_id],
            )?;
        } else {
            conn.execute(
                "DELETE FROM book_labels WHERE book_id = ?1 AND label_id = ?2",
                params![book_id, label_id],
            )?;
        }
    }
    Ok(())
}

pub fn set_cover(conn: &Connection, id: i64, path: &str) -> Result<()> {
    conn.execute(
        "UPDATE books SET cover_path = ?2 WHERE id = ?1",
        params![id, path],
    )?;
    Ok(())
}

/// Applies an edit. Returns true when something an e-reader shows (title, author,
/// series, description) changed, so copies on devices should be refreshed.
pub fn update_book(conn: &Connection, id: i64, patch: &BookPatch) -> Result<bool> {
    let current = get_book(conn, id)?;
    let text = |value: &Option<String>, fallback: &str| -> String {
        value
            .as_deref()
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .unwrap_or(fallback)
            .to_string()
    };
    let clean = |value: Option<String>| {
        value
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };
    let title = text(&patch.title, &current.title);
    let author = text(&patch.author, &current.author);
    let sort = match &patch.author_sort {
        Some(Some(sort)) if !sort.trim().is_empty() => sort.trim().to_string(),
        // Author changed without an explicit sort name: recompute it.
        _ if author != current.author => author_sort(&author),
        Some(_) => author_sort(&author),
        None => current
            .author_sort
            .clone()
            .unwrap_or_else(|| author_sort(&author)),
    };
    let series = patch
        .series
        .clone()
        .map(clean)
        .unwrap_or(current.series.clone());
    let series_index = if series.is_some() {
        patch.series_index.unwrap_or(current.series_index)
    } else {
        None
    };
    let description = patch
        .description
        .clone()
        .map(clean)
        .unwrap_or(current.description.clone());
    let publisher = patch
        .publisher
        .clone()
        .map(clean)
        .unwrap_or(current.publisher.clone());
    let published = patch
        .published
        .clone()
        .map(clean)
        .unwrap_or(current.published.clone());
    let isbn = patch
        .isbn
        .clone()
        .map(clean)
        .unwrap_or(current.isbn.clone());
    let language = patch
        .language
        .clone()
        .map(clean)
        .unwrap_or(current.language.clone());
    let rating = patch
        .rating
        .unwrap_or(current.rating)
        .filter(|r| (1..=5).contains(r));
    conn.execute(
        "UPDATE books SET title = ?2, author = ?3, author_sort = ?4, series = ?5, series_index = ?6,
                          description = ?7, publisher = ?8, published = ?9, isbn = ?10, language = ?11,
                          rating = ?12
         WHERE id = ?1",
        params![id, title, author, sort, series, series_index, description, publisher, published, isbn, language, rating],
    )?;
    if let Some(tags) = &patch.tags {
        set_labels(conn, id, "tag", tags)?;
    }
    if let Some(collections) = &patch.collections {
        set_labels(conn, id, "collection", collections)?;
    }
    Ok(title != current.title
        || author != current.author
        || series != current.series
        || series_index != current.series_index
        || description != current.description)
}

/// Queues a book again for every device it's on, so the copy there gets refreshed.
pub fn requeue_on_devices(conn: &Connection, book_id: i64) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO queue (device_id, book_id, queued_at)
         SELECT device_id, book_id, ?2 FROM device_books WHERE book_id = ?1",
        params![book_id, now()],
    )?;
    Ok(())
}

pub fn delete_book(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM books WHERE id = ?1", [id])?;
    Ok(())
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingPosition {
    pub position: String,
    pub fraction: f64,
}

pub fn reading_position(conn: &Connection, book_id: i64) -> Result<Option<ReadingPosition>> {
    Ok(conn
        .query_row(
            "SELECT position, fraction FROM reading_positions WHERE book_id = ?1",
            [book_id],
            |r| {
                Ok(ReadingPosition {
                    position: r.get(0)?,
                    fraction: r.get(1)?,
                })
            },
        )
        .optional()?)
}

/// Saves where you are in a book read on this computer, and updates its progress and
/// status in the library to match (reaching the last page marks it finished).
pub fn save_reading_position(
    conn: &Connection,
    book_id: i64,
    position: &str,
    fraction: f64,
) -> Result<()> {
    let fraction = fraction.clamp(0.0, 1.0);
    let at = now();
    conn.execute(
        "INSERT INTO reading_positions (book_id, position, fraction, updated_at) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(book_id) DO UPDATE SET position = excluded.position, fraction = excluded.fraction,
                                            updated_at = excluded.updated_at",
        params![book_id, position, fraction, at],
    )?;
    let (progress, status) = if fraction >= 0.995 {
        (100.0, "finished")
    } else {
        ((fraction * 100.0).max(0.1), "reading")
    };
    set_progress(conn, book_id, progress, status, Some(&at))
}

pub fn set_progress(
    conn: &Connection,
    id: i64,
    progress: f64,
    status: &str,
    last_read: Option<&str>,
) -> Result<()> {
    conn.execute(
        "UPDATE books SET progress = ?2, status = ?3, last_read = COALESCE(?4, last_read) WHERE id = ?1",
        params![id, progress, status, last_read],
    )?;
    Ok(())
}

// --- Devices ---

pub struct KnownDevice {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub last_seen: String,
}

pub fn upsert_device(conn: &Connection, id: &str, name: &str, kind: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO devices (id, name, kind, last_seen) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET name = excluded.name, last_seen = excluded.last_seen",
        params![id, name, kind, now()],
    )?;
    Ok(())
}

pub fn known_devices(conn: &Connection) -> Result<Vec<KnownDevice>> {
    let mut stmt =
        conn.prepare("SELECT id, name, kind, last_seen FROM devices ORDER BY last_seen DESC")?;
    let rows = stmt.query_map([], |r| {
        Ok(KnownDevice {
            id: r.get(0)?,
            name: r.get(1)?,
            kind: r.get(2)?,
            last_seen: r.get(3)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn forget_device(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM queue WHERE device_id = ?1", [id])?;
    conn.execute("DELETE FROM device_books WHERE device_id = ?1", [id])?;
    conn.execute("DELETE FROM devices WHERE id = ?1", [id])?;
    Ok(())
}

pub fn queue_add(conn: &Connection, device_id: &str, book_id: i64) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO queue (device_id, book_id, queued_at)
         SELECT ?1, ?2, ?3 WHERE NOT EXISTS
           (SELECT 1 FROM device_books WHERE device_id = ?1 AND book_id = ?2)",
        params![device_id, book_id, now()],
    )?;
    Ok(())
}

/// Queues a new book for every device Shelf knows about.
pub fn queue_for_all_devices(conn: &Connection, book_id: i64) -> Result<()> {
    for device in known_devices(conn)? {
        queue_add(conn, &device.id, book_id)?;
    }
    Ok(())
}

pub fn queue_remove(conn: &Connection, device_id: &str, book_id: i64) -> Result<()> {
    conn.execute(
        "DELETE FROM queue WHERE device_id = ?1 AND book_id = ?2",
        params![device_id, book_id],
    )?;
    Ok(())
}

pub fn queued_books(conn: &Connection, device_id: &str) -> Result<Vec<i64>> {
    let mut stmt =
        conn.prepare("SELECT book_id FROM queue WHERE device_id = ?1 ORDER BY queued_at")?;
    let rows = stmt.query_map([device_id], |r| r.get(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn device_book_add(
    conn: &Connection,
    device_id: &str,
    book_id: i64,
    device_path: &str,
) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO device_books (device_id, book_id, device_path, sent_at) VALUES (?1, ?2, ?3, ?4)",
        params![device_id, book_id, device_path, now()],
    )?;
    Ok(())
}

pub fn device_book_remove(conn: &Connection, device_id: &str, book_id: i64) -> Result<()> {
    conn.execute(
        "DELETE FROM device_books WHERE device_id = ?1 AND book_id = ?2",
        params![device_id, book_id],
    )?;
    Ok(())
}

pub fn device_book_path(
    conn: &Connection,
    device_id: &str,
    book_id: i64,
) -> Result<Option<String>> {
    Ok(conn
        .query_row(
            "SELECT device_path FROM device_books WHERE device_id = ?1 AND book_id = ?2",
            params![device_id, book_id],
            |r| r.get(0),
        )
        .optional()?)
}

pub fn device_books(conn: &Connection, device_id: &str) -> Result<Vec<(i64, String)>> {
    let mut stmt =
        conn.prepare("SELECT book_id, device_path FROM device_books WHERE device_id = ?1")?;
    let rows = stmt.query_map([device_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

// --- Wishlist ---

fn wish_from_row(r: &Row) -> rusqlite::Result<WishItem> {
    Ok(WishItem {
        id: r.get(0)?,
        catalog_key: r.get(1)?,
        title: r.get(2)?,
        author: r.get(3)?,
        cover_url: r.get(4)?,
        added_at: r.get(5)?,
        origin: r.get(6)?,
    })
}

pub fn list_wishlist(conn: &Connection) -> Result<Vec<WishItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, catalog_key, title, author, cover_url, added_at, origin FROM wishlist ORDER BY added_at DESC",
    )?;
    let rows = stmt.query_map([], wish_from_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

pub fn add_wish(conn: &Connection, book: &CatalogBook, origin: &str) -> Result<WishItem> {
    if let Some(existing) = conn
        .query_row(
            "SELECT id, catalog_key, title, author, cover_url, added_at, origin FROM wishlist WHERE catalog_key = ?1",
            [&book.key],
            wish_from_row,
        )
        .optional()?
    {
        return Ok(existing);
    }
    conn.execute(
        "INSERT INTO wishlist (catalog_key, title, author, cover_url, added_at, origin) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![book.key, book.title, book.author, book.cover_url, now(), origin],
    )?;
    let id = conn.last_insert_rowid();
    Ok(conn.query_row(
        "SELECT id, catalog_key, title, author, cover_url, added_at, origin FROM wishlist WHERE id = ?1",
        [id],
        wish_from_row,
    )?)
}

pub fn remove_wish(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM wishlist WHERE id = ?1", [id])?;
    Ok(())
}

/// Lower-cases and strips everything but letters and digits ("&" counts as "and").
pub fn normalize(text: &str) -> String {
    text.replace('&', "and")
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Drops a leading "The", "A" or "An", which catalogues often disagree on.
fn without_article(title: &str) -> &str {
    let trimmed = title.trim_start();
    for article in ["the ", "an ", "a "] {
        if trimmed.len() > article.len()
            && trimmed.is_char_boundary(article.len())
            && trimmed[..article.len()].eq_ignore_ascii_case(article)
        {
            return &trimmed[article.len()..];
        }
    }
    trimmed
}

/// The title without its subtitle: "The Hobbit, or There and Back Again" → "hobbit".
fn main_title(title: &str) -> String {
    let lower = without_article(title).to_lowercase();
    let cut = [":", "(", ";", ", or ", " - "]
        .iter()
        .filter_map(|sep| lower.find(sep))
        .min()
        .unwrap_or(lower.len());
    normalize(&lower[..cut])
}

fn last_name(author: &str) -> String {
    let first_author = author.split(['&', ',', ';']).next().unwrap_or(author);
    normalize(
        first_author
            .split_whitespace()
            .last()
            .unwrap_or(first_author),
    )
}

/// A title's words, lower-cased, without punctuation or anything from the first "(" on
/// (usually the series: "(Mistborn, #1)").
fn title_words(title: &str) -> Vec<String> {
    let cut = &title[..title.find('(').unwrap_or(title.len())];
    cut.replace('&', " and ")
        .replace(['\'', '\u{2019}'], "")
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

/// True when one title is the end of the other, as when a franchise or series name comes
/// first: "Star Wars Inquisitor: Rise of the Red Blade" and "Inquisitor: Rise of the Red
/// Blade". The shorter title needs three words or more (counting a leading "The"), so
/// "Thrawn" alone doesn't match every "Something: Thrawn".
fn ends_with_title(a: &str, b: &str) -> bool {
    let (wa, wb) = (title_words(a), title_words(b));
    let (short, long) = if wa.len() <= wb.len() {
        (wa, wb)
    } else {
        (wb, wa)
    };
    if short.len() < 3 {
        return false;
    }
    let tail = match short[0].as_str() {
        "the" | "a" | "an" => &short[1..],
        _ => &short[..],
    };
    tail.len() < long.len() && long.ends_with(tail)
}

/// True when a library book and a wished-for book are probably the same work.
pub fn same_work(title_a: &str, author_a: &str, title_b: &str, author_b: &str) -> bool {
    let (ta, tb) = (
        normalize(without_article(title_a)),
        normalize(without_article(title_b)),
    );
    if ta.is_empty() || tb.is_empty() {
        return false;
    }
    let titles_match =
        ta == tb || main_title(title_a) == main_title(title_b) || ends_with_title(title_a, title_b);
    let (la, lb) = (last_name(author_a), last_name(author_b));
    titles_match && (la.is_empty() || lb.is_empty() || la == lb)
}

/// Removes and returns the wishlist item matching a newly imported book, if any.
pub fn take_matching_wish(conn: &Connection, title: &str, author: &str) -> Result<Option<i64>> {
    for wish in list_wishlist(conn)? {
        if same_work(title, author, &wish.title, &wish.author) {
            remove_wish(conn, wish.id)?;
            return Ok(Some(wish.id));
        }
    }
    Ok(None)
}

// --- Key/value settings ---

pub fn kv_get<T: DeserializeOwned>(conn: &Connection, key: &str) -> Result<Option<T>> {
    let raw: Option<String> = conn
        .query_row("SELECT value FROM kv WHERE key = ?1", [key], |r| r.get(0))
        .optional()?;
    Ok(match raw {
        Some(text) => Some(serde_json::from_str(&text)?),
        None => None,
    })
}

pub fn kv_set<T: Serialize>(conn: &Connection, key: &str, value: &T) -> Result<()> {
    conn.execute(
        "INSERT INTO kv (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, serde_json::to_string(value)?],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_tidy_old_data() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA_V1).unwrap();
        conn.execute_batch(SCHEMA_V2).unwrap();
        conn.execute_batch(SCHEMA_V3).unwrap();
        conn.pragma_update(None, "user_version", 4).unwrap();
        conn.execute(
            "INSERT INTO books (title, author, series, series_index, format, size_bytes, added_at,
                                file_path, content_hash)
             VALUES ('The Stand', 'Stephen King', NULL, 1.0, 'epub', 1, 'now', 'a.epub', 'h')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO wishlist (catalog_key, title, author, added_at, origin)
             VALUES ('k', 'Swan Song', 'Robert  McCammon', 'now', 'goodreads')",
            [],
        )
        .unwrap();
        migrate(&conn).unwrap();
        let index: Option<f64> = conn
            .query_row("SELECT series_index FROM books", [], |r| r.get(0))
            .unwrap();
        assert_eq!(index, None);
        let author: String = conn
            .query_row("SELECT author FROM wishlist", [], |r| r.get(0))
            .unwrap();
        assert_eq!(author, "Robert McCammon");
        let version: i32 = conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, 5);
    }

    #[test]
    fn drops_repeated_authors() {
        assert_eq!(dedupe_authors("Cixin Liu & Liu, Cixin"), "Cixin Liu");
        assert_eq!(
            dedupe_authors("Daniel Abraham & Ty Franck"),
            "Daniel Abraham & Ty Franck"
        );
        assert_eq!(dedupe_authors("Terry Pratchett"), "Terry Pratchett");
    }

    #[test]
    fn matches_titles_with_subtitles_and_punctuation() {
        assert!(same_work(
            "The Hobbit, or There and Back Again",
            "J. R. R. Tolkien",
            "The Hobbit",
            "Tolkien"
        ));
        assert!(same_work(
            "Pride and Prejudice",
            "Jane Austen",
            "Pride & Prejudice",
            "Austen, Jane"
        ));
        assert!(same_work(
            "Piranesi",
            "Susanna Clarke",
            "PIRANESI",
            "Clarke"
        ));
        assert!(!same_work(
            "Dune",
            "Frank Herbert",
            "Dune Messiah",
            "Frank Herbert"
        ));
        assert!(!same_work(
            "Emma",
            "Jane Austen",
            "Persuasion",
            "Jane Austen"
        ));
        assert!(!same_work(
            "Circe",
            "Madeline Miller",
            "Circe",
            "Someone Else"
        ));
        assert!(same_work(
            "Sword of Kaigen",
            "M. L. Wang",
            "The Sword of Kaigen: A Theonite War Story",
            "M.L. Wang"
        ));
        assert!(same_work(
            "A Memory Called Empire",
            "Arkady Martine",
            "Memory Called Empire",
            "Martine"
        ));
        assert!(!same_work(
            "The Way of Kings",
            "Brandon Sanderson",
            "Words of Radiance",
            "Brandon Sanderson"
        ));
        // A franchise or series name in front of the title.
        assert!(same_work(
            "Star Wars Inquisitor: Rise of the Red Blade",
            "Delilah S. Dawson",
            "Inquisitor: Rise of the Red Blade (Star Wars)",
            "Delilah S. Dawson"
        ));
        assert!(same_work(
            "Star Wars: Inquisitor: Rise of the Red Blade",
            "Delilah S. Dawson",
            "Inquisitor: Rise of the Red Blade",
            "Delilah S. Dawson"
        ));
        assert!(same_work(
            "Inquisitor - Rise of the Red Blade (Star Wars)",
            "Delilah Dawson",
            "Star Wars: Inquisitor: Rise of the Red Blade",
            ""
        ));
        assert!(same_work(
            "Mistborn: The Final Empire",
            "Brandon Sanderson",
            "The Final Empire (Mistborn, #1)",
            "Sanderson"
        ));
        // A short, generic ending isn't enough on its own.
        assert!(!same_work(
            "Circe: A Novel",
            "Madeline Miller",
            "Galatea: A Novel",
            "Madeline Miller"
        ));
        assert!(!same_work(
            "Star Wars: Thrawn",
            "Timothy Zahn",
            "Thrawn",
            "Timothy Zahn"
        ));
    }

    #[test]
    fn saves_reading_positions() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        let meta = BookMeta {
            title: "Dune".into(),
            author: "Frank Herbert".into(),
            ..Default::default()
        };
        let id = insert_book(
            &conn,
            &NewBook {
                meta: &meta,
                format: "epub",
                size_bytes: 1,
                file_path: "/d.epub",
                content_hash: "h",
            },
        )
        .unwrap();
        assert!(reading_position(&conn, id).unwrap().is_none());
        save_reading_position(&conn, id, "epubcfi(/6/4!/4/2)", 0.42).unwrap();
        let pos = reading_position(&conn, id).unwrap().unwrap();
        assert_eq!(
            (pos.position.as_str(), pos.fraction),
            ("epubcfi(/6/4!/4/2)", 0.42)
        );
        let book = get_book(&conn, id).unwrap();
        assert_eq!((book.progress, book.status.as_str()), (42.0, "reading"));
        assert!(book.last_read.is_some());
        save_reading_position(&conn, id, "epubcfi(/6/90!/4/2)", 1.0).unwrap();
        assert_eq!(get_book(&conn, id).unwrap().status, "finished");
        delete_book(&conn, id).unwrap();
        assert!(
            reading_position(&conn, id).unwrap().is_none(),
            "removed with the book"
        );
    }

    #[test]
    fn sorts_author_names_like_calibre() {
        assert_eq!(author_sort("Frank Herbert"), "Herbert, Frank");
        assert_eq!(
            author_sort("Daniel Abraham & Ty Franck"),
            "Abraham, Daniel & Franck, Ty"
        );
        assert_eq!(
            author_sort("Martin Luther King Jr."),
            "King, Martin Luther Jr."
        );
        assert_eq!(author_sort("Homer"), "Homer");
        assert_eq!(author_sort("Austen, Jane"), "Austen, Jane");
    }

    #[test]
    fn edits_metadata_and_labels() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        let meta = BookMeta {
            title: "Dune".into(),
            author: "Frank Herbert".into(),
            ..Default::default()
        };
        let id = insert_book(
            &conn,
            &NewBook {
                meta: &meta,
                format: "epub",
                size_bytes: 1,
                file_path: "/d.epub",
                content_hash: "h",
            },
        )
        .unwrap();
        upsert_device(&conn, "kobo-1", "Kobo", "kobo").unwrap();
        device_book_add(&conn, "kobo-1", id, "Shelf/Dune.kepub.epub").unwrap();

        let patch: BookPatch = serde_json::from_str(
            r#"{"series":"Dune","seriesIndex":1,"rating":4,"collections":["Re-reads"],"tags":["Classic SF"]}"#,
        )
        .unwrap();
        assert!(
            update_book(&conn, id, &patch).unwrap(),
            "series shows on the device"
        );
        let patch: BookPatch = serde_json::from_str(r#"{"rating":5}"#).unwrap();
        assert!(!update_book(&conn, id, &patch).unwrap(), "a rating doesn't");
        let book = get_book(&conn, id).unwrap();
        assert_eq!(
            (book.series.as_deref(), book.series_index, book.rating),
            (Some("Dune"), Some(1.0), Some(5))
        );
        assert_eq!(
            (book.tags.clone(), book.collections.clone()),
            (vec!["Classic SF".to_string()], vec!["Re-reads".to_string()])
        );

        let patch: BookPatch = serde_json::from_str(r#"{"author":"Brian Herbert"}"#).unwrap();
        update_book(&conn, id, &patch).unwrap();
        assert_eq!(
            get_book(&conn, id).unwrap().author_sort.as_deref(),
            Some("Herbert, Brian")
        );

        requeue_on_devices(&conn, id).unwrap();
        assert_eq!(
            get_book(&conn, id).unwrap().queued_for,
            vec!["kobo-1".to_string()]
        );
        let labels = list_labels(&conn, "collection").unwrap();
        assert_eq!((labels[0].name.as_str(), labels[0].count), ("Re-reads", 1));
    }

    #[test]
    fn migrates_and_round_trips_a_book() {
        let conn = Connection::open_in_memory().unwrap();
        migrate(&conn).unwrap();
        let id = insert_book(
            &conn,
            &NewBook {
                meta: &BookMeta {
                    title: "Dune".into(),
                    author: "Frank Herbert".into(),
                    series: Some("Dune".into()),
                    series_index: Some(1.0),
                    collections: vec!["Favourites".into()],
                    ..Default::default()
                },
                format: "epub",
                size_bytes: 10,
                file_path: "/tmp/dune.epub",
                content_hash: "abc",
            },
        )
        .unwrap();
        upsert_device(&conn, "kobo-1", "Kobo", "kobo").unwrap();
        queue_add(&conn, "kobo-1", id).unwrap();
        let book = get_book(&conn, id).unwrap();
        assert_eq!(book.queued_for, vec!["kobo-1".to_string()]);
        assert_eq!(book.author_sort.as_deref(), Some("Herbert, Frank"));
        assert_eq!(book.collections, vec!["Favourites"]);
        device_book_add(&conn, "kobo-1", id, "Shelf/Dune.kepub.epub").unwrap();
        queue_remove(&conn, "kobo-1", id).unwrap();
        let book = get_book(&conn, id).unwrap();
        assert_eq!(book.on_devices, vec!["kobo-1".to_string()]);
        assert!(book.queued_for.is_empty());
    }
}

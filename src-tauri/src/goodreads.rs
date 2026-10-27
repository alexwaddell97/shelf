//! One-way sync from Goodreads. Goodreads closed its API to new apps in 2020, so this
//! reads the public RSS feed Goodreads publishes for each shelf, or a CSV export.
//! Nothing is ever sent back to Goodreads.
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
use roxmltree::Document;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shelf {
    ToRead,
    Reading,
    Read,
}

impl Shelf {
    fn feed_name(self) -> &'static str {
        match self {
            Shelf::ToRead => "to-read",
            Shelf::Reading => "currently-reading",
            Shelf::Read => "read",
        }
    }

    fn from_name(name: &str) -> Option<Shelf> {
        [Shelf::ToRead, Shelf::Reading, Shelf::Read]
            .into_iter()
            .find(|s| s.feed_name() == name)
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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub user_id: String,
    pub name: Option<String>,
    pub last_checked: Option<String>,
    pub want_to_read: usize,
    pub reading: usize,
    pub read: usize,
    /// Wishlist items added by the last check.
    pub last_added: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Applied {
    pub added_to_wishlist: usize,
    pub removed_from_wishlist: usize,
    pub marked_reading: usize,
    pub marked_finished: usize,
}

/// Pulls the numeric user id out of a profile link like
/// `https://www.goodreads.com/user/show/12345-alex`, or accepts a bare id.
pub fn parse_user_id(input: &str) -> Option<String> {
    let input = input.trim();
    if !input.is_empty() && input.chars().all(|c| c.is_ascii_digit()) {
        return Some(input.to_string());
    }
    for marker in [
        "/user/show/",
        "/review/list/",
        "/review/list_rss/",
        "/user/",
    ] {
        if let Some(pos) = input.find(marker) {
            let digits: String = input[pos + marker.len()..]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if !digits.is_empty() {
                return Some(digits);
            }
        }
    }
    None
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

fn child_text(node: roxmltree::Node, name: &str) -> Option<String> {
    node.children()
        .find(|n| n.has_tag_name(name))
        .and_then(|n| n.text())
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

/// Parses one page of a shelf feed: (owner's name, entries).
fn parse_feed(xml: &str, shelf: Shelf) -> Result<(Option<String>, Vec<Entry>)> {
    let doc = Document::parse(xml)?;
    let channel = doc.descendants().find(|n| n.has_tag_name("channel"));
    // The channel title reads "Alex's bookshelf: to-read".
    let name = channel
        .and_then(|c| child_text(c, "title"))
        .and_then(|t| t.split("'s bookshelf").next().map(str::to_string))
        .filter(|n| !n.contains("bookshelf"));
    let entries = doc
        .descendants()
        .filter(|n| n.has_tag_name("item"))
        .filter_map(|item| {
            Some(Entry {
                book_id: child_text(item, "book_id")?,
                title: clean_title(&child_text(item, "title")?),
                author: child_text(item, "author_name")
                    .map(|a| db::collapse_spaces(&a))
                    .unwrap_or_else(|| "Unknown author".into()),
                shelf,
                cover_url: child_text(item, "book_large_image_url")
                    .or_else(|| child_text(item, "book_image_url")),
                read_at: child_text(item, "user_read_at")
                    .and_then(|d| chrono::DateTime::parse_from_rfc2822(&d).ok())
                    .map(|d| {
                        d.to_utc()
                            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
                    }),
            })
        })
        .collect();
    Ok((name, entries))
}

async fn fetch_shelf(
    http: &reqwest::Client,
    user_id: &str,
    shelf: Shelf,
) -> Result<(Option<String>, Vec<Entry>)> {
    let mut name = None;
    let mut all: Vec<Entry> = Vec::new();
    let mut seen = HashSet::new();
    for page in 1..=30 {
        let url = format!(
            "https://www.goodreads.com/review/list_rss/{user_id}?shelf={}&page={page}",
            shelf.feed_name()
        );
        let response = http.get(url).send().await?;
        if response.status().is_client_error() {
            return Err(Error::msg(
                "Goodreads didn't share those shelves. Check the profile link, and that the profile is public.",
            ));
        }
        let xml = response.error_for_status()?.text().await?;
        let (page_name, entries) = parse_feed(&xml, shelf)?;
        name = name.or(page_name);
        let fresh: Vec<Entry> = entries
            .into_iter()
            .filter(|e| seen.insert(e.book_id.clone()))
            .collect();
        if fresh.is_empty() {
            break;
        }
        all.extend(fresh);
    }
    Ok((name, all))
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

pub fn account(state: &AppState) -> Result<Option<Account>> {
    db::kv_get(&state.db(), "goodreads")
}

/// Fetches all three shelves for a user and applies them. Saves the account.
pub async fn check(state: &AppState, user_id: &str) -> Result<Account> {
    let (want, reading, read) = tokio::join!(
        fetch_shelf(&state.http, user_id, Shelf::ToRead),
        fetch_shelf(&state.http, user_id, Shelf::Reading),
        fetch_shelf(&state.http, user_id, Shelf::Read),
    );
    let (name, want) = want?;
    let (_, reading) = reading?;
    let (_, read) = read?;
    let mut entries = want.clone();
    entries.extend(reading.iter().cloned());
    entries.extend(read.iter().cloned());
    let applied = apply(state, &entries, true)?;
    let account = Account {
        user_id: user_id.to_string(),
        name,
        last_checked: Some(db::now()),
        want_to_read: want.len(),
        reading: reading.len(),
        read: read.len(),
        last_added: applied.added_to_wishlist,
    };
    db::kv_set(&state.db(), "goodreads", &account)?;
    Ok(account)
}

pub fn disconnect(state: &AppState) -> Result<()> {
    state
        .db()
        .execute("DELETE FROM kv WHERE key = 'goodreads'", [])?;
    Ok(())
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
                author: get(author).unwrap_or_else(|| "Unknown author".into()),
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

/// Re-checks the connected Goodreads account every hour.
pub fn start_hourly(app: AppHandle) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(60 * 60));
            let state = app.state::<AppState>();
            if let Ok(Some(account)) = account(&state) {
                if tauri::async_runtime::block_on(check(&state, &account.user_id)).is_ok() {
                    let _ = app.emit("library-changed", ());
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const FEED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0"><channel>
  <title>Alex's bookshelf: read</title>
  <item>
    <title>Leviathan Wakes (The Expanse, #1)</title>
    <book_id>8855321</book_id>
    <book_large_image_url>https://images.example/lw.jpg</book_large_image_url>
    <author_name>James S.A. Corey</author_name>
    <user_read_at><![CDATA[Sat, 12 Aug 2023 00:00:00 -0700]]></user_read_at>
  </item>
  <item>
    <title>Piranesi</title>
    <book_id>50202953</book_id>
    <author_name>Susanna Clarke</author_name>
    <user_read_at></user_read_at>
  </item>
</channel></rss>"#;

    #[test]
    fn parses_profile_links() {
        assert_eq!(
            parse_user_id("https://www.goodreads.com/user/show/12345-alex").as_deref(),
            Some("12345")
        );
        assert_eq!(
            parse_user_id("goodreads.com/review/list/987?shelf=read").as_deref(),
            Some("987")
        );
        assert_eq!(parse_user_id(" 4242 ").as_deref(), Some("4242"));
        assert_eq!(parse_user_id("alex"), None);
    }

    #[test]
    fn parses_a_shelf_feed() {
        let (name, entries) = parse_feed(FEED, Shelf::Read).unwrap();
        assert_eq!(name.as_deref(), Some("Alex"));
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].title, "Leviathan Wakes");
        assert_eq!(entries[0].read_at.as_deref(), Some("2023-08-12T07:00:00Z"));
        assert_eq!(entries[1].read_at, None);
    }

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

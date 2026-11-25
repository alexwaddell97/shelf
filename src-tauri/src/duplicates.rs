//! Finding the same book imported more than once, and merging the copies into one.
//!
//! Two books are duplicates when they share an ISBN, or when they have the same whole title
//! and author (`same_book`: accents, punctuation and "The" don't matter, but a different
//! subtitle does). Pairs you mark "not duplicates" are remembered and never offered again.

use crate::db::{self, same_book};
use crate::error::{Error, Result};
use crate::model::Book;
use crate::state::AppState;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::{BTreeSet, HashMap};

const IGNORED_KEY: &str = "duplicates_ignored";

/// A set of books that look like the same work, best candidate to keep first.
pub type Group = Vec<Book>;

fn isbn_key(isbn: &Option<String>) -> Option<String> {
    let digits: String = isbn.as_deref()?.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    (digits.len() == 10 || digits.len() == 13).then_some(digits)
}

/// First author's surname, for bucketing: only books that share one are compared in full.
fn surname(book: &Book) -> String {
    let sort = book.author_sort.as_deref().unwrap_or("");
    let name = match sort.split_once(',') {
        Some((last, _)) => last.to_string(),
        None => book.author.split('&').next().unwrap_or("").split_whitespace().last().unwrap_or("").to_string(),
    };
    name.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()
}

/// Higher is a better copy to keep: on a device, finished or further read, a cover, a
/// Kobo-friendly format, more details filled in, then the oldest import.
fn keep_score(b: &Book) -> (bool, i32, bool, bool, usize) {
    let progress = match b.status.as_str() {
        "finished" => 200,
        "reading" => 100 + b.progress as i32,
        _ => 0,
    };
    let details = [
        b.description.is_some(),
        b.publisher.is_some(),
        b.published.is_some(),
        b.isbn.is_some(),
        b.series.is_some(),
    ]
    .iter()
    .filter(|x| **x)
    .count();
    (
        !b.on_devices.is_empty(),
        progress,
        b.cover_path.is_some(),
        matches!(b.format.as_str(), "epub" | "kepub"),
        details,
    )
}

fn ignored(conn: &Connection) -> Result<BTreeSet<(i64, i64)>> {
    let pairs: Vec<(i64, i64)> = db::kv_get(conn, IGNORED_KEY)?.unwrap_or_default();
    Ok(pairs.into_iter().collect())
}

/// Groups of books that look like duplicates, best copy first, biggest groups first.
pub fn find(conn: &Connection) -> Result<Vec<Group>> {
    let books = db::list_books(conn)?;
    let ignored = ignored(conn)?;
    let n = books.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    let join = |parent: &mut Vec<usize>, a: usize, b: usize| {
        let pair = (books[a].id.min(books[b].id), books[a].id.max(books[b].id));
        if ignored.contains(&pair) {
            return;
        }
        let (ra, rb) = (root(parent, a), root(parent, b));
        if ra != rb {
            parent[ra] = rb;
        }
    };

    let mut by_isbn: HashMap<String, Vec<usize>> = HashMap::new();
    let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, b) in books.iter().enumerate() {
        if let Some(k) = isbn_key(&b.isbn) {
            by_isbn.entry(k).or_default().push(i);
        }
        by_name.entry(surname(b)).or_default().push(i);
    }
    for members in by_isbn.values() {
        for w in members.windows(2) {
            join(&mut parent, w[0], w[1]);
        }
    }
    for members in by_name.values() {
        for (x, &a) in members.iter().enumerate() {
            for &b in &members[x + 1..] {
                if same_book(&books[a].title, &books[a].author, &books[b].title, &books[b].author) {
                    join(&mut parent, a, b);
                }
            }
        }
    }

    let mut groups: HashMap<usize, Vec<Book>> = HashMap::new();
    for i in 0..n {
        let r = root(&mut parent, i);
        groups.entry(r).or_default().push(books[i].clone());
    }
    let mut out: Vec<Group> = groups.into_values().filter(|g| g.len() > 1).collect();
    for g in &mut out {
        g.sort_by(|a, b| keep_score(b).cmp(&keep_score(a)).then(a.added_at.cmp(&b.added_at)));
    }
    out.sort_by(|a, b| b.len().cmp(&a.len()).then(a[0].title.cmp(&b[0].title)));
    Ok(out)
}

/// Remembers that these books aren't duplicates of each other.
pub fn ignore(conn: &Connection, ids: &[i64]) -> Result<()> {
    let mut pairs = ignored(conn)?;
    for (x, &a) in ids.iter().enumerate() {
        for &b in &ids[x + 1..] {
            pairs.insert((a.min(b), a.max(b)));
        }
    }
    db::kv_set(conn, IGNORED_KEY, &pairs.into_iter().collect::<Vec<_>>())
}

fn status_rank(status: &str, progress: f64) -> f64 {
    match status {
        "finished" => 1000.0,
        "reading" => 100.0 + progress,
        _ => 0.0,
    }
}

/// Folds `remove` into `keep` and deletes them. Returns the files the removed books used,
/// for the caller to delete once the database change has gone through.
fn merge_in(conn: &Connection, keep: i64, remove: &[i64]) -> Result<Vec<String>> {
    let mut leftovers = Vec::new();
    for &r in remove {
        if r == keep {
            continue;
        }
        let k = db::get_book(conn, keep)?;
        let o = db::get_book(conn, r)?;

        // Tags and collections: the union.
        conn.execute(
            "INSERT OR IGNORE INTO book_labels (book_id, label_id) SELECT ?1, label_id FROM book_labels WHERE book_id = ?2",
            params![keep, r],
        )?;
        // E-reader copies: re-link the other copy to the kept book where the kept one isn't
        // on that device; otherwise the device keeps an extra copy you can remove there.
        conn.execute(
            "UPDATE device_books SET book_id = ?1 WHERE book_id = ?2
             AND device_id NOT IN (SELECT device_id FROM device_books WHERE book_id = ?1)",
            params![keep, r],
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO queue (device_id, book_id, queued_at) SELECT device_id, ?1, queued_at FROM queue WHERE book_id = ?2",
            params![keep, r],
        )?;
        // Where you were reading on this computer, if the kept copy has nowhere yet.
        let has_position: Option<i64> = conn
            .query_row("SELECT 1 FROM reading_positions WHERE book_id = ?1", [keep], |row| row.get(0))
            .optional()?;
        if has_position.is_none() {
            conn.execute("UPDATE reading_positions SET book_id = ?1 WHERE book_id = ?2", params![keep, r])?;
        }
        // Reading progress: whichever copy got further.
        if status_rank(&o.status, o.progress) > status_rank(&k.status, k.progress) {
            conn.execute(
                "UPDATE books SET status = ?2, progress = ?3, last_read = COALESCE(?4, last_read) WHERE id = ?1",
                params![keep, o.status, o.progress, o.last_read],
            )?;
        }
        // Details the kept copy is missing.
        conn.execute(
            "UPDATE books SET
                rating = COALESCE(rating, ?2),
                series = COALESCE(series, ?3),
                series_index = CASE WHEN series IS NULL THEN ?4 ELSE series_index END,
                publisher = COALESCE(publisher, ?5),
                published = COALESCE(published, ?6),
                isbn = COALESCE(isbn, ?7),
                language = COALESCE(language, ?8),
                description = CASE WHEN description IS NULL OR length(trim(description)) < 60 THEN COALESCE(?9, description) ELSE description END
             WHERE id = ?1",
            params![keep, o.rating, o.series, o.series_index, o.publisher, o.published, o.isbn, o.language, o.description],
        )?;
        let mut moved_cover = false;
        if k.cover_path.is_none() && o.cover_path.is_some() {
            conn.execute("UPDATE books SET cover_path = ?2 WHERE id = ?1", params![keep, o.cover_path])?;
            moved_cover = true;
        }
        db::delete_book(conn, r)?;
        leftovers.push(o.file_path.clone());
        if !moved_cover {
            if let Some(c) = o.cover_path {
                leftovers.push(c);
            }
        }
    }
    db::requeue_on_devices(conn, keep)?;
    Ok(leftovers)
}

/// Merges duplicates into the book to keep, in one transaction, then deletes the removed
/// copies' files from Shelf's library folder (never files outside it).
pub fn merge(state: &AppState, keep: i64, remove: &[i64]) -> Result<Book> {
    if remove.iter().all(|&r| r == keep) {
        return Err(Error::msg("Choose at least one other copy to merge."));
    }
    let settings = state.settings()?;
    let leftovers = {
        let mut conn = state.db();
        let tx = conn.transaction()?;
        let leftovers = merge_in(&tx, keep, remove)?;
        tx.commit()?;
        leftovers
    };
    let safe = [settings.library_dir.clone(), state.paths.covers_dir.clone()];
    for path in leftovers {
        let p = std::path::Path::new(&path);
        if safe.iter().any(|dir| p.starts_with(dir)) {
            let _ = std::fs::remove_file(p);
        }
    }
    db::get_book(&state.db(), keep)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn import(state: &AppState, dir: &std::path::Path, name: &str, title: &str, author: &str) -> i64 {
        let file = dir.join(name);
        crate::epub::write_test_epub(&file, title, author);
        crate::library::import_into(state, &file, None, &|_| {}).unwrap().0
    }

    #[test]
    fn finds_and_merges_duplicates() {
        let data = tempfile::tempdir().unwrap();
        let state = AppState::open(data.path().to_path_buf(), true).unwrap();
        let a = import(&state, data.path(), "a.epub", "The Final Empire", "Brandon Sanderson");
        let b = import(&state, data.path(), "b.epub", "Mistborn: The Final Empire", "Brandon Sanderson");
        let other = import(&state, data.path(), "c.epub", "The Well of Ascension", "Brandon Sanderson");
        // Same main title, different books.
        let sequel = import(&state, data.path(), "d.epub", "Thrawn: Treason", "Timothy Zahn");
        let first = import(&state, data.path(), "e.epub", "Thrawn", "Timothy Zahn");

        let groups = find(&state.db()).unwrap();
        assert_eq!(groups.len(), 1);
        let ids: Vec<i64> = groups[0].iter().map(|x| x.id).collect();
        assert!(ids.contains(&a) && ids.contains(&b) && !ids.contains(&other));
        assert!(!ids.contains(&sequel) && !ids.contains(&first), "a different subtitle is a different book");

        // The removed copy was finished and tagged: both carry over.
        db::set_progress(&state.db(), b, 100.0, "finished", None).unwrap();
        db::set_labels(&state.db(), b, "tag", &["Cosmere".to_string()]).unwrap();
        let removed_file = db::get_book(&state.db(), b).unwrap().file_path;

        let kept = merge(&state, a, &[b]).unwrap();
        assert_eq!(kept.status, "finished");
        assert_eq!(kept.tags, vec!["Cosmere".to_string()]);
        assert!(db::get_book(&state.db(), b).is_err(), "the other copy is gone");
        assert!(!std::path::Path::new(&removed_file).exists(), "and its file");
        assert!(find(&state.db()).unwrap().is_empty());
    }

    #[test]
    fn remembers_books_that_are_not_duplicates() {
        let data = tempfile::tempdir().unwrap();
        let state = AppState::open(data.path().to_path_buf(), true).unwrap();
        let a = import(&state, data.path(), "a.epub", "Dune", "Frank Herbert");
        let b = import(&state, data.path(), "b.epub", "Dune (Dune Chronicles 1)", "Frank Herbert");
        assert_eq!(find(&state.db()).unwrap().len(), 1);
        ignore(&state.db(), &[a, b]).unwrap();
        assert!(find(&state.db()).unwrap().is_empty());
    }
}

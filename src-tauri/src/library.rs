//! Importing ebook files into the library folder.

use crate::db::{self, BookMeta, NewBook};
use crate::epub;
use crate::error::{Error, Result};
use crate::model::ImportProgress;
use crate::state::AppState;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter, Manager};

/// Formats Shelf accepts, by the "format" name it stores.
pub const SUPPORTED: &[&str] = &["epub", "kepub", "mobi", "azw3", "pdf"];

/// The stored format of a file, treating "x.kepub.epub" as kepub.
pub fn format_of(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_string_lossy().to_lowercase();
    if name.ends_with(".kepub.epub") || name.ends_with(".kepub") {
        return Some("kepub".into());
    }
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    SUPPORTED.contains(&ext.as_str()).then_some(ext)
}

/// The file extension a format is saved with.
pub fn extension_for(format: &str) -> &str {
    if format == "kepub" {
        "kepub.epub"
    } else {
        format
    }
}

/// Makes a string safe to use as a file or folder name on every platform.
pub fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_control() || r#"<>:"/\|?*"#.contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let mut cleaned: String = cleaned
        .trim()
        .trim_end_matches(['.', ' '])
        .chars()
        .take(80)
        .collect();
    cleaned = cleaned.trim_end_matches(['.', ' ']).to_string();
    if cleaned.is_empty() {
        return "Untitled".into();
    }
    let stem = cleaned.split('.').next().unwrap_or("").to_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ((stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.len() == 4
            && stem.as_bytes()[3].is_ascii_digit());
    if reserved {
        format!("_{cleaned}")
    } else {
        cleaned
    }
}

/// A path in `dir` for `stem.ext` that doesn't exist yet ("Name (2).epub", ...).
pub fn unique_path(dir: &Path, stem: &str, ext: &str) -> PathBuf {
    let first = dir.join(format!("{stem}.{ext}"));
    if !first.exists() {
        return first;
    }
    (2..)
        .map(|n| dir.join(format!("{stem} ({n}).{ext}")))
        .find(|p| !p.exists())
        .expect("an unused file name")
}

/// FNV-1a over the whole file, prefixed with its size. Used to spot duplicate imports.
pub fn hash_file(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hash: u64 = 0xcbf29ce484222325;
    let mut size: u64 = 0;
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        size += n as u64;
        for &byte in &buf[..n] {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    Ok(format!("{size:x}-{hash:016x}"))
}

/// A readable title from a file name: "the_name-of-the_wind.epub" → "the name of the wind".
fn title_from_file_name(path: &Path) -> String {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let stem = name.split('.').next().unwrap_or(&name);
    let readable = stem.replace(['_', '-'], " ");
    let readable = readable.split_whitespace().collect::<Vec<_>>().join(" ");
    if readable.is_empty() {
        "Untitled".into()
    } else {
        readable
    }
}

fn emit(
    app: &AppHandle,
    file: &str,
    step: &'static str,
    book_id: Option<i64>,
    matched: bool,
    error: Option<String>,
) {
    let _ = app.emit(
        "import-progress",
        ImportProgress {
            file: file.to_string(),
            step,
            book_id,
            matched_wishlist: matched,
            error,
        },
    );
}

/// Copies one file into the library and records it, emitting progress events.
/// Returns the new book's id.
pub fn import_file(app: &AppHandle, src: &Path) -> Result<i64> {
    let state = app.state::<AppState>();
    let label = src
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    emit(app, &label, "reading", None, false, None);

    match import_into(&state, src, None, &|step| {
        emit(app, &label, step, None, false, None)
    }) {
        Ok((id, matched)) => {
            emit(app, &label, "done", Some(id), matched, None);
            let _ = app.emit("library-changed", ());
            Ok(id)
        }
        Err(err) => {
            emit(app, &label, "failed", None, false, Some(err.to_string()));
            Err(err)
        }
    }
}

/// Things that take precedence over what's inside the file (e.g. calibre's metadata).
#[derive(Default)]
pub struct Overrides {
    /// Empty strings and `None`s mean "use what the file says".
    pub meta: Option<BookMeta>,
    pub cover_file: Option<PathBuf>,
    /// Don't queue the book for devices even when auto-send is on (bulk imports).
    pub no_auto_send: bool,
}

/// Combines what the file says with any overrides, field by field.
fn merge_meta(file: &epub::EpubMeta, overrides: Option<&BookMeta>, src: &Path) -> BookMeta {
    let from_file = BookMeta {
        title: file
            .title
            .clone()
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| title_from_file_name(src)),
        author: if file.authors.is_empty() {
            "Unknown author".into()
        } else {
            file.authors.join(" & ")
        },
        author_sort: file.author_sort.clone(),
        series: file.series.clone(),
        series_index: file.series_index,
        description: file.description.clone(),
        publisher: file.publisher.clone(),
        published: file.published.clone(),
        isbn: file.isbn.clone(),
        language: file.language.clone(),
        rating: None,
        tags: file.subjects.clone(),
        collections: Vec::new(),
    };
    let Some(o) = overrides else { return from_file };
    let pick = |a: &str, b: &str| {
        if a.trim().is_empty() {
            b.to_string()
        } else {
            a.to_string()
        }
    };
    let (series, series_index) = if o.series.is_some() {
        (o.series.clone(), o.series_index)
    } else {
        (from_file.series.clone(), from_file.series_index)
    };
    BookMeta {
        title: pick(&o.title, &from_file.title),
        // A sort name from the file only fits the file's author.
        author_sort: o.author_sort.clone().or_else(|| {
            o.author
                .trim()
                .is_empty()
                .then(|| from_file.author_sort.clone())
                .flatten()
        }),
        author: pick(&o.author, &from_file.author),
        series,
        series_index,
        description: o.description.clone().or(from_file.description),
        publisher: o.publisher.clone().or(from_file.publisher),
        published: o.published.clone().or(from_file.published),
        isbn: o.isbn.clone().or(from_file.isbn),
        language: o.language.clone().or(from_file.language),
        rating: o.rating,
        tags: if o.tags.is_empty() {
            from_file.tags
        } else {
            o.tags.clone()
        },
        collections: o.collections.clone(),
    }
}

/// The import itself. Returns the new book's id and whether it matched a wishlist item.
pub fn import_into(
    state: &AppState,
    src: &Path,
    overrides: Option<&Overrides>,
    step: &dyn Fn(&'static str),
) -> Result<(i64, bool)> {
    let format = format_of(src).ok_or_else(|| Error::msg("Shelf can't read this kind of file."))?;
    let size = fs::metadata(src)?.len() as i64;
    let hash = hash_file(src)?;
    if db::book_by_hash(&state.db(), &hash)?.is_some() {
        return Err(Error::Duplicate);
    }

    let mut file_meta = if format == "epub" || format == "kepub" {
        epub::read(src).unwrap_or_default()
    } else {
        Default::default()
    };
    let empty = Overrides::default();
    let overrides = overrides.unwrap_or(&empty);
    if let Some(bytes) = overrides.cover_file.as_ref().and_then(|p| fs::read(p).ok()) {
        let ext = overrides
            .cover_file
            .as_ref()
            .and_then(|p| p.extension())
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_else(|| "jpg".into());
        file_meta.cover = Some((bytes, ext));
    }
    let meta = merge_meta(&file_meta, overrides.meta.as_ref(), src);

    let settings = state.settings()?;
    let folder = settings.library_dir.join(sanitize(&meta.author));
    fs::create_dir_all(&folder)?;
    let dest = unique_path(&folder, &sanitize(&meta.title), extension_for(&format));
    fs::copy(src, &dest)?;

    step("cover");
    let conn = state.db();
    let id = db::insert_book(
        &conn,
        &NewBook {
            meta: &meta,
            format: &format,
            size_bytes: size,
            file_path: &dest.to_string_lossy(),
            content_hash: &hash,
        },
    )?;
    if let Some((bytes, ext)) = &file_meta.cover {
        let cover = state.paths.covers_dir.join(format!("{id}.{ext}"));
        fs::write(&cover, bytes)?;
        db::set_cover(&conn, id, &cover.to_string_lossy())?;
    }

    step("saving");
    let matched = db::take_matching_wish(&conn, &meta.title, &meta.author)?.is_some();
    if settings.auto_send && !overrides.no_auto_send {
        db::queue_for_all_devices(&conn, id)?;
    }
    Ok((id, matched))
}

/// Imports several files, skipping ones that fail (each failure is reported by event).
pub fn import_files(app: &AppHandle, paths: &[PathBuf]) -> Vec<i64> {
    paths
        .iter()
        .filter_map(|p| import_file(app, p).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_names() {
        assert_eq!(sanitize("AC/DC: Live?"), "AC_DC_ Live_");
        assert_eq!(sanitize("  Ends with dots... "), "Ends with dots");
        assert_eq!(sanitize("CON"), "_CON");
        assert_eq!(sanitize(""), "Untitled");
    }

    #[test]
    fn detects_formats() {
        assert_eq!(
            format_of(Path::new("a/Book.KEPUB.EPUB")).as_deref(),
            Some("kepub")
        );
        assert_eq!(format_of(Path::new("a/Book.epub")).as_deref(), Some("epub"));
        assert_eq!(format_of(Path::new("a/Book.docx")), None);
    }

    #[test]
    fn titles_from_file_names() {
        assert_eq!(
            title_from_file_name(Path::new("the_name-of-the_wind.epub")),
            "the name of the wind"
        );
    }
}

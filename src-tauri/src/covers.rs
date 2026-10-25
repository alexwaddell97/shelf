//! Custom covers: from an image file, from an online cover search, or back to none.
//! Covers are normalised to JPEG and capped in size so they stay quick to show and send.

use crate::db;
use crate::error::{Error, Result};
use crate::state::AppState;
use image::ImageReader;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::{Path, PathBuf};

/// Big enough for any e-reader screen, small enough to keep the library light.
const MAX_WIDTH: u32 = 1400;
const MAX_HEIGHT: u32 = 2100;
const MAX_DOWNLOAD: usize = 20 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoverCandidate {
    /// Small image for the picker.
    pub thumb_url: String,
    /// Full-size image to use.
    pub url: String,
}

/// Decodes any supported image and re-encodes it as a size-capped JPEG.
pub fn normalise(bytes: &[u8]) -> Result<Vec<u8>> {
    let image = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()?
        .decode()
        .map_err(|_| Error::msg("That image couldn't be read. Try a JPEG, PNG, WebP or GIF."))?;
    let image = if image.width() > MAX_WIDTH || image.height() > MAX_HEIGHT {
        image.resize(MAX_WIDTH, MAX_HEIGHT, image::imageops::FilterType::Lanczos3)
    } else {
        image
    };
    let mut out = Vec::new();
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 88);
    image
        .to_rgb8()
        .write_with_encoder(encoder)
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(out)
}

/// Saves new cover bytes for a book, replacing (and deleting) the old cover file.
/// A fresh file name each time keeps the interface from showing a cached old image.
pub fn save(state: &AppState, book_id: i64, bytes: &[u8]) -> Result<PathBuf> {
    let jpeg = normalise(bytes)?;
    let stamp = chrono::Utc::now().timestamp_millis();
    let path = state
        .paths
        .covers_dir
        .join(format!("{book_id}-{stamp}.jpg"));
    std::fs::write(&path, jpeg)?;
    let conn = state.db();
    let old = db::get_book(&conn, book_id)?.cover_path;
    db::set_cover(&conn, book_id, &path.to_string_lossy())?;
    db::requeue_on_devices(&conn, book_id)?;
    if let Some(old) = old {
        let _ = std::fs::remove_file(old);
    }
    Ok(path)
}

pub fn from_file(state: &AppState, book_id: i64, file: &Path) -> Result<PathBuf> {
    let bytes = std::fs::read(file)?;
    save(state, book_id, &bytes)
}

/// Removes the cover, so the interface shows its generated one.
pub fn remove(state: &AppState, book_id: i64) -> Result<()> {
    let conn = state.db();
    let old = db::get_book(&conn, book_id)?.cover_path;
    conn.execute(
        "UPDATE books SET cover_path = NULL WHERE id = ?1",
        [book_id],
    )?;
    db::requeue_on_devices(&conn, book_id)?;
    if let Some(old) = old {
        let _ = std::fs::remove_file(old);
    }
    Ok(())
}

/// Hosts covers may be downloaded from (the ones the cover search and catalogue return).
pub fn check_cover_url(url: &str) -> Result<()> {
    let parsed =
        reqwest::Url::parse(url).map_err(|_| Error::msg("That image link isn't valid."))?;
    let host = parsed.host_str().unwrap_or_default();
    let allowed = [
        "covers.openlibrary.org",
        "archive.org",
        "gutenberg.org",
        "gr-assets.com",
    ]
    .iter()
    .any(|h| host == *h || host.ends_with(&format!(".{h}")));
    if parsed.scheme() == "https" && allowed {
        Ok(())
    } else {
        Err(Error::msg(
            "Covers can only be downloaded from Open Library, Project Gutenberg or Goodreads.",
        ))
    }
}

pub async fn download(http: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
    check_cover_url(url)?;
    let response = http.get(url).send().await?.error_for_status()?;
    if response
        .content_length()
        .is_some_and(|len| len as usize > MAX_DOWNLOAD)
    {
        return Err(Error::msg("That image is too large."));
    }
    let bytes = response.bytes().await?;
    if bytes.len() > MAX_DOWNLOAD {
        return Err(Error::msg("That image is too large."));
    }
    Ok(bytes.to_vec())
}

#[derive(Deserialize)]
struct SearchResponse {
    docs: Vec<SearchDoc>,
}

#[derive(Deserialize)]
struct SearchDoc {
    key: String,
    cover_i: Option<i64>,
}

#[derive(Deserialize)]
struct EditionsResponse {
    entries: Vec<Edition>,
}

#[derive(Deserialize)]
struct Edition {
    #[serde(default)]
    covers: Vec<i64>,
}

/// Cover options from Open Library: the matching works' covers plus their editions' covers.
pub async fn search(
    http: &reqwest::Client,
    title: &str,
    author: &str,
) -> Result<Vec<CoverCandidate>> {
    let url = format!(
        "https://openlibrary.org/search.json?title={}&author={}&limit=5&fields=key,cover_i",
        urlencoding::encode(title),
        urlencoding::encode(author)
    );
    let found: SearchResponse = http
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let mut ids: Vec<i64> = found.docs.iter().filter_map(|d| d.cover_i).collect();
    if let Some(work) = found.docs.first() {
        let url = format!("https://openlibrary.org{}/editions.json?limit=60", work.key);
        if let Ok(response) = http.get(url).send().await {
            if let Ok(editions) = response.json::<EditionsResponse>().await {
                ids.extend(editions.entries.into_iter().flat_map(|e| e.covers));
            }
        }
    }
    let mut seen = std::collections::HashSet::new();
    Ok(ids
        .into_iter()
        .filter(|id| *id > 0 && seen.insert(*id))
        .take(30)
        .map(|id| CoverCandidate {
            thumb_url: format!("https://covers.openlibrary.org/b/id/{id}-M.jpg"),
            url: format!("https://covers.openlibrary.org/b/id/{id}-L.jpg"),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let image = image::RgbImage::from_pixel(width, height, image::Rgb([200, 120, 40]));
        let mut out = Vec::new();
        image::DynamicImage::ImageRgb8(image)
            .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png)
            .unwrap();
        out
    }

    #[test]
    fn normalises_to_a_capped_jpeg() {
        let jpeg = normalise(&png(3000, 4500)).unwrap();
        assert!(jpeg.starts_with(&[0xFF, 0xD8]), "is a JPEG");
        let decoded = image::load_from_memory(&jpeg).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (1400, 2100));
        assert!(normalise(b"not an image").is_err());
    }

    #[test]
    fn saves_replaces_and_removes_covers() {
        let data = tempfile::tempdir().unwrap();
        let state = AppState::open(data.path().to_path_buf(), true).unwrap();
        let file = data.path().join("b.epub");
        crate::epub::write_test_epub(&file, "Piranesi", "Susanna Clarke");
        let (id, _) = crate::library::import_into(&state, &file, None, &|_| {}).unwrap();
        let original = db::get_book(&state.db(), id).unwrap().cover_path.unwrap();

        let first = save(&state, id, &png(600, 900)).unwrap();
        assert!(first.is_file());
        assert!(
            !Path::new(&original).exists(),
            "the old cover file is cleaned up"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
        let second = save(&state, id, &png(600, 900)).unwrap();
        assert_ne!(first, second);
        assert!(!first.exists());

        remove(&state, id).unwrap();
        assert!(db::get_book(&state.db(), id).unwrap().cover_path.is_none());
        assert!(!second.exists());
    }

    #[test]
    fn only_downloads_covers_from_known_hosts() {
        assert!(check_cover_url("https://covers.openlibrary.org/b/id/1-L.jpg").is_ok());
        assert!(check_cover_url("https://ia800100.us.archive.org/x.jpg").is_ok());
        assert!(check_cover_url("https://i.gr-assets.com/images/x.jpg").is_ok());
        assert!(check_cover_url("https://evil.example/x.jpg").is_err());
        assert!(check_cover_url("http://covers.openlibrary.org/b/id/1-L.jpg").is_err());
    }
}

//! Types shared with the frontend. Field names serialize as camelCase to match
//! `src/lib/api.ts`.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Book {
    pub id: i64,
    pub title: String,
    pub author: String,
    pub series: Option<String>,
    pub series_index: Option<f64>,
    pub description: Option<String>,
    pub format: String,
    pub size_bytes: i64,
    pub added_at: String,
    pub cover_path: Option<String>,
    pub file_path: String,
    pub progress: f64,
    pub status: String,
    pub last_read: Option<String>,
    pub author_sort: Option<String>,
    pub publisher: Option<String>,
    pub published: Option<String>,
    pub isbn: Option<String>,
    pub language: Option<String>,
    pub rating: Option<u8>,
    pub tags: Vec<String>,
    pub collections: Vec<String>,
    pub on_devices: Vec<String>,
    pub queued_for: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookPatch {
    pub title: Option<String>,
    pub author: Option<String>,
    // `Some(None)` clears the field; a missing key leaves it unchanged.
    #[serde(default, deserialize_with = "deserialize_some")]
    pub series: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub series_index: Option<Option<f64>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub description: Option<Option<String>>,
    /// `Some(None)` (or an empty string) recomputes it from the author.
    #[serde(default, deserialize_with = "deserialize_some")]
    pub author_sort: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub publisher: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub published: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub isbn: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub language: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub rating: Option<Option<u8>>,
    pub tags: Option<Vec<String>>,
    pub collections: Option<Vec<String>>,
}

fn deserialize_some<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub connected: bool,
    pub mount_path: Option<String>,
    pub last_seen: Option<String>,
    /// When Shelf last finished a sync with this device.
    pub last_synced: Option<String>,
    pub support_level: String,
    pub send_format: String,
    pub free_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub device_id: String,
    pub sent: usize,
    pub progress_updated: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub url_template: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogBook {
    pub key: String,
    pub title: String,
    pub author: String,
    pub year: Option<i32>,
    pub cover_url: Option<String>,
    pub is_free: bool,
    pub epub_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WishItem {
    pub id: i64,
    pub catalog_key: Option<String>,
    pub title: String,
    pub author: String,
    pub cover_url: Option<String>,
    pub added_at: String,
    pub origin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub library_dir: PathBuf,
    pub watch_folders: Vec<PathBuf>,
    pub auto_send: bool,
    pub convert_for_device: bool,
    pub backup_device_db: bool,
    pub setup_done: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tools {
    pub calibre: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProgress {
    pub file: String,
    pub step: &'static str,
    pub book_id: Option<i64>,
    pub matched_wishlist: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncProgress {
    pub device_id: String,
    /// "sending" (copying books), "reading" (updating reading progress) or "importing".
    pub phase: &'static str,
    pub done: usize,
    pub total: usize,
    pub current: Option<String>,
}

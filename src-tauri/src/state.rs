use crate::db;
use crate::error::Result;
use crate::model::{Settings, Source};
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use tauri::{AppHandle, Manager};

pub struct Paths {
    pub data_dir: PathBuf,
    pub covers_dir: PathBuf,
    pub tmp_dir: PathBuf,
    /// Set when `SHELF_DATA_DIR` points Shelf at a separate folder for testing; the
    /// library then defaults to living inside it and no folders are watched.
    pub sandboxed: bool,
}

pub struct AppState {
    db: Mutex<Connection>,
    pub paths: Paths,
    pub http: reqwest::Client,
    /// Connected devices: id → mount point. Updated by the device poller.
    pub connected: Mutex<HashMap<String, PathBuf>>,
    pub watcher: Mutex<Option<notify::RecommendedWatcher>>,
    /// A book opened from the file manager, waiting for the interface to show it.
    pub pending_open: Mutex<Option<i64>>,
}

impl AppState {
    pub fn init(app: &AppHandle) -> Result<Self> {
        let override_dir = std::env::var_os("SHELF_DATA_DIR").map(PathBuf::from);
        let sandboxed = override_dir.is_some();
        let data_dir = match override_dir {
            Some(dir) => dir,
            None => app.path().app_data_dir()?,
        };
        Self::open(data_dir, sandboxed)
    }

    /// Opens (creating if needed) the library stored in `data_dir`.
    pub fn open(data_dir: PathBuf, sandboxed: bool) -> Result<Self> {
        let paths = Paths {
            covers_dir: data_dir.join("covers"),
            tmp_dir: data_dir.join("tmp"),
            data_dir,
            sandboxed,
        };
        std::fs::create_dir_all(&paths.covers_dir)?;
        std::fs::create_dir_all(&paths.tmp_dir)?;
        let conn = db::open(&paths.data_dir.join("shelf.db"))?;
        let http = reqwest::Client::builder()
            .user_agent(concat!(
                "Shelf/",
                env!("CARGO_PKG_VERSION"),
                " (open-source ebook manager)"
            ))
            .timeout(std::time::Duration::from_secs(12))
            .build()?;
        Ok(AppState {
            db: Mutex::new(conn),
            paths,
            http,
            connected: Mutex::new(HashMap::new()),
            watcher: Mutex::new(None),
            pending_open: Mutex::new(None),
        })
    }

    pub fn db(&self) -> MutexGuard<'_, Connection> {
        self.db
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn connected(&self) -> MutexGuard<'_, HashMap<String, PathBuf>> {
        self.connected
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn settings(&self) -> Result<Settings> {
        let conn = self.db();
        Ok(db::kv_get(&conn, "settings")?.unwrap_or_else(|| self.default_settings()))
    }

    pub fn save_settings(&self, settings: &Settings) -> Result<()> {
        db::kv_set(&self.db(), "settings", settings)
    }

    fn default_settings(&self) -> Settings {
        let library_dir = match dirs::document_dir() {
            Some(docs) if !self.paths.sandboxed => docs.join("Shelf Library"),
            _ => self.paths.data_dir.join("library"),
        };
        let watch_folders = if self.paths.sandboxed {
            Vec::new()
        } else {
            dirs::download_dir().into_iter().collect()
        };
        Settings {
            library_dir,
            watch_folders,
            auto_send: true,
            convert_for_device: true,
            backup_device_db: true,
            setup_done: false,
        }
    }

    pub fn sources(&self) -> Result<Vec<Source>> {
        let conn = self.db();
        Ok(db::kv_get(&conn, "sources")?.unwrap_or_else(default_sources))
    }

    pub fn save_sources(&self, sources: &[Source]) -> Result<()> {
        db::kv_set(&self.db(), "sources", &sources)
    }
}

/// The search links Shelf ships with: legitimate stores and free public-domain libraries.
/// Users can add any other site themselves.
pub fn default_sources() -> Vec<Source> {
    let source = |id: &str, name: &str, kind: &str, url: &str, enabled: bool| Source {
        id: id.into(),
        name: name.into(),
        kind: kind.into(),
        url_template: url.into(),
        enabled,
    };
    vec![
        source(
            "kobo",
            "Kobo Store",
            "store",
            "https://www.kobo.com/search?query={query}",
            true,
        ),
        source(
            "play",
            "Google Play Books",
            "store",
            "https://play.google.com/store/search?q={query}&c=books",
            true,
        ),
        source(
            "ebookscom",
            "eBooks.com",
            "store",
            "https://www.ebooks.com/en-us/searchapp/searchresults.net?term={query}",
            true,
        ),
        source(
            "smashwords",
            "Smashwords",
            "store",
            "https://www.smashwords.com/books/search?query={query}",
            false,
        ),
        source(
            "standardebooks",
            "Standard Ebooks",
            "free",
            "https://standardebooks.org/ebooks?query={query}",
            true,
        ),
        source(
            "gutenberg",
            "Project Gutenberg",
            "free",
            "https://www.gutenberg.org/ebooks/search/?query={query}",
            true,
        ),
        source(
            "archive",
            "Internet Archive",
            "free",
            "https://archive.org/search?query={query}",
            false,
        ),
    ]
}

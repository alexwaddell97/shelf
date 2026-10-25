use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("couldn't read the ebook archive: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("couldn't read the ebook metadata: {0}")]
    Xml(#[from] roxmltree::Error),
    #[error("network error: {0}")]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Tauri(#[from] tauri::Error),
    #[error("This book is already in your library.")]
    Duplicate,
    #[error("{0}")]
    Msg(String),
}

impl Error {
    pub fn msg(text: impl Into<String>) -> Self {
        Error::Msg(text.into())
    }
}

// Commands return errors to the frontend as plain strings.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

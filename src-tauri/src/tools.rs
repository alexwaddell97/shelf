//! Finds the optional external tools Shelf can use.

use crate::model::Tools;
use std::path::{Path, PathBuf};

fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn on_path(name: &str) -> Option<PathBuf> {
    let file = exe(name);
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .map(|dir| dir.join(&file))
        .find(|candidate| candidate.is_file())
}

/// kepubify (https://pgaskin.net/kepubify/) converts EPUB to Kobo's kepub format.
/// Looked for on PATH and in Shelf's own tools folder.
pub fn kepubify(data_dir: &Path) -> Option<PathBuf> {
    let bundled = data_dir.join("tools").join(exe("kepubify"));
    if bundled.is_file() {
        return Some(bundled);
    }
    on_path("kepubify")
}

/// calibre's command-line converter, used for formats Shelf can't convert itself.
pub fn ebook_convert() -> Option<PathBuf> {
    if let Some(found) = on_path("ebook-convert") {
        return Some(found);
    }
    let known: &[&str] = if cfg!(windows) {
        &[
            r"C:\Program Files\Calibre2\ebook-convert.exe",
            r"C:\Program Files (x86)\Calibre2\ebook-convert.exe",
        ]
    } else if cfg!(target_os = "macos") {
        &["/Applications/calibre.app/Contents/MacOS/ebook-convert"]
    } else {
        &["/usr/bin/ebook-convert", "/opt/calibre/ebook-convert"]
    };
    known.iter().map(PathBuf::from).find(|p| p.is_file())
}

pub fn find(data_dir: &Path) -> Tools {
    Tools {
        kepubify: kepubify(data_dir).map(|p| p.display().to_string()),
        calibre: ebook_convert().map(|p| p.display().to_string()),
    }
}

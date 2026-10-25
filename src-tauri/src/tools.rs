//! Finds the optional external tools Shelf can use.

use crate::model::Tools;
use std::path::PathBuf;

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

pub fn find() -> Tools {
    Tools {
        calibre: ebook_convert().map(|p| p.display().to_string()),
    }
}

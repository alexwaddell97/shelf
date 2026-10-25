//! Book search beyond the library: Open Library for the catalogue, and Project
//! Gutenberg (through the Gutendex API) for free public-domain EPUBs.

use crate::db::same_work;
use crate::error::{Error, Result};
use crate::model::CatalogBook;
use serde::Deserialize;

#[derive(Deserialize)]
struct OlResponse {
    docs: Vec<OlDoc>,
}

#[derive(Deserialize)]
struct OlDoc {
    key: String,
    title: String,
    #[serde(default)]
    author_name: Vec<String>,
    first_publish_year: Option<i32>,
    cover_i: Option<i64>,
}

#[derive(Deserialize)]
struct GutendexResponse {
    results: Vec<GutendexBook>,
}

#[derive(Deserialize)]
struct GutendexBook {
    id: i64,
    title: String,
    #[serde(default)]
    authors: Vec<GutendexAuthor>,
    #[serde(default)]
    formats: std::collections::HashMap<String, String>,
}

#[derive(Deserialize)]
struct GutendexAuthor {
    name: String,
}

/// Gutenberg lists authors as "Austen, Jane"; turn that into "Jane Austen".
fn display_name(name: &str) -> String {
    match name.split_once(", ") {
        Some((last, first)) => format!("{} {last}", first.split(" (").next().unwrap_or(first)),
        None => name.to_string(),
    }
}

async fn open_library(http: &reqwest::Client, query: &str) -> Result<Vec<CatalogBook>> {
    let url = format!(
        "https://openlibrary.org/search.json?q={}&limit=16&fields=key,title,author_name,first_publish_year,cover_i",
        urlencoding::encode(query)
    );
    let response: OlResponse = http
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(response
        .docs
        .into_iter()
        .map(|doc| CatalogBook {
            key: doc.key,
            title: doc.title,
            author: doc
                .author_name
                .first()
                .cloned()
                .unwrap_or_else(|| "Unknown author".into()),
            year: doc.first_publish_year,
            cover_url: doc
                .cover_i
                .map(|id| format!("https://covers.openlibrary.org/b/id/{id}-L.jpg")),
            is_free: false,
            epub_url: None,
        })
        .collect())
}

async fn gutenberg(http: &reqwest::Client, query: &str) -> Result<Vec<CatalogBook>> {
    let url = format!(
        "https://gutendex.com/books/?search={}",
        urlencoding::encode(query)
    );
    let response: GutendexResponse = http
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(response
        .results
        .into_iter()
        .filter_map(|book| {
            let epub = book.formats.get("application/epub+zip")?.clone();
            Some(CatalogBook {
                key: format!("gutenberg:{}", book.id),
                author: book
                    .authors
                    .first()
                    .map(|a| display_name(&a.name))
                    .unwrap_or_else(|| "Unknown author".into()),
                title: book.title,
                year: None,
                cover_url: book.formats.get("image/jpeg").cloned(),
                is_free: true,
                epub_url: Some(epub),
            })
        })
        .take(8)
        .collect())
}

/// Searches both catalogues at once. Open Library results that have a matching
/// Gutenberg book are marked free; other Gutenberg finds are appended.
pub async fn search(http: &reqwest::Client, query: &str) -> Result<Vec<CatalogBook>> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let (ol, gb) = tokio::join!(open_library(http, query), gutenberg(http, query));
    let (mut results, free) = match (ol, gb) {
        (Err(e), Err(_)) => return Err(e),
        (ol, gb) => (ol.unwrap_or_default(), gb.unwrap_or_default()),
    };
    let mut unmatched = Vec::new();
    for free_book in free {
        match results.iter_mut().find(|b| {
            !b.is_free && same_work(&b.title, &b.author, &free_book.title, &free_book.author)
        }) {
            Some(book) => {
                book.is_free = true;
                book.epub_url = free_book.epub_url;
            }
            None => unmatched.push(free_book),
        }
    }
    results.extend(unmatched);
    Ok(results)
}

/// Only Project Gutenberg downloads are allowed through `download_free`.
pub fn check_free_url(url: &str) -> Result<()> {
    let parsed =
        reqwest::Url::parse(url).map_err(|_| Error::msg("That download link isn't valid."))?;
    let host = parsed.host_str().unwrap_or_default();
    if parsed.scheme() == "https" && (host == "gutenberg.org" || host.ends_with(".gutenberg.org")) {
        Ok(())
    } else {
        Err(Error::msg(
            "Shelf only downloads free books directly from Project Gutenberg.",
        ))
    }
}

pub async fn download(http: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
    check_free_url(url)?;
    let response = http
        .get(url)
        .timeout(std::time::Duration::from_secs(60))
        .send()
        .await?
        .error_for_status()?;
    Ok(response.bytes().await?.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_gutenberg_names() {
        assert_eq!(display_name("Austen, Jane"), "Jane Austen");
        assert_eq!(
            display_name("Shelley, Mary Wollstonecraft (Godwin)"),
            "Mary Wollstonecraft Shelley"
        );
        assert_eq!(display_name("Homer"), "Homer");
    }

    #[test]
    fn only_allows_gutenberg_downloads() {
        assert!(check_free_url("https://www.gutenberg.org/ebooks/1342.epub3.images").is_ok());
        assert!(check_free_url("http://www.gutenberg.org/ebooks/1342.epub3.images").is_err());
        assert!(check_free_url("https://example.com/book.epub").is_err());
        assert!(check_free_url("https://gutenberg.org.evil.com/x.epub").is_err());
    }
}

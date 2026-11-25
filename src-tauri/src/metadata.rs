//! Book details from Open Library, looked up by ISBN when the book has one, otherwise by
//! title and author. Details come from one edition (the book's own language, else English)
//! rather than a mix of every edition's, and results that aren't the same work are dropped,
//! so "fill in what's missing" never copies another book's details.
//!
//! Google Books was tried too, but without an API key its free quota is shared by every
//! app and runs out daily, so it isn't dependable enough to rely on.

use crate::db::same_work;
use crate::error::{Error, Result};
use crate::model::{Book, BookPatch};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Open Library's search can take well over ten seconds at busy times, and sometimes
/// doesn't accept the connection at all; a few patient tries get through most of those.
const TIMEOUT: Duration = Duration::from_secs(40);
const RETRY_PAUSES: [Duration; 2] = [Duration::from_secs(2), Duration::from_secs(5)];

/// Details found for a book.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    /// Where it came from, e.g. "Open Library".
    pub source: String,
    /// The source's page for the edition.
    pub link: Option<String>,
    pub title: String,
    pub author: String,
    pub description: Option<String>,
    pub publisher: Option<String>,
    /// "YYYY" or "YYYY-MM-DD".
    pub published: Option<String>,
    pub isbn: Option<String>,
    pub language: Option<String>,
    pub cover_url: Option<String>,
    /// The book's own ISBN found it, rather than its title and author.
    pub by_isbn: bool,
}

/// A description this short is a placeholder ("The Three Body Problem Series"), not a blurb.
pub fn description_missing(text: Option<&str>) -> bool {
    text.map(str::trim).is_none_or(|t| t.chars().count() < 60)
}

/// Open Library descriptions sometimes end in a source line or Markdown link list.
fn tidy_description(text: &str) -> String {
    let cut = text.find("----------").unwrap_or(text.len());
    text[..cut]
        .lines()
        .filter(|l| {
            let l = l.trim_start();
            !l.starts_with("[1]:") && !l.starts_with("([source]")
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn digits(isbn: &str) -> String {
    isbn.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

/// Open Library dates come as "2008", "May 2008" or "May 1, 2008": keep a full date when
/// there is one, otherwise the year.
fn normalise_date(text: &str) -> Option<String> {
    let text = text.trim();
    for format in ["%B %d, %Y", "%b %d, %Y", "%Y-%m-%d", "%d %B %Y"] {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(text, format) {
            return Some(d.format("%Y-%m-%d").to_string());
        }
    }
    let chars: Vec<char> = text.chars().collect();
    chars
        .windows(4)
        .find(|w| w.iter().all(|c| c.is_ascii_digit()) && (w[0] == '1' || w[0] == '2'))
        .map(|w| w.iter().collect())
}

/// "en", "eng", "en-GB" → Open Library's three-letter code.
fn ol_language(code: Option<&str>) -> &'static str {
    match code.map(|c| c.to_ascii_lowercase()) {
        Some(c) if c.starts_with("fr") => "fre",
        Some(c) if c.starts_with("de") || c == "ger" => "ger",
        Some(c) if c.starts_with("es") || c == "spa" => "spa",
        Some(c) if c.starts_with("it") => "ita",
        Some(c) if c.starts_with("nl") || c == "dut" => "dut",
        Some(c) if c.starts_with("pt") || c == "por" => "por",
        _ => "eng",
    }
}

async fn get_once<T: DeserializeOwned>(http: &reqwest::Client, url: &str) -> Result<T> {
    Ok(http
        .get(url)
        .timeout(TIMEOUT)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

/// Worth trying again: Open Library was slow, refused the connection, or said it's busy.
fn transient(e: &Error) -> bool {
    matches!(e, Error::Http(e) if e.is_timeout()
        || e.is_connect()
        || e.is_request()
        || e.status().is_some_and(|s| s.is_server_error() || s.as_u16() == 429))
}

/// Up to three tries, with growing pauses, when Open Library is slow or busy.
async fn get<T: DeserializeOwned>(http: &reqwest::Client, url: &str) -> Result<T> {
    let mut result = get_once(http, url).await;
    for pause in RETRY_PAUSES {
        match &result {
            Err(e) if transient(e) => {
                tokio::time::sleep(pause).await;
                result = get_once(http, url).await;
            }
            _ => break,
        }
    }
    result
}

/// The title to search with: without its subtitle or series note, which Open Library's
/// search doesn't match ("Holy Island: A DCI Ryan Mystery (Book 1)" → "Holy Island").
fn search_title(title: &str) -> &str {
    let cut = [":", " (", ";", " - "]
        .iter()
        .filter_map(|sep| title.find(sep))
        .min()
        .unwrap_or(title.len());
    let main = title[..cut].trim();
    if main.is_empty() { title } else { main }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Ref {
    key: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Edition {
    key: String,
    title: String,
    publishers: Vec<String>,
    publish_date: Option<String>,
    isbn_13: Vec<String>,
    isbn_10: Vec<String>,
    languages: Vec<Ref>,
    covers: Vec<i64>,
    works: Vec<Ref>,
}

impl Edition {
    fn isbn(&self) -> Option<String> {
        self.isbn_13.first().or(self.isbn_10.first()).map(|i| digits(i))
    }
    fn language(&self) -> Option<String> {
        self.languages
            .first()
            .and_then(|l| l.key.rsplit('/').next())
            .map(str::to_string)
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Editions {
    entries: Vec<Edition>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Work {
    description: Option<Text>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Text {
    Plain(String),
    Typed { value: String },
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Search {
    docs: Vec<Doc>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Doc {
    key: String,
    title: String,
    author_name: Vec<String>,
    first_publish_year: Option<i32>,
    cover_i: Option<i64>,
}

async fn description(http: &reqwest::Client, work_key: &str) -> Option<String> {
    let work: Work = get(http, &format!("https://openlibrary.org{work_key}.json")).await.ok()?;
    let text = match work.description? {
        Text::Plain(s) | Text::Typed { value: s } => tidy_description(&s),
    };
    (!text.is_empty()).then_some(text)
}

fn cover(id: i64) -> String {
    format!("https://covers.openlibrary.org/b/id/{id}-L.jpg")
}

/// The edition with that ISBN, with its work's description.
async fn by_isbn(http: &reqwest::Client, isbn: &str, author: &str) -> Result<Option<Found>> {
    let edition: Edition = match get(http, &format!("https://openlibrary.org/isbn/{isbn}.json")).await {
        Ok(e) => e,
        Err(Error::Http(e)) if e.status() == Some(reqwest::StatusCode::NOT_FOUND) => return Ok(None),
        Err(e) => return Err(e),
    };
    let description = match edition.works.first() {
        Some(w) => description(http, &w.key).await,
        None => None,
    };
    Ok(Some(Found {
        source: "Open Library".into(),
        link: Some(format!("https://openlibrary.org{}", edition.key)),
        title: edition.title.clone(),
        author: author.to_string(),
        description,
        publisher: edition.publishers.first().cloned(),
        published: edition.publish_date.as_deref().and_then(normalise_date),
        isbn: edition.isbn(),
        language: edition.language(),
        cover_url: edition.covers.first().map(|&c| cover(c)),
        by_isbn: true,
    }))
}

/// One work found by title and author: its description, and the details of the edition
/// best matching the book's language.
async fn by_work(http: &reqwest::Client, doc: Doc, language: &str) -> Found {
    let editions_url = format!("https://openlibrary.org{}/editions.json?limit=50", doc.key);
    let (description, editions) = tokio::join!(
        description(http, &doc.key),
        get::<Editions>(http, &editions_url),
    );
    let editions = editions.map(|e| e.entries).unwrap_or_default();
    let wanted = format!("/languages/{language}");
    let score = |e: &Edition| {
        (
            e.languages.iter().any(|l| l.key == wanted),
            e.isbn().is_some(),
            !e.publishers.is_empty(),
            !e.covers.is_empty(),
        )
    };
    let edition = editions.iter().max_by_key(|e| score(e));
    let author = doc.author_name.join(" & ");
    Found {
        source: "Open Library".into(),
        link: Some(format!(
            "https://openlibrary.org{}",
            edition.map(|e| e.key.as_str()).unwrap_or(doc.key.as_str())
        )),
        title: doc.title,
        author,
        description,
        publisher: edition.and_then(|e| e.publishers.first().cloned()),
        published: edition
            .and_then(|e| e.publish_date.as_deref().and_then(normalise_date))
            .or(doc.first_publish_year.map(|y| y.to_string())),
        isbn: edition.and_then(Edition::isbn),
        language: edition.and_then(Edition::language),
        cover_url: edition
            .and_then(|e| e.covers.first().copied())
            .or(doc.cover_i)
            .map(cover),
        by_isbn: false,
    }
}

/// Looks a book up on Open Library: its ISBN first, then its title and author. Only the
/// same work is kept; ISBN matches come first.
pub async fn lookup(http: &reqwest::Client, book: &Book) -> Result<Vec<Found>> {
    let isbn = book
        .isbn
        .as_deref()
        .map(digits)
        .filter(|i| i.len() == 10 || i.len() == 13);
    let author = book.author.split(" & ").next().unwrap_or(&book.author).to_string();
    let language = ol_language(book.language.as_deref());

    let search_url = format!(
        "https://openlibrary.org/search.json?title={}&author={}&limit=3&fields=key,title,author_name,first_publish_year,cover_i",
        urlencoding::encode(search_title(&book.title)),
        urlencoding::encode(&author)
    );
    let (exact, search) = tokio::join!(
        async {
            match &isbn {
                Some(i) => by_isbn(http, i, &book.author).await,
                None => Ok(None),
            }
        },
        get::<Search>(http, &search_url),
    );

    let mut found: Vec<Found> = Vec::new();
    let mut failed = exact.is_err();
    if let Ok(Some(f)) = exact {
        found.push(f);
    }
    match search {
        Ok(search) => {
            // The two best matching works, looked up together.
            let mut works = search
                .docs
                .into_iter()
                .filter(|d| same_work(&book.title, &book.author, &d.title, &d.author_name.join(" & ")));
            match (works.next(), works.next()) {
                (Some(a), Some(b)) => {
                    let (a, b) = tokio::join!(by_work(http, a, language), by_work(http, b, language));
                    found.extend([a, b]);
                }
                (Some(a), None) => found.push(by_work(http, a, language).await),
                _ => {}
            }
        }
        Err(_) => failed = true,
    }
    if found.is_empty() && failed {
        return Err(Error::msg(
            "Open Library didn't answer after several tries. It's sometimes overloaded: try again in a few minutes.",
        ));
    }
    // The same edition can come back from the ISBN and the title search.
    let mut seen = std::collections::HashSet::new();
    found.retain(|f| seen.insert(f.link.clone()));
    Ok(found)
}

/// The patch that fills a book's empty fields from what was found, taking each field from the
/// first result that has it. Nothing already filled in is changed.
pub fn fill_gaps(book: &Book, found: &[Found]) -> BookPatch {
    let first = |pick: &dyn Fn(&Found) -> Option<String>| {
        found
            .iter()
            .find_map(|f| pick(f).filter(|v| !v.trim().is_empty()))
    };
    let empty = |v: &Option<String>| v.as_deref().is_none_or(|s| s.trim().is_empty());
    let mut patch = BookPatch::default();
    if description_missing(book.description.as_deref()) {
        if let Some(d) = first(&|f| f.description.clone()) {
            patch.description = Some(Some(d));
        }
    }
    if empty(&book.publisher) {
        patch.publisher = first(&|f| f.publisher.clone()).map(Some);
    }
    if empty(&book.published) {
        patch.published = first(&|f| f.published.clone()).map(Some);
    }
    if empty(&book.isbn) {
        patch.isbn = first(&|f| f.isbn.clone()).map(Some);
    }
    if empty(&book.language) {
        patch.language = first(&|f| f.language.clone()).map(Some);
    }
    patch
}

/// Whether a `fill_gaps` patch fills in anything at all.
pub fn fills_anything(patch: &BookPatch) -> bool {
    patch.description.is_some()
        || patch.publisher.is_some()
        || patch.published.is_some()
        || patch.isbn.is_some()
        || patch.language.is_some()
}

/// The first cover found, for books that have none.
pub fn first_cover(found: &[Found]) -> Option<String> {
    found.iter().find_map(|f| f.cover_url.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tidies_text_and_dates() {
        assert_eq!(tidy_description("A tale.\n\n----------\nAlso contained in: x"), "A tale.");
        assert!(description_missing(Some("The Three Body Problem Series")));
        assert!(!description_missing(Some(
            "With the scope of Dune and the commercial action of Independence Day, this novel is a page-turner."
        )));
        assert_eq!(normalise_date("May 1, 2008").as_deref(), Some("2008-05-01"));
        assert_eq!(normalise_date("May 2008").as_deref(), Some("2008"));
        assert_eq!(normalise_date("c2014").as_deref(), Some("2014"));
        assert_eq!(normalise_date("unknown"), None);
        assert_eq!(ol_language(Some("en-GB")), "eng");
        assert_eq!(ol_language(Some("fr")), "fre");
        assert_eq!(search_title("Holy Island: A DCI Ryan Mystery (Book 1)"), "Holy Island");
        assert_eq!(search_title("Piranesi"), "Piranesi");
    }

    /// Talks to the real Open Library: `cargo test live_lookup -- --ignored --nocapture`.
    #[tokio::test]
    #[ignore = "needs the internet and a working Open Library"]
    async fn live_lookup() {
        let http = reqwest::Client::builder()
            .user_agent("Shelf test (https://github.com/alexwaddell97/shelf)")
            .build()
            .unwrap();
        let book = Book {
            title: "The Three Body Problem".into(),
            author: "Cixin Liu".into(),
            ..Default::default()
        };
        let started = std::time::Instant::now();
        let found = lookup(&http, &book).await.expect("lookup failed");
        println!("{} results in {:?}", found.len(), started.elapsed());
        for f in &found {
            println!("{:?} {:?} {:?} {:?}", f.title, f.publisher, f.published, f.isbn);
        }
        assert!(!found.is_empty());
    }

    #[test]
    fn fills_only_what_is_missing() {
        let book = Book {
            title: "Piranesi".into(),
            author: "Susanna Clarke".into(),
            publisher: Some("Bloomsbury".into()),
            description: Some("Piranesi Series".into()),
            ..Default::default()
        };
        let found = vec![
            Found {
                publisher: Some("Someone else".into()),
                published: Some("2020".into()),
                ..Default::default()
            },
            Found {
                description: Some(
                    "Piranesi's house is no ordinary building: its rooms are infinite, its corridors endless.".into(),
                ),
                isbn: Some("9781635575637".into()),
                ..Default::default()
            },
        ];
        let patch = fill_gaps(&book, &found);
        assert!(patch.publisher.is_none(), "a filled-in field is left alone");
        assert_eq!(patch.published, Some(Some("2020".into())));
        assert_eq!(patch.isbn, Some(Some("9781635575637".into())));
        assert!(patch.description.is_some(), "a placeholder description counts as missing");
        assert!(patch.language.is_none());
    }
}

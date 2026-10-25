//! Reads metadata and the cover image out of an EPUB file.

use crate::error::{Error, Result};
use roxmltree::{Document, Node, ParsingOptions};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

#[derive(Debug, Default)]
pub struct EpubMeta {
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub series: Option<String>,
    pub series_index: Option<f64>,
    /// The first author's sort name ("Herbert, Frank"), when the file gives one.
    pub author_sort: Option<String>,
    pub publisher: Option<String>,
    /// "YYYY-MM-DD" or "YYYY".
    pub published: Option<String>,
    pub isbn: Option<String>,
    pub language: Option<String>,
    /// dc:subject entries, used as tags.
    pub subjects: Vec<String>,
    /// Image bytes and a file extension ("jpg", "png", ...).
    pub cover: Option<(Vec<u8>, String)>,
}

/// Keeps the date part of a dc:date ("2019-03-05T00:00:00Z" → "2019-03-05"), or a bare year.
pub fn date_part(text: &str) -> Option<String> {
    let text = text.trim();
    let date: String = text.chars().take(10).collect();
    let valid = date.len() >= 4
        && date[..4].chars().all(|c| c.is_ascii_digit())
        && !date.starts_with("0101");
    valid.then(|| {
        if date.len() == 10 && date.as_bytes()[4] == b'-' {
            date
        } else {
            date[..4].to_string()
        }
    })
}

/// An ISBN-10 or ISBN-13 from text like "urn:isbn:978-0-441-01359-3", without dashes.
pub fn isbn_from(text: &str) -> Option<String> {
    let digits: String = text
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == 'X' || *c == 'x')
        .collect();
    let looks_like = (digits.len() == 13
        && (digits.starts_with("978") || digits.starts_with("979")))
        || digits.len() == 10;
    looks_like.then(|| digits.to_uppercase())
}

const OPF_NS: &str = "http://www.idpf.org/2007/opf";

/// Some tools write OPF files that use `opf:` or `dc:` prefixes without declaring them,
/// which strict XML parsers reject but e-readers shrug off. Adds any missing
/// declarations to the root element so the file parses (and anything we write is valid).
pub fn repair_opf(text: &str) -> std::borrow::Cow<'_, str> {
    const KNOWN: &[(&str, &str)] = &[
        ("opf", OPF_NS),
        ("dc", "http://purl.org/dc/elements/1.1/"),
        ("dcterms", "http://purl.org/dc/terms/"),
    ];
    let missing: Vec<&(&str, &str)> = KNOWN
        .iter()
        .filter(|(prefix, _)| {
            let used =
                text.contains(&format!("<{prefix}:")) || text.contains(&format!(" {prefix}:"));
            used && !text.contains(&format!("xmlns:{prefix}="))
        })
        .collect();
    let root = text.find("<package").or_else(|| text.find(":package"));
    let (Some(root), false) = (root, missing.is_empty()) else {
        return std::borrow::Cow::Borrowed(text);
    };
    let name_end = root
        + text[root..]
            .find(|c: char| c.is_whitespace() || c == '>')
            .unwrap_or(0);
    let declarations: String = missing
        .iter()
        .map(|(prefix, uri)| format!(" xmlns:{prefix}=\"{uri}\""))
        .collect();
    let mut repaired = text.to_string();
    repaired.insert_str(name_end, &declarations);
    std::borrow::Cow::Owned(repaired)
}

fn read_entry(zip: &mut ZipArchive<File>, name: &str) -> Result<Vec<u8>> {
    let mut entry = zip.by_name(name)?;
    let mut buf = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut buf)?;
    Ok(buf)
}

fn parse(text: &str) -> Result<Document<'_>> {
    let options = ParsingOptions {
        allow_dtd: true,
        ..ParsingOptions::default()
    };
    Ok(Document::parse_with_options(text, options)?)
}

fn text_of(node: Node) -> Option<String> {
    let text: String = node
        .descendants()
        .filter(|n| n.is_text())
        .filter_map(|n| n.text())
        .collect();
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (!text.is_empty()).then_some(text)
}

/// Strips HTML tags and entities that publishers often put in descriptions. Block
/// tags (paragraphs, line breaks, ...) become spaces; inline ones (bold, links) vanish.
pub fn plain_text(html: &str) -> String {
    const BLOCK: &[&str] = &[
        "p",
        "br",
        "div",
        "li",
        "ul",
        "ol",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "blockquote",
        "tr",
    ];
    let mut out = String::with_capacity(html.len());
    let mut tag: Option<String> = None;
    for c in html.chars() {
        match (&mut tag, c) {
            (None, '<') => tag = Some(String::new()),
            (Some(name), '>') => {
                let name = name
                    .trim_start_matches('/')
                    .split([' ', '/'])
                    .next()
                    .unwrap_or("")
                    .to_lowercase();
                if BLOCK.contains(&name.as_str()) {
                    out.push(' ');
                }
                tag = None;
            }
            (Some(name), c) => name.push(c),
            (None, c) => out.push(c),
        }
    }
    let out = out
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">");
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Joins an href from the OPF onto the OPF's own folder inside the archive.
pub fn resolve(opf_path: &str, href: &str) -> String {
    let href = urlencoding::decode(href)
        .map(|h| h.into_owned())
        .unwrap_or_else(|_| href.to_string());
    let mut parts: Vec<&str> = match opf_path.rfind('/') {
        Some(i) => opf_path[..i].split('/').collect(),
        None => Vec::new(),
    };
    for segment in href.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    parts.join("/")
}

fn image_ext(media_type: &str, href: &str) -> String {
    match media_type {
        "image/jpeg" | "image/jpg" => "jpg".into(),
        "image/png" => "png".into(),
        "image/gif" => "gif".into(),
        "image/webp" => "webp".into(),
        "image/svg+xml" => "svg".into(),
        _ => href.rsplit('.').next().unwrap_or("jpg").to_lowercase(),
    }
}

pub fn read(path: &Path) -> Result<EpubMeta> {
    let mut zip = ZipArchive::new(File::open(path)?)?;

    let container =
        String::from_utf8_lossy(&read_entry(&mut zip, "META-INF/container.xml")?).into_owned();
    let container = parse(&container)?;
    let opf_path = container
        .descendants()
        .find(|n| n.has_tag_name("rootfile"))
        .and_then(|n| n.attribute("full-path"))
        .ok_or_else(|| Error::msg("This EPUB has no package file."))?
        .to_string();

    let opf_text = String::from_utf8_lossy(&read_entry(&mut zip, &opf_path)?).into_owned();
    let opf_text = repair_opf(&opf_text);
    let opf = parse(&opf_text)?;

    let mut meta = EpubMeta::default();
    let metadata = opf
        .descendants()
        .find(|n| n.tag_name().name() == "metadata");
    let mut cover_id: Option<String> = None;
    let mut collection_id: Option<String> = None;
    let mut first_creator_id: Option<String> = None;
    let mut refines: Vec<(String, String, String)> = Vec::new(); // (target id, property, value)

    if let Some(metadata) = metadata {
        for node in metadata.children().filter(|n| n.is_element()) {
            match node.tag_name().name() {
                "title" if meta.title.is_none() => meta.title = text_of(node),
                "creator" => {
                    // EPUB 2 marks the role on the element; skip editors, illustrators, ...
                    let role = node
                        .attributes()
                        .find(|a| a.name() == "role")
                        .map(|a| a.value());
                    if role.is_some_and(|r| r != "aut") {
                        continue;
                    }
                    if let Some(name) = text_of(node) {
                        if meta.authors.is_empty() {
                            meta.author_sort = node
                                .attributes()
                                .find(|a| a.name() == "file-as")
                                .map(|a| a.value().to_string());
                            first_creator_id = node.attribute("id").map(str::to_string);
                        }
                        meta.authors.push(name);
                    }
                }
                "publisher" if meta.publisher.is_none() => meta.publisher = text_of(node),
                "date" if meta.published.is_none() => {
                    // EPUB 2 can list several dates; prefer publication over modification.
                    let event = node
                        .attributes()
                        .find(|a| a.name() == "event")
                        .map(|a| a.value());
                    if event.is_none_or(|e| e == "publication") {
                        meta.published = text_of(node).and_then(|d| date_part(&d));
                    }
                }
                "identifier" if meta.isbn.is_none() => {
                    let scheme = node
                        .attributes()
                        .find(|a| a.name() == "scheme")
                        .map(|a| a.value().to_lowercase());
                    if let Some(text) = text_of(node) {
                        if scheme.as_deref() == Some("isbn") || text.to_lowercase().contains("isbn")
                        {
                            meta.isbn = isbn_from(&text);
                        }
                    }
                }
                "language" if meta.language.is_none() => meta.language = text_of(node),
                "subject" => {
                    if let Some(subject) = text_of(node) {
                        if !meta
                            .subjects
                            .iter()
                            .any(|s| s.eq_ignore_ascii_case(&subject))
                        {
                            meta.subjects.push(subject);
                        }
                    }
                }
                "description" if meta.description.is_none() => {
                    meta.description = text_of(node)
                        .map(|d| plain_text(&d))
                        .filter(|d| !d.is_empty());
                }
                "meta" => {
                    let name = node.attribute("name").unwrap_or_default();
                    let content = node.attribute("content").unwrap_or_default();
                    match name {
                        "cover" => cover_id = Some(content.to_string()),
                        "calibre:series" if !content.is_empty() => {
                            meta.series = Some(content.to_string())
                        }
                        "calibre:series_index" => meta.series_index = content.parse().ok(),
                        _ => {}
                    }
                    match node.attribute("property") {
                        Some("belongs-to-collection") => {
                            if meta.series.is_none() {
                                meta.series = text_of(node);
                                collection_id = node.attribute("id").map(str::to_string);
                            }
                        }
                        Some(property) => {
                            if let (Some(target), Some(value)) =
                                (node.attribute("refines"), text_of(node))
                            {
                                refines.push((
                                    target.trim_start_matches('#').to_string(),
                                    property.to_string(),
                                    value,
                                ));
                            }
                        }
                        None => {}
                    }
                }
                _ => {}
            }
        }
    }

    for (target, property, value) in &refines {
        if collection_id.as_ref() == Some(target)
            && property == "group-position"
            && meta.series_index.is_none()
        {
            meta.series_index = value.parse().ok();
        }
        // EPUB 3 gives the sort name as <meta refines="#creator" property="file-as">.
        if first_creator_id.as_ref() == Some(target)
            && property == "file-as"
            && meta.author_sort.is_none()
        {
            meta.author_sort = Some(value.clone());
        }
    }

    // Find the cover: EPUB 3 marks it with properties="cover-image"; EPUB 2 points at it
    // from <meta name="cover">; failing that, take an image whose id or name says "cover".
    let items: Vec<Node> = opf
        .descendants()
        .filter(|n| {
            n.tag_name().name() == "item"
                && (n.tag_name().namespace() == Some(OPF_NS) || n.tag_name().namespace().is_none())
        })
        .collect();
    let is_image = |n: &Node| {
        n.attribute("media-type")
            .is_some_and(|m| m.starts_with("image/"))
    };
    let cover_item = items
        .iter()
        .find(|n| {
            n.attribute("properties")
                .is_some_and(|p| p.split_whitespace().any(|p| p == "cover-image"))
        })
        .or_else(|| {
            cover_id.as_deref().and_then(|id| {
                items
                    .iter()
                    .find(|n| n.attribute("id") == Some(id) && is_image(n))
            })
        })
        .or_else(|| {
            items.iter().find(|n| {
                is_image(n)
                    && (n
                        .attribute("id")
                        .is_some_and(|i| i.to_lowercase().contains("cover"))
                        || n.attribute("href")
                            .is_some_and(|h| h.to_lowercase().contains("cover")))
            })
        });

    if let Some(item) = cover_item {
        let href = item.attribute("href").unwrap_or_default();
        let media_type = item.attribute("media-type").unwrap_or_default();
        if let Ok(bytes) = read_entry(&mut zip, &resolve(&opf_path, href)) {
            if !bytes.is_empty() {
                meta.cover = Some((bytes, image_ext(media_type, href)));
            }
        }
    }

    Ok(meta)
}

/// Writes a minimal valid EPUB 3 with a cover, for tests elsewhere in the crate.
#[cfg(test)]
pub(crate) fn write_test_epub(path: &Path, title: &str, author: &str) {
    use std::io::Write;
    let mut zip = zip::ZipWriter::new(File::create(path).unwrap());
    let options = zip::write::SimpleFileOptions::default();
    let files = [
        ("mimetype", "application/epub+zip".to_string()),
        (
            "META-INF/container.xml",
            r#"<?xml version="1.0"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#.to_string(),
        ),
        (
            "content.opf",
            format!(
                r#"<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>{title}</dc:title><dc:creator>{author}</dc:creator></metadata><manifest><item id="c" properties="cover-image" href="cover.svg" media-type="image/svg+xml"/></manifest></package>"#
            ),
        ),
        ("cover.svg", r#"<svg xmlns="http://www.w3.org/2000/svg" width="6" height="9"/>"#.to_string()),
    ];
    for (name, body) in files {
        let options = if name == "mimetype" {
            options.compression_method(zip::CompressionMethod::Stored)
        } else {
            options
        };
        zip.start_file(name, options).unwrap();
        zip.write_all(body.as_bytes()).unwrap();
    }
    zip.finish().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn make_epub(dir: &Path, opf: &str) -> std::path::PathBuf {
        let path = dir.join("book.epub");
        let mut zip = zip::ZipWriter::new(File::create(&path).unwrap());
        let options = SimpleFileOptions::default();
        zip.start_file("mimetype", options).unwrap();
        zip.write_all(b"application/epub+zip").unwrap();
        zip.start_file("META-INF/container.xml", options).unwrap();
        zip.write_all(
            br#"<?xml version="1.0"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#,
        )
        .unwrap();
        zip.start_file("OEBPS/content.opf", options).unwrap();
        zip.write_all(opf.as_bytes()).unwrap();
        zip.start_file("OEBPS/images/cover image.jpg", options)
            .unwrap();
        zip.write_all(b"\xFF\xD8fakejpeg").unwrap();
        zip.finish().unwrap();
        path
    }

    #[test]
    fn reads_epub2_metadata_and_cover() {
        let dir = tempfile::tempdir().unwrap();
        let path = make_epub(
            dir.path(),
            r#"<?xml version="1.0"?>
            <package xmlns="http://www.idpf.org/2007/opf" version="2.0">
              <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
                <dc:title>The Fifth Season</dc:title>
                <dc:creator opf:file-as="Jemisin, N. K." opf:role="aut" xmlns:opf="http://www.idpf.org/2007/opf">N. K. Jemisin</dc:creator>
                <dc:creator opf:role="ill" xmlns:opf="http://www.idpf.org/2007/opf">An Illustrator</dc:creator>
                <dc:publisher>Orbit</dc:publisher>
                <dc:date opf:event="modification" xmlns:opf="http://www.idpf.org/2007/opf">2020-01-01</dc:date>
                <dc:date>2015-08-04T00:00:00+00:00</dc:date>
                <dc:identifier opf:scheme="ISBN" xmlns:opf="http://www.idpf.org/2007/opf">978-0-316-22929-6</dc:identifier>
                <dc:language>en</dc:language>
                <dc:subject>Fantasy</dc:subject>
                <dc:subject>fantasy</dc:subject>
                <dc:subject>Science Fiction</dc:subject>
                <dc:description>&lt;p&gt;The world &lt;b&gt;ends&lt;/b&gt; again.&lt;/p&gt;</dc:description>
                <meta name="calibre:series" content="The Broken Earth"/>
                <meta name="calibre:series_index" content="1.0"/>
                <meta name="cover" content="cov"/>
              </metadata>
              <manifest><item id="cov" href="images/cover%20image.jpg" media-type="image/jpeg"/></manifest>
            </package>"#,
        );
        let meta = read(&path).unwrap();
        assert_eq!(meta.title.as_deref(), Some("The Fifth Season"));
        assert_eq!(meta.authors, vec!["N. K. Jemisin"]);
        assert_eq!(meta.description.as_deref(), Some("The world ends again."));
        assert_eq!(meta.series.as_deref(), Some("The Broken Earth"));
        assert_eq!(meta.series_index, Some(1.0));
        assert_eq!(
            meta.authors,
            vec!["N. K. Jemisin"],
            "illustrators aren't authors"
        );
        assert_eq!(meta.author_sort.as_deref(), Some("Jemisin, N. K."));
        assert_eq!(meta.publisher.as_deref(), Some("Orbit"));
        assert_eq!(meta.published.as_deref(), Some("2015-08-04"));
        assert_eq!(meta.isbn.as_deref(), Some("9780316229296"));
        assert_eq!(meta.language.as_deref(), Some("en"));
        assert_eq!(meta.subjects, vec!["Fantasy", "Science Fiction"]);
        let (bytes, ext) = meta.cover.unwrap();
        assert_eq!(ext, "jpg");
        assert!(bytes.starts_with(b"\xFF\xD8"));
    }

    #[test]
    fn reads_epub3_collection_and_cover_image_property() {
        let dir = tempfile::tempdir().unwrap();
        let path = make_epub(
            dir.path(),
            r##"<?xml version="1.0"?>
            <package xmlns="http://www.idpf.org/2007/opf" version="3.0">
              <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
                <dc:title>Dune Messiah</dc:title>
                <dc:creator>Frank Herbert</dc:creator>
                <meta property="belongs-to-collection" id="c1">Dune</meta>
                <meta refines="#c1" property="group-position">2</meta>
              </metadata>
              <manifest><item id="x" properties="cover-image" href="images/cover image.jpg" media-type="image/jpeg"/></manifest>
            </package>"##,
        );
        let meta = read(&path).unwrap();
        assert_eq!(meta.series.as_deref(), Some("Dune"));
        assert_eq!(meta.series_index, Some(2.0));
        assert!(meta.cover.is_some());
    }

    #[test]
    fn repairs_undeclared_prefixes() {
        let broken = r#"<package xmlns="http://www.idpf.org/2007/opf" version="3.0"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:date opf:event="modification">2018</dc:date></metadata></package>"#;
        assert!(Document::parse(broken).is_err());
        let repaired = repair_opf(broken);
        assert!(Document::parse(&repaired).is_ok());
        let fine = r#"<package xmlns:opf="x"><a opf:b="c"/></package>"#;
        assert!(matches!(repair_opf(fine), std::borrow::Cow::Borrowed(_)));
    }

    #[test]
    fn resolves_relative_hrefs() {
        assert_eq!(
            resolve("OEBPS/content.opf", "../images/a%20b.jpg"),
            "images/a b.jpg"
        );
        assert_eq!(resolve("content.opf", "cover.jpg"), "cover.jpg");
    }
}

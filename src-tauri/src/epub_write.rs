//! Writes Shelf's metadata and cover into a copy of an EPUB, for sending to an e-reader
//! (which shows what's inside the file, not what Shelf knows). The library's own file is
//! never modified.
//!
//! Only the OPF's descriptive metadata and the cover image change; every other entry is
//! copied byte for byte, and identifiers (which other parts of the book refer to) are kept.

use crate::error::{Error, Result};
use crate::model::Book;
use roxmltree::{Document, Node, ParsingOptions};
use std::fs::File;
use std::io::{Cursor, Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const DC_NS: &str = "http://purl.org/dc/elements/1.1/";
const NEW_COVER_ID: &str = "shelf-cover";
const NEW_COVER_HREF: &str = "shelf-cover.jpg";

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn read_entry(zip: &mut ZipArchive<File>, name: &str) -> Result<Vec<u8>> {
    let mut entry = zip.by_name(name)?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf)?;
    Ok(buf)
}

/// Re-encodes an image into the format of the cover it replaces, so the manifest's
/// media type stays truthful.
fn encode_as(bytes: &[u8], media_type: &str) -> Result<Vec<u8>> {
    let format = match media_type {
        "image/jpeg" | "image/jpg" => image::ImageFormat::Jpeg,
        "image/png" => image::ImageFormat::Png,
        "image/gif" => image::ImageFormat::Gif,
        "image/webp" => image::ImageFormat::WebP,
        _ => return Err(Error::msg("unsupported cover format")),
    };
    let decoded = image::load_from_memory(bytes).map_err(|e| Error::msg(e.to_string()))?;
    let decoded = if format == image::ImageFormat::Jpeg {
        image::DynamicImage::ImageRgb8(decoded.to_rgb8())
    } else {
        decoded
    };
    let mut out = Vec::new();
    decoded
        .write_to(&mut Cursor::new(&mut out), format)
        .map_err(|e| Error::msg(e.to_string()))?;
    Ok(out)
}

/// Joins an href from the OPF onto the OPF's own folder inside the archive.
fn resolve(opf_path: &str, href: &str) -> String {
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

/// One text change to the OPF: replace `start..end` with `text`.
struct Splice {
    start: usize,
    end: usize,
    text: String,
}

struct OpfEdit {
    opf: String,
    /// Cover entry to overwrite in the archive, with its media type.
    replace_cover: Option<(String, String)>,
    /// Whether a brand-new cover entry must be added next to the OPF.
    add_cover: bool,
}

/// The `prefix:` an element was written with in the source ("" when unprefixed).
fn prefix_of(text: &str, node: Node) -> String {
    let tag = &text[node.range().start + 1..];
    let name_end = tag
        .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .unwrap_or(tag.len());
    match tag[..name_end].split_once(':') {
        Some((prefix, _)) => format!("{prefix}:"),
        None => String::new(),
    }
}

fn rewrite_opf(text: &str, book: &Book, has_new_cover: bool) -> Result<OpfEdit> {
    let doc = Document::parse_with_options(
        text,
        ParsingOptions {
            allow_dtd: true,
            ..ParsingOptions::default()
        },
    )?;
    let package = doc.root_element();
    let epub3 = package
        .attribute("version")
        .is_some_and(|v| v.starts_with('3'));
    let metadata = doc
        .descendants()
        .find(|n| n.tag_name().name() == "metadata")
        .ok_or_else(|| Error::msg("The book has no metadata section."))?;
    let manifest = doc
        .descendants()
        .find(|n| n.tag_name().name() == "manifest");
    let meta_prefix = prefix_of(text, metadata);

    let children: Vec<Node> = metadata.children().filter(|n| n.is_element()).collect();
    let is_dc = |n: &Node, name: &str| {
        n.tag_name().namespace() == Some(DC_NS) && n.tag_name().name() == name
    };

    // Ids whose refinements (file-as, roles, series positions) go with the elements we replace.
    let mut dropped_ids: Vec<String> = Vec::new();
    let mut remove: Vec<Node> = Vec::new();
    for node in &children {
        let name = node.attribute("name").unwrap_or_default();
        let property = node.attribute("property").unwrap_or_default();
        let replaced = is_dc(node, "title")
            || is_dc(node, "creator")
            || is_dc(node, "description")
            || (is_dc(node, "publisher") && book.publisher.is_some())
            || (is_dc(node, "date") && book.published.is_some())
            || (is_dc(node, "language") && book.language.is_some())
            || is_dc(node, "subject")
            || matches!(
                name,
                "calibre:series"
                    | "calibre:series_index"
                    | "calibre:title_sort"
                    | "calibre:author_link_map"
            )
            || (name == "cover" && has_new_cover)
            || property == "belongs-to-collection";
        if replaced {
            if let Some(id) = node.attribute("id") {
                dropped_ids.push(id.to_string());
            }
            remove.push(*node);
        }
    }
    for node in &children {
        let refines = node.attribute("refines").map(|r| r.trim_start_matches('#'));
        if refines.is_some_and(|r| dropped_ids.iter().any(|id| id == r)) {
            remove.push(*node);
        }
    }

    // Cover: overwrite the existing image if there is one, otherwise add a new entry.
    let opf_items: Vec<Node> = manifest
        .map(|m| m.children().filter(|n| n.has_tag_name("item")).collect())
        .unwrap_or_default();
    let cover_id = children
        .iter()
        .find(|n| n.attribute("name") == Some("cover"))
        .and_then(|n| n.attribute("content"));
    let existing_cover = opf_items
        .iter()
        .find(|n| {
            n.attribute("properties")
                .is_some_and(|p| p.split_whitespace().any(|p| p == "cover-image"))
        })
        .or_else(|| {
            cover_id.and_then(|id| opf_items.iter().find(|n| n.attribute("id") == Some(id)))
        })
        .filter(|n| {
            n.attribute("media-type")
                .is_some_and(|m| m.starts_with("image/") && m != "image/svg+xml")
        });
    let replace_cover = match (has_new_cover, existing_cover) {
        (true, Some(item)) => Some((
            item.attribute("href").unwrap_or_default().to_string(),
            item.attribute("media-type").unwrap_or_default().to_string(),
        )),
        _ => None,
    };
    let add_cover = has_new_cover && replace_cover.is_none();
    // Keep the existing <meta name="cover"> when we're only overwriting the image it points at.
    if replace_cover.is_some() {
        remove.retain(|n| n.attribute("name") != Some("cover"));
    }

    // New metadata, written just before </metadata>.
    let dc = |name: &str, value: &str, extra: &str| {
        format!(
            "\n    <dc:{name} xmlns:dc=\"{DC_NS}\"{extra}>{}</dc:{name}>",
            escape(value)
        )
    };
    let meta_tag = |attrs: String, body: Option<&str>| match body {
        Some(body) => format!(
            "\n    <{meta_prefix}meta {attrs}>{}</{meta_prefix}meta>",
            escape(body)
        ),
        None => format!("\n    <{meta_prefix}meta {attrs}/>"),
    };
    let mut added = String::new();
    added.push_str(&dc("title", &book.title, ""));
    for (i, author) in book.author.split(" & ").enumerate() {
        let id = if epub3 {
            format!(" id=\"shelf-creator-{i}\"")
        } else {
            String::new()
        };
        added.push_str(&dc("creator", author.trim(), &id));
        if epub3 {
            added.push_str(&meta_tag(
                format!(
                    "refines=\"#shelf-creator-{i}\" property=\"role\" scheme=\"marc:relators\""
                ),
                Some("aut"),
            ));
        }
    }
    if let Some(sort) = book.author_sort.as_deref().filter(|_| epub3) {
        let first = sort.split(" & ").next().unwrap_or(sort);
        added.push_str(&meta_tag(
            "refines=\"#shelf-creator-0\" property=\"file-as\"".into(),
            Some(first),
        ));
    }
    if let Some(description) = &book.description {
        added.push_str(&dc("description", description, ""));
    }
    if let Some(publisher) = &book.publisher {
        added.push_str(&dc("publisher", publisher, ""));
    }
    if let Some(published) = &book.published {
        added.push_str(&dc("date", published, ""));
    }
    if let Some(language) = &book.language {
        added.push_str(&dc("language", language, ""));
    }
    for tag in &book.tags {
        added.push_str(&dc("subject", tag, ""));
    }
    if let Some(series) = &book.series {
        // calibre's form is what Kobo and most readers understand; EPUB 3 gets the standard form too.
        added.push_str(&meta_tag(
            format!("name=\"calibre:series\" content=\"{}\"", escape(series)),
            None,
        ));
        if let Some(index) = book.series_index {
            added.push_str(&meta_tag(
                format!("name=\"calibre:series_index\" content=\"{index}\""),
                None,
            ));
        }
        if epub3 {
            added.push_str(&meta_tag(
                "property=\"belongs-to-collection\" id=\"shelf-series\"".into(),
                Some(series),
            ));
            added.push_str(&meta_tag(
                "refines=\"#shelf-series\" property=\"collection-type\"".into(),
                Some("series"),
            ));
            if let Some(index) = book.series_index {
                added.push_str(&meta_tag(
                    "refines=\"#shelf-series\" property=\"group-position\"".into(),
                    Some(&index.to_string()),
                ));
            }
        }
    }
    if add_cover {
        added.push_str(&meta_tag(
            format!("name=\"cover\" content=\"{NEW_COVER_ID}\""),
            None,
        ));
    }
    added.push_str("\n  ");

    let mut splices: Vec<Splice> = remove
        .iter()
        .map(|n| Splice {
            start: n.range().start,
            end: n.range().end,
            text: String::new(),
        })
        .collect();
    let metadata_close = text[..metadata.range().end]
        .rfind("</")
        .ok_or_else(|| Error::msg("Unclosed metadata section."))?;
    splices.push(Splice {
        start: metadata_close,
        end: metadata_close,
        text: added,
    });
    if add_cover && epub3 {
        // Only one item may be the cover: take the marker off any old (e.g. SVG) cover.
        for item in opf_items.iter().filter(|n| {
            n.attribute("properties")
                .is_some_and(|p| p.split_whitespace().any(|p| p == "cover-image"))
        }) {
            let range = item.range();
            let element = &text[range.clone()];
            if let Some(attr) = element.find("properties") {
                if let Some(pos) = element[attr..].find("cover-image") {
                    let start = range.start + attr + pos;
                    splices.push(Splice {
                        start,
                        end: start + "cover-image".len(),
                        text: String::new(),
                    });
                }
            }
        }
    }
    if add_cover {
        let manifest = manifest.ok_or_else(|| Error::msg("The book has no manifest."))?;
        let manifest_prefix = prefix_of(text, manifest);
        let close = text[..manifest.range().end]
            .rfind("</")
            .ok_or_else(|| Error::msg("Unclosed manifest."))?;
        let properties = if epub3 {
            " properties=\"cover-image\""
        } else {
            ""
        };
        splices.push(Splice {
            start: close,
            end: close,
            text: format!("  <{manifest_prefix}item id=\"{NEW_COVER_ID}\" href=\"{NEW_COVER_HREF}\" media-type=\"image/jpeg\"{properties}/>\n  "),
        });
    }

    // Apply from the end so earlier offsets stay valid; removals never overlap insertions.
    splices.sort_by(|a, b| b.start.cmp(&a.start).then(b.end.cmp(&a.end)));
    let mut out = text.to_string();
    let mut last_start = usize::MAX;
    for splice in splices {
        if splice.end > last_start {
            continue; // nested inside something already removed
        }
        out.replace_range(splice.start..splice.end, &splice.text);
        last_start = splice.start;
    }
    // Make sure what we produced still parses before using it.
    Document::parse_with_options(
        &out,
        ParsingOptions {
            allow_dtd: true,
            ..ParsingOptions::default()
        },
    )?;
    Ok(OpfEdit {
        opf: out,
        replace_cover,
        add_cover,
    })
}

/// Copies `src` to `dst` with the book's metadata and (optionally) a new cover written in.
pub fn write_for_device(src: &Path, dst: &Path, book: &Book, cover: Option<&[u8]>) -> Result<()> {
    let mut zip = ZipArchive::new(File::open(src)?)?;
    let container =
        String::from_utf8_lossy(&read_entry(&mut zip, "META-INF/container.xml")?).into_owned();
    let container_doc = Document::parse(&container)?;
    let opf_path = container_doc
        .descendants()
        .find(|n| n.has_tag_name("rootfile"))
        .and_then(|n| n.attribute("full-path"))
        .ok_or_else(|| Error::msg("This EPUB has no package file."))?
        .to_string();
    let opf_text = String::from_utf8_lossy(&read_entry(&mut zip, &opf_path)?).into_owned();
    let opf_text = crate::epub::repair_opf(&opf_text);
    let edit = rewrite_opf(&opf_text, book, cover.is_some())?;

    let cover_entry = edit
        .replace_cover
        .as_ref()
        .map(|(href, media_type)| (resolve(&opf_path, href), media_type.clone()));
    let new_cover_bytes = match (&cover_entry, cover) {
        (Some((_, media_type)), Some(bytes)) => Some(encode_as(bytes, media_type)?),
        (None, Some(bytes)) if edit.add_cover => Some(encode_as(bytes, "image/jpeg")?),
        _ => None,
    };

    let mut out = ZipWriter::new(File::create(dst)?);
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for i in 0..zip.len() {
        let name = zip.by_index_raw(i)?.name().to_string();
        if name == "mimetype" {
            // The EPUB spec requires this first and uncompressed, whatever the source did.
            out.start_file(
                name,
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )?;
            out.write_all(b"application/epub+zip")?;
        } else if name == opf_path {
            out.start_file(name, deflated)?;
            out.write_all(edit.opf.as_bytes())?;
        } else if cover_entry.as_ref().is_some_and(|(path, _)| *path == name)
            && new_cover_bytes.is_some()
        {
            out.start_file(name, deflated)?;
            out.write_all(new_cover_bytes.as_deref().unwrap_or_default())?;
        } else {
            // Everything else is copied byte for byte.
            out.raw_copy_file(zip.by_index_raw(i)?)?;
        }
    }
    if edit.add_cover {
        if let Some(bytes) = &new_cover_bytes {
            out.start_file(resolve(&opf_path, NEW_COVER_HREF), deflated)?;
            out.write_all(bytes)?;
        }
    }
    out.finish()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epub;

    fn book(title: &str) -> Book {
        Book {
            id: 1,
            title: title.into(),
            author: "Daniel Abraham & Ty Franck".into(),
            series: Some("The Expanse".into()),
            series_index: Some(1.0),
            description: Some("Space noir & mystery.".into()),
            format: "epub".into(),
            size_bytes: 0,
            added_at: String::new(),
            cover_path: None,
            file_path: String::new(),
            progress: 0.0,
            status: "unread".into(),
            last_read: None,
            author_sort: Some("Abraham, Daniel & Franck, Ty".into()),
            publisher: Some("Orbit".into()),
            published: Some("2011-06-15".into()),
            isbn: None,
            language: Some("en".into()),
            rating: None,
            tags: vec!["Science Fiction".into()],
            collections: vec![],
            on_devices: vec![],
            queued_for: vec![],
        }
    }

    fn jpeg() -> Vec<u8> {
        let image = image::RgbImage::from_pixel(60, 90, image::Rgb([10, 20, 200]));
        let mut out = Vec::new();
        image::DynamicImage::ImageRgb8(image)
            .write_to(&mut Cursor::new(&mut out), image::ImageFormat::Jpeg)
            .unwrap();
        out
    }

    /// Rewrites every EPUB in a real calibre library into temporary copies and reads them
    /// back. Run with:
    /// `SHELF_TEST_CALIBRE_LIBRARY=/path/to/library cargo test real_epubs -- --ignored --nocapture`
    #[test]
    #[ignore = "needs a real calibre library in SHELF_TEST_CALIBRE_LIBRARY"]
    fn rewrites_real_epubs() {
        let Some(root) = std::env::var_os("SHELF_TEST_CALIBRE_LIBRARY") else {
            return;
        };
        let mut files = Vec::new();
        let mut dirs = vec![std::path::PathBuf::from(root)];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    dirs.push(path);
                } else if path.extension().is_some_and(|e| e == "epub") {
                    files.push(path);
                }
            }
        }
        let out_dir = tempfile::tempdir().unwrap();
        let (mut ok, mut failures) = (0, Vec::new());
        for (i, file) in files.iter().enumerate() {
            let original = std::fs::read(file).unwrap();
            let mut b = book("Shelf Test Title");
            b.series_index = Some(3.0);
            let dst = out_dir.path().join(format!("{i}.epub"));
            let result =
                write_for_device(file, &dst, &b, Some(&jpeg())).and_then(|_| epub::read(&dst));
            match result {
                Ok(meta)
                    if meta.title.as_deref() == Some("Shelf Test Title")
                        && meta.series_index == Some(3.0)
                        && meta.cover.is_some() =>
                {
                    ok += 1
                }
                Ok(meta) => failures.push(format!(
                    "{}: read back {:?} / {:?}",
                    file.display(),
                    meta.title,
                    meta.series_index
                )),
                Err(err) => failures.push(format!("{}: {err}", file.display())),
            }
            assert_eq!(std::fs::read(file).unwrap(), original, "source untouched");
            let _ = std::fs::remove_file(&dst);
        }
        println!("rewrote {ok} of {} EPUBs", files.len());
        for failure in &failures {
            println!("FAILED {failure}");
        }
        assert!(failures.is_empty());
    }

    #[test]
    fn writes_metadata_into_a_copy() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("in.epub");
        let dst = dir.path().join("out.epub");
        epub::write_test_epub(&src, "Wrong Title", "Nobody");
        let before = std::fs::read(&src).unwrap();

        write_for_device(&src, &dst, &book("Leviathan Wakes"), None).unwrap();
        assert_eq!(
            std::fs::read(&src).unwrap(),
            before,
            "the original is untouched"
        );

        let meta = epub::read(&dst).unwrap();
        assert_eq!(meta.title.as_deref(), Some("Leviathan Wakes"));
        assert_eq!(meta.authors, vec!["Daniel Abraham", "Ty Franck"]);
        assert_eq!(meta.author_sort.as_deref(), Some("Abraham, Daniel"));
        assert_eq!(meta.series.as_deref(), Some("The Expanse"));
        assert_eq!(meta.series_index, Some(1.0));
        assert_eq!(meta.description.as_deref(), Some("Space noir & mystery."));
        assert_eq!(meta.publisher.as_deref(), Some("Orbit"));
        assert_eq!(meta.published.as_deref(), Some("2011-06-15"));
        assert_eq!(meta.subjects, vec!["Science Fiction"]);

        // mimetype must stay the first entry, stored without compression.
        let mut zip = ZipArchive::new(File::open(&dst).unwrap()).unwrap();
        let first = zip.by_index(0).unwrap();
        assert_eq!(
            (first.name(), first.compression()),
            ("mimetype", CompressionMethod::Stored)
        );
    }

    #[test]
    fn adds_a_cover_when_the_book_has_none_it_can_replace() {
        // The test EPUB's only cover is an SVG, which gets a new JPEG cover alongside.
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("in.epub");
        let dst = dir.path().join("out.epub");
        epub::write_test_epub(&src, "A", "B");
        write_for_device(&src, &dst, &book("A"), Some(&jpeg())).unwrap();
        let (bytes, ext) = epub::read(&dst).unwrap().cover.unwrap();
        assert_eq!(ext, "jpg");
        assert!(bytes.starts_with(&[0xFF, 0xD8]));
        // Writing again replaces that cover in place rather than adding another.
        let again = dir.path().join("again.epub");
        write_for_device(&dst, &again, &book("A"), Some(&jpeg())).unwrap();
        let zip = ZipArchive::new(File::open(&again).unwrap()).unwrap();
        assert_eq!(
            zip.file_names()
                .filter(|n| n.ends_with("shelf-cover.jpg"))
                .count(),
            1
        );
        let opf = String::from_utf8(
            read_entry(
                &mut ZipArchive::new(File::open(&again).unwrap()).unwrap(),
                "content.opf",
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(opf.matches("cover-image").count(), 1);
    }

    #[test]
    fn replaces_an_existing_jpeg_cover_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("in.epub");
        let dst = dir.path().join("out.epub");
        {
            let mut zip = ZipWriter::new(File::create(&src).unwrap());
            let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            zip.start_file("mimetype", stored).unwrap();
            zip.write_all(b"application/epub+zip").unwrap();
            zip.start_file("META-INF/container.xml", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(br#"<?xml version="1.0"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OPS/book.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#).unwrap();
            zip.start_file("OPS/book.opf", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(br#"<?xml version="1.0"?><opf:package xmlns:opf="http://www.idpf.org/2007/opf" version="2.0" unique-identifier="uid"><opf:metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:identifier id="uid">urn:uuid:1</dc:identifier><dc:title>Old</dc:title><dc:creator>Old Author</dc:creator><dc:language>fr</dc:language><opf:meta name="cover" content="img"/><opf:meta name="calibre:series" content="Old Series"/></opf:metadata><opf:manifest><opf:item id="img" href="images/c.jpg" media-type="image/jpeg"/></opf:manifest></opf:package>"#).unwrap();
            zip.start_file("OPS/images/c.jpg", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(&jpeg()).unwrap();
            zip.finish().unwrap();
        }
        let mut b = book("New");
        b.language = None;
        write_for_device(&src, &dst, &b, Some(&jpeg())).unwrap();
        let mut zip = ZipArchive::new(File::open(&dst).unwrap()).unwrap();
        let opf = String::from_utf8(read_entry(&mut zip, "OPS/book.opf").unwrap()).unwrap();
        assert!(
            opf.contains("<dc:identifier id=\"uid\">urn:uuid:1</dc:identifier>"),
            "identifiers are kept"
        );
        assert!(
            opf.contains("<dc:language>fr</dc:language>"),
            "language kept when Shelf has none"
        );
        assert!(!opf.contains("Old Series") && !opf.contains(">Old<"));
        assert!(
            opf.contains("<opf:meta name=\"calibre:series\" content=\"The Expanse\"/>"),
            "uses the file's prefix"
        );
        assert_eq!(opf.matches("name=\"cover\"").count(), 1);
        assert!(
            zip.file_names().all(|n| !n.contains("shelf-cover")),
            "existing cover replaced, not added"
        );
        let meta = epub::read(&dst).unwrap();
        assert_eq!(meta.title.as_deref(), Some("New"));
        assert!(meta.cover.is_some());
    }
}

//! Converts EPUB to kepub, the flavour of EPUB that Kobo's own reading engine uses.
//!
//! A kepub is an ordinary EPUB whose content documents have:
//! - every sentence (and image) wrapped in `<span class="koboSpan" id="kobo.P.S">`, which
//!   Kobo uses for reading position, page and time-left stats, and highlights;
//! - the body's content wrapped in `<div id="book-columns"><div id="book-inner">`, which
//!   Kobo's renderer lays pages out with;
//! - a small style fix so the wrapper doesn't add margins.
//!
//! Text is never changed: only those elements are inserted. A document that fails to
//! convert cleanly is left exactly as it was.

use crate::error::{Error, Result};
use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use roxmltree::{Document, ParsingOptions};
use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// Elements whose text Kobo never needs spans for.
const SKIP: &[&str] = &[
    "head", "script", "style", "svg", "math", "noscript", "textarea", "template",
];

/// Elements that start a new "paragraph" in Kobo's span numbering.
const BLOCKS: &[&str] = &[
    "p",
    "div",
    "li",
    "dt",
    "dd",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "blockquote",
    "td",
    "th",
    "caption",
    "figcaption",
    "pre",
    "section",
    "article",
    "aside",
    "header",
    "footer",
    "table",
    "ol",
    "ul",
];

const STYLE_HACKS: &str = "div#book-inner { margin-top: 0; margin-bottom: 0; }";

/// Entities that can close a sentence after its full stop (closing quotes and the like).
const CLOSING_ENTITIES: &[&str] = &[
    "quot", "apos", "rdquo", "rsquo", "raquo", "#8221", "#8217", "#187", "#x201d", "#x2019", "#xbb",
];

/// Splits escaped text into sentences. Each piece keeps its trailing whitespace, so the
/// pieces join back into exactly the original text.
pub fn split_sentences(text: &str) -> Vec<&str> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut pieces = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < chars.len() {
        if !matches!(chars[i].1, '.' | '!' | '?' | '…') {
            i += 1;
            continue;
        }
        // Take any further punctuation and closing quotes or brackets.
        let mut j = i + 1;
        loop {
            if j < chars.len()
                && matches!(
                    chars[j].1,
                    '.' | '!' | '?' | '…' | '"' | '\'' | '”' | '’' | '»' | ')' | ']'
                )
            {
                j += 1;
                continue;
            }
            if j < chars.len() && chars[j].1 == '&' {
                let rest = &text[chars[j].0..];
                if let Some(end) = rest.find(';').filter(|end| *end <= 10) {
                    if CLOSING_ENTITIES.contains(&rest[1..end].to_lowercase().as_str()) {
                        j += rest[..=end].chars().count();
                        continue;
                    }
                }
            }
            break;
        }
        // It's only the end of a sentence if whitespace follows.
        if j < chars.len() && chars[j].1.is_whitespace() {
            while j < chars.len() && chars[j].1.is_whitespace() {
                j += 1;
            }
            let end = chars.get(j).map(|(pos, _)| *pos).unwrap_or(text.len());
            pieces.push(&text[start..end]);
            start = end;
        }
        i = j;
    }
    if start < text.len() {
        pieces.push(&text[start..]);
    }
    pieces
}

fn write(out: &mut Writer<Vec<u8>>, event: Event<'_>) -> Result<()> {
    out.write_event(event)
        .map_err(|e| Error::msg(e.to_string()))
}

fn local_name(raw: &[u8]) -> String {
    let name = raw.split(|b| *b == b':').next_back().unwrap_or(raw);
    String::from_utf8_lossy(name).to_lowercase()
}

struct Spans {
    paragraph: u32,
    segment: u32,
}

impl Spans {
    fn next_id(&mut self) -> String {
        self.segment += 1;
        format!("kobo.{}.{}", self.paragraph, self.segment)
    }

    fn open(&mut self) -> Event<'static> {
        let id = self.next_id();
        Event::Start(
            BytesStart::new("span").with_attributes([("class", "koboSpan"), ("id", id.as_str())]),
        )
    }
}

/// Converts one XHTML content document. Returns `None` if it's already a kepub document.
pub fn convert_xhtml(text: &str) -> Result<Option<String>> {
    if text.contains("koboSpan") {
        return Ok(None);
    }
    let mut reader = Reader::from_str(text);
    reader.config_mut().check_end_names = false;
    reader.config_mut().allow_unmatched_ends = true;
    let mut out = Writer::new(Vec::with_capacity(text.len() + text.len() / 4));
    let mut spans = Spans {
        paragraph: 0,
        segment: 0,
    };
    let mut skip_depth = 0usize;
    let mut in_body = false;

    loop {
        let event = reader
            .read_event()
            .map_err(|e| Error::msg(format!("couldn't read the page: {e}")))?;
        match event {
            Event::Eof => break,
            Event::Start(start) => {
                let name = local_name(start.name().as_ref());
                if SKIP.contains(&name.as_str()) {
                    skip_depth += 1;
                }
                if in_body && skip_depth == 0 && BLOCKS.contains(&name.as_str()) {
                    spans.paragraph += 1;
                    spans.segment = 0;
                }
                write(&mut out, Event::Start(start))?;
                if name == "body" {
                    in_body = true;
                    write(
                        &mut out,
                        Event::Start(
                            BytesStart::new("div").with_attributes([("id", "book-columns")]),
                        ),
                    )?;
                    write(
                        &mut out,
                        Event::Start(
                            BytesStart::new("div").with_attributes([("id", "book-inner")]),
                        ),
                    )?;
                }
            }
            Event::End(end) => {
                let name = local_name(end.name().as_ref());
                if name == "head" {
                    write(
                        &mut out,
                        Event::Start(
                            BytesStart::new("style").with_attributes([
                                ("type", "text/css"),
                                ("class", "kobostylehacks"),
                            ]),
                        ),
                    )?;
                    write(&mut out, Event::Text(BytesText::from_escaped(STYLE_HACKS)))?;
                    write(&mut out, Event::End(BytesEnd::new("style")))?;
                }
                if name == "body" && in_body {
                    write(&mut out, Event::End(BytesEnd::new("div")))?;
                    write(&mut out, Event::End(BytesEnd::new("div")))?;
                    in_body = false;
                }
                if SKIP.contains(&name.as_str()) {
                    skip_depth = skip_depth.saturating_sub(1);
                }
                write(&mut out, Event::End(end))?;
            }
            Event::Empty(empty) => {
                let name = local_name(empty.name().as_ref());
                if in_body && skip_depth == 0 && (name == "img" || name == "image") {
                    write(&mut out, spans.open())?;
                    write(&mut out, Event::Empty(empty))?;
                    write(&mut out, Event::End(BytesEnd::new("span")))?;
                } else {
                    write(&mut out, Event::Empty(empty))?;
                }
            }
            Event::Text(text) if in_body && skip_depth == 0 => {
                let raw =
                    std::str::from_utf8(text.as_ref()).map_err(|e| Error::msg(e.to_string()))?;
                if raw.trim().is_empty() {
                    write(&mut out, Event::Text(text))?;
                    continue;
                }
                for sentence in split_sentences(raw) {
                    write(&mut out, spans.open())?;
                    write(&mut out, Event::Text(BytesText::from_escaped(sentence)))?;
                    write(&mut out, Event::End(BytesEnd::new("span")))?;
                }
            }
            other => write(&mut out, other)?,
        }
    }
    String::from_utf8(out.into_inner())
        .map(Some)
        .map_err(|e| Error::msg(e.to_string()))
}

fn read_entry(zip: &mut ZipArchive<File>, name: &str) -> Result<Vec<u8>> {
    let mut entry = zip.by_name(name)?;
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf)?;
    Ok(buf)
}

fn parses(text: &str) -> bool {
    Document::parse_with_options(
        text,
        ParsingOptions {
            allow_dtd: true,
            ..ParsingOptions::default()
        },
    )
    .is_ok()
}

/// Statistics from a conversion, mostly for tests.
#[derive(Debug, Default)]
pub struct Report {
    pub converted: usize,
    pub left_alone: usize,
}

/// Writes a kepub copy of `src` to `dst`.
pub fn convert(src: &Path, dst: &Path) -> Result<Report> {
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
    let opf = Document::parse_with_options(
        &opf_text,
        ParsingOptions {
            allow_dtd: true,
            ..ParsingOptions::default()
        },
    )?;
    let pages: HashSet<String> = opf
        .descendants()
        .filter(|n| n.tag_name().name() == "item")
        .filter(|n| {
            matches!(
                n.attribute("media-type"),
                Some("application/xhtml+xml" | "text/html")
            )
        })
        .filter_map(|n| n.attribute("href"))
        .map(|href| crate::epub::resolve(&opf_path, href))
        .collect();

    let mut report = Report::default();
    let mut out = ZipWriter::new(File::create(dst)?);
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for i in 0..zip.len() {
        let name = zip.by_index_raw(i)?.name().to_string();
        if name == "mimetype" {
            out.start_file(
                name,
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )?;
            out.write_all(b"application/epub+zip")?;
            continue;
        }
        if pages.contains(&name) {
            let bytes = read_entry(&mut zip, &name)?;
            let converted = std::str::from_utf8(&bytes).ok().and_then(|text| {
                let result = convert_xhtml(text).ok().flatten()?;
                // Never make a page worse: if it was valid XML, the result must be too.
                (!parses(text) || parses(&result)).then_some(result)
            });
            match converted {
                Some(result) => {
                    out.start_file(name, deflated)?;
                    out.write_all(result.as_bytes())?;
                    report.converted += 1;
                }
                None => {
                    out.start_file(name, deflated)?;
                    out.write_all(&bytes)?;
                    report.left_alone += 1;
                }
            }
            continue;
        }
        out.raw_copy_file(zip.by_index_raw(i)?)?;
    }
    out.finish()?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_of(xml: &str) -> String {
        let doc = Document::parse_with_options(
            xml,
            ParsingOptions {
                allow_dtd: true,
                ..ParsingOptions::default()
            },
        )
        .unwrap();
        let body = doc.descendants().find(|n| n.has_tag_name("body")).unwrap();
        body.descendants()
            .filter(|n| n.is_text())
            .filter_map(|n| n.text())
            .collect()
    }

    /// Converts every EPUB in a real calibre library and checks each page: it must still
    /// parse (if it did before), keep exactly the same text, and gain Kobo spans. Run with:
    /// `SHELF_TEST_CALIBRE_LIBRARY=/path/to/library cargo test --release real_kepub -- --ignored --nocapture`
    #[test]
    #[ignore = "needs a real calibre library in SHELF_TEST_CALIBRE_LIBRARY"]
    fn real_kepub_conversion() {
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
        let started = std::time::Instant::now();
        let (mut books, mut pages, mut left_alone, mut spans) = (0, 0, 0, 0usize);
        let mut problems = Vec::new();
        for (i, file) in files.iter().enumerate() {
            let dst = out_dir.path().join(format!("{i}.kepub.epub"));
            let report = match convert(file, &dst) {
                Ok(report) => report,
                Err(err) => {
                    problems.push(format!("{}: {err}", file.display()));
                    continue;
                }
            };
            books += 1;
            pages += report.converted;
            left_alone += report.left_alone;
            let mut before = ZipArchive::new(File::open(file).unwrap()).unwrap();
            let mut after = ZipArchive::new(File::open(&dst).unwrap()).unwrap();
            let names: Vec<String> = after.file_names().map(str::to_string).collect();
            for name in names
                .iter()
                .filter(|n| n.ends_with(".xhtml") || n.ends_with(".html") || n.ends_with(".htm"))
            {
                let old =
                    String::from_utf8_lossy(&read_entry(&mut before, name).unwrap()).into_owned();
                let new =
                    String::from_utf8_lossy(&read_entry(&mut after, name).unwrap()).into_owned();
                spans += new.matches("koboSpan").count();
                if parses(&old) {
                    if !parses(&new) {
                        problems.push(format!("{} / {name}: no longer valid", file.display()));
                    } else if text_of(&old) != text_of(&new) {
                        problems.push(format!("{} / {name}: text changed", file.display()));
                    }
                }
            }
            let _ = std::fs::remove_file(&dst);
        }
        println!(
            "converted {books} of {} books in {:.1?}: {pages} pages converted, {left_alone} left alone, {spans} Kobo spans",
            files.len(),
            started.elapsed()
        );
        for problem in problems.iter().take(20) {
            println!("PROBLEM {problem}");
        }
        assert!(problems.is_empty(), "{} problems", problems.len());
    }

    #[test]
    fn splits_sentences_without_losing_text() {
        let text = "It was late. \u{201c}Who&rsquo;s there?&rdquo; she asked!  Nobody answered... Then 3.5 seconds";
        let pieces = split_sentences(text);
        assert_eq!(pieces.concat(), text);
        assert_eq!(
            pieces,
            vec![
                "It was late. ",
                "\u{201c}Who&rsquo;s there?&rdquo; ",
                "she asked!  ",
                "Nobody answered... ",
                "Then 3.5 seconds"
            ]
        );
        assert_eq!(split_sentences("No full stop"), vec!["No full stop"]);
    }

    #[test]
    fn converts_a_page() {
        let page = r#"<?xml version="1.0" encoding="utf-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml"><head><title>Ch 1</title><style>p { margin: 0 }</style></head>
<body><h1>Chapter One</h1><p>First sentence. Second <em>one</em> here!</p>
<p><img src="a.jpg" alt=""/></p><svg xmlns="http://www.w3.org/2000/svg"><text>Skip me.</text></svg></body></html>"#;
        let out = convert_xhtml(page).unwrap().unwrap();
        assert!(out.contains(r#"<body><div id="book-columns"><div id="book-inner"><h1><span class="koboSpan" id="kobo.1.1">Chapter One</span></h1>"#));
        assert!(out.contains(r#"<p><span class="koboSpan" id="kobo.2.1">First sentence. </span><span class="koboSpan" id="kobo.2.2">Second </span><em><span class="koboSpan" id="kobo.2.3">one</span></em><span class="koboSpan" id="kobo.2.4"> here!</span></p>"#));
        assert!(
            out.contains(
                r#"<span class="koboSpan" id="kobo.3.1"><img src="a.jpg" alt=""/></span>"#
            )
        );
        assert!(out.contains("<text>Skip me.</text>"), "no spans inside SVG");
        assert!(out.contains(r#"<style type="text/css" class="kobostylehacks">"#));
        assert!(out.contains("</div></div></body>"));
        assert_eq!(
            text_of(&out).replace(STYLE_HACKS, ""),
            text_of(page),
            "text is unchanged"
        );
        assert!(
            convert_xhtml(&out).unwrap().is_none(),
            "already-converted pages are left alone"
        );
    }

    #[test]
    fn keeps_undeclared_entities_intact() {
        let page = "<html><body><p>A&nbsp;B. C&mdash;D.</p></body></html>";
        let out = convert_xhtml(page).unwrap().unwrap();
        assert!(out.contains(r#"<span class="koboSpan" id="kobo.1.1">A&nbsp;B. </span><span class="koboSpan" id="kobo.1.2">C&mdash;D.</span>"#));
    }

    #[test]
    fn converts_an_epub() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("in.epub");
        let dst = dir.path().join("out.kepub.epub");
        {
            let mut zip = ZipWriter::new(File::create(&src).unwrap());
            let stored = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
            zip.start_file("mimetype", stored).unwrap();
            zip.write_all(b"application/epub+zip").unwrap();
            zip.start_file("META-INF/container.xml", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(br#"<?xml version="1.0"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/c.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#).unwrap();
            zip.start_file("OEBPS/c.opf", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(br#"<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="2.0"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>T</dc:title></metadata><manifest><item id="p1" href="text/ch%201.xhtml" media-type="application/xhtml+xml"/><item id="css" href="s.css" media-type="text/css"/></manifest><spine><itemref idref="p1"/></spine></package>"#).unwrap();
            zip.start_file("OEBPS/text/ch 1.xhtml", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(br#"<html xmlns="http://www.w3.org/1999/xhtml"><head><title>x</title></head><body><p>Hello there. Bye.</p></body></html>"#).unwrap();
            zip.start_file("OEBPS/s.css", SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"p { margin: 0 }").unwrap();
            zip.finish().unwrap();
        }
        let report = convert(&src, &dst).unwrap();
        assert_eq!((report.converted, report.left_alone), (1, 0));
        let mut zip = ZipArchive::new(File::open(&dst).unwrap()).unwrap();
        let page =
            String::from_utf8(read_entry(&mut zip, "OEBPS/text/ch 1.xhtml").unwrap()).unwrap();
        assert!(page.contains(r#"id="kobo.1.2">Bye.</span>"#));
        assert_eq!(
            read_entry(&mut zip, "OEBPS/s.css").unwrap(),
            b"p { margin: 0 }"
        );
        let first = zip.by_index(0).unwrap();
        assert_eq!(
            (first.name(), first.compression()),
            ("mimetype", CompressionMethod::Stored)
        );
    }
}

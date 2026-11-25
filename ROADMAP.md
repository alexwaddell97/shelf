# Shelf roadmap

Shelf's aim is to be the ebook manager you'd pick over calibre if you mostly care about **your library and your e-reader**: fast, good-looking, nothing to configure, no plugins to hunt down. It stays local-first: a desktop app that works offline and talks to your e-reader over USB.

This page compares Shelf with calibre and the other tools people use alongside it, then sets out what comes next. Priorities are a guide, not a promise. Issues and pull requests are welcome.

## Where Shelf already does better

- **Kobo without plugins.** Native kepub conversion, metadata and covers written into the copy on the device, books calibre already put there linked automatically, and reading progress and finished status read back. With calibre, that needs the Kobo Utilities and kepubify plugins.
- **A modern, fast interface.** Bookshelf, grid and list views, instant search with highlighted suggestions, and a Ctrl+K command palette.
- **Watched folders.** A book that lands in Downloads goes straight into the library. calibre needs an add-on for this.
- **Wishlist and "Find elsewhere".** Wanted books (including your Goodreads want-to-read shelf, from its library export) become real books when you add the file, and each catalogue page links to legal sources.
- **A built-in reader** that opens each book in its own window and remembers your place.

## Gap analysis

What calibre and other tools offer that Shelf doesn't yet. **Value** is to someone managing books for an e-reader.

| Feature | calibre | Other tools | Shelf today | Value |
|---|---|---|---|---|
| Download metadata and covers (Open Library, Google Books, Hardcover…) | Yes | Calibre-Web Automated, BookLore | Open Library, with a field-by-field preview and bulk "fill in what's missing" (0.2) | Done |
| Duplicate detection and merge | Plugin (Find Duplicates) | Calibre-Web Automated | Same ISBN or same title and author, merged keeping everything (0.2) | Done |
| Bulk metadata editing | Yes | Calibre-Web Automated | Author, series with numbering, tags, collections, rating (0.2) | Done |
| Smart shelves and saved searches (`author:`, `rating:>3`…) | Virtual libraries, search syntax | "Magic shelves" in CWA and BookLore | Filters, no saved searches | **High** |
| Kobo highlights and annotations export | Plugin | Readwise (Kobo store books only) | No | **High** |
| Kindle (USB and Send to Kindle email) | Yes | CWA (email) | No | **High** |
| Two-way reading-service sync (Hardcover, Goodreads, StoryGraph) | Plugin | CWA, KOReader plugins | Goodreads export import (its feeds were retired in Nov 2026) | **High** |
| Kobo collections from series, tags and shelves | Plugin | CWA, Komga | No | Medium–high |
| Reading stats (time, pages, books per year) | No | KOReader, KoInsight | Progress only | Medium–high |
| Format conversion (EPUB ⇄ AZW3/MOBI, PDF/DOCX → EPUB) | Its core strength | CWA (via calibre) | EPUB → kepub only | Medium–high |
| EPUB repair and quick fixes | Full editor | CWA "EPUB fixer" | Repairs some broken files on import | Medium |
| Backup and restore, readable files on disk | Metadata backup | CWA daily backups | No | Medium |
| Custom fields | Custom columns | Rare | No | Medium |
| Page counts and reading-time estimates | Plugin (Count Pages) | KOReader | No | Medium |
| OPDS catalogue and LAN access for phones and KOReader | Content server | Calibre-Web, CWA, Komga, Kavita | No | Medium |
| Wireless Kobo sync (Kobo store emulation) | No | Calibre-Web, CWA, Komga | No | Low–medium: it needs a server |
| News download, multi-user, comics | Yes / no / basic | Komga and Kavita for comics | No | Low |

**What people dislike about calibre**, and what Shelf should keep getting right:

- a cluttered, dated interface with settings scattered across many dialogs;
- slow starts and large libraries;
- core features needing plugins that break between versions;
- e-reader sync that's fragile after firmware updates;
- reading progress from the device not shown without plugins.

The research behind this table comes partly from the projects' own pages and partly from general knowledge of these tools. Treat the "other tools" column as a starting point and check it before relying on it.

## Roadmap

### 0.2: trust and tidying

Make a large library easy to clean up, and make Shelf safe to rely on. See the [changelog](CHANGELOG.md).

Done in 0.2.0:

- **Metadata and covers** from Open Library, with a field-by-field preview in the editor and "fill in what's missing" for many books. Google Books was tried, but without an API key its shared free quota runs out daily; an optional key of your own may come later.
- **Duplicate finder.** Same ISBN, or the same whole title and author (a different subtitle means a different book), merged keeping every copy's tags, collections, progress and details.
- **Bulk edit.** Multi-select in the list, then set author, series (numbered in order), tags, collections or rating.
- **macOS builds** for Apple Silicon and Intel, from GitHub Actions.
- **Auto-update** with signed updates.

Still to come in 0.2.x:

- **Backup and restore.** One-click export of the library and settings, plus an OPF sidecar next to each book so the library stays readable without Shelf.
- **Code signing** for Windows (or a Microsoft Store listing) and macOS (an Apple Developer ID), to remove the "unknown publisher" warnings.
- **Find and replace** in bulk edit.

### 0.3: your reading life

Use what's already on the e-reader.

- **Highlights and annotations** read from the Kobo, sideloaded books included. Show them per book, and export them to Markdown, CSV and Readwise.
- **Reading stats**: books finished, time read and a yearly goal, from the Kobo's own records.
- **Hardcover sync**: want-to-read, status and rating, both ways, through Hardcover's API. This matters more now: Goodreads retired its shelf feeds in November 2026, so Goodreads is import-only (from its library export).
- **Kobo collections** generated from series, tags or Shelf collections.
- **Saved searches and smart shelves**, with a calibre-style search syntax.

### 0.4: more devices and formats

- **Kindle**: send over USB, plus Send to Kindle by email, with automatic fixes for the EPUB problems Amazon rejects.
- **Generic e-readers** (PocketBook, Tolino, anything that mounts as a drive) through a simple folder driver.
- **Conversion** between EPUB and AZW3/MOBI, and from PDF or DOCX to EPUB at basic quality.
- **Quick EPUB fixes**: validate, repair common errors, and edit the table of contents.

### Later

- An OPDS catalogue and optional LAN access, so phones and KOReader can browse the library.
- Custom fields (text, number, yes/no, choice), shown in views and search.
- Page counts and reading-time estimates.
- A KOReader progress-sync endpoint.

### Not planned

- **DRM removal or piracy sources.** Shelf won't ship either. "Find elsewhere" links only to legal stores and free libraries, and you can add your own links.
- **Automated downloading of books.**
- **News recipes, multi-user accounts and comics.** Other tools do these well.

## Recently fixed

From a round of review and testing (October 2026):

- **Crashes and freezes**
  - The app closed when importing a book whose date doesn't start with a plain year (e.g. "Été 2019").
  - A book page froze when one author was listed twice ("Cixin Liu & Liu, Cixin"). Shelf now shows an error page instead of freezing if a page ever fails.
  - The right-click menu could freeze with two e-readers of the same name, and the Authors page could break on accented initials.
- **E-reader safety**
  - Two editions of the same book no longer overwrite each other on the e-reader.
  - A plain EPUB can no longer be written over a kepub.
  - Re-sending a book no longer risks losing the copy if the e-reader fills up or is unplugged part-way.
  - Unplugging during a sync no longer makes Shelf forget which books are on the device.
- **Imports and data**
  - Imports of the same file at once can't create duplicates, and failed imports clean up after themselves.
  - Database upgrades are all-or-nothing, and settings added in future versions won't stop older installs from loading.
- **Interface**
  - The metadata editor no longer loses changes to a stray click or Escape.
  - The Wishlist screen stays fast with large libraries.
  - Settings changes can't overtake each other.
  - The header search clears properly, and import progress can't get stuck.

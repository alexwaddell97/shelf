# Shelf

A fast, modern ebook library for your computer that keeps your e-reader in sync.

Drop in your EPUBs, and Shelf reads their covers and details, files them neatly, converts them for your device and copies them over. When you plug your e-reader back in, your reading progress comes back. Free, open source and local: your library never leaves your computer.

> **Status: early development (v0.1).** The Kobo driver is the first one; other e-readers are on the [roadmap](ROADMAP.md). Expect rough edges, and please report them.

## Features

- **Library** — cover grid, instant search, filters by reading status and device, and sorting by title, author (by surname), series order, publication date or rating. Browse by author, series, collection or tag.
- **Full metadata editing** — title, authors and sort name, series and number, rating, collections, tags, publisher, date, language, ISBN and description, with Previous/Next to step through books. Covers can come from an image file or Open Library's cover search. Your edits and covers are written into the copy sent to your e-reader (your library file is left as it is), and books already on the device are updated on the next sync.
- **Open ebooks with Shelf** — once installed and chosen as the app for `.epub`, `.mobi` or `.azw3`, double-clicking a book adds it to your library and shows it.
- **Import anything you drop** — drag files onto the window, use *Add books*, or let Shelf watch a folder (your Downloads folder by default) and import new ebooks as they arrive. Duplicates are skipped.
- **E-reader sync** — Shelf spots your e-reader when you plug it in over USB, sends queued books in the best format for it, and reads reading progress back. For Kobo, EPUBs are converted to kepub, Kobo's own format, for faster page turns, reading stats and better typography. The conversion is built in, so there's nothing extra to install.
- **Find new books** — search the [Open Library](https://openlibrary.org) catalogue from the same search box. Public-domain books from [Project Gutenberg](https://www.gutenberg.org) can be added to your library in one click; for everything else, Shelf shows links to search stores and libraries for that book.
- **Wishlist** — save books you don't have yet. When a matching file arrives, it's matched automatically.
- **Your own sources** — add any shop or library that has a search page, using a URL template like `https://example.com/search?q={query}`.
- **Bring your calibre library** — Shelf finds your calibre library and imports it with calibre's titles, authors, series, descriptions and covers. calibre's files are only read, never changed, so you can keep using both.
- **Goodreads** — connect your public profile and your *Want to Read* shelf fills your wishlist, while *Read* and *Currently Reading* mark books in your library. It's one-way and needs no password (Goodreads closed its API to new apps, so Shelf reads the public shelf feeds). You can also import a Goodreads CSV export.

## E-reader support

| Device | Status | Sends |
| --- | --- | --- |
| Kobo (Clara, Libra, Sage, Elipsa, …) | Supported | kepub (built in) |
| PocketBook | Planned | EPUB |
| Tolino | Planned | EPUB |
| Kindle | Planned (needs calibre for conversion) | AZW3 |
| Boox and other Android readers | Planned | EPUB |
| KOReader | Planned | EPUB |

Drivers live in [`src-tauri/src/devices/`](src-tauri/src/devices). Adding one is a great first contribution: see [CONTRIBUTING.md](CONTRIBUTING.md).

## Privacy

Shelf has no accounts, no telemetry and no analytics. It only goes online when you search for books (Open Library and Gutendex, a Project Gutenberg API), download a free book from Project Gutenberg, open a store link in your browser, or — if you connect Goodreads — read your public shelf feeds once an hour.

## Optional tools

- **calibre** — if installed, its `ebook-convert` will be used for formats Shelf can't convert itself (coming with the Kindle driver).

## Building from source

Prerequisites: [Rust](https://rustup.rs) (stable), [Node.js](https://nodejs.org) 22 or newer, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS (WebView2 and the C++ build tools on Windows, WebKitGTK on Linux, Xcode command-line tools on macOS).

```bash
npm install
npm run tauri dev      # run with hot reload
npm run tauri build    # make an installer for your OS
```

Run the checks before sending a pull request:

```bash
npm run check
cargo test --manifest-path src-tauri/Cargo.toml
```

## Where things are stored

- **Your books**: `Documents/Shelf Library/<Author>/<Title>.epub` by default.
- **Library database, covers and device backups**: the app data folder (`%APPDATA%\org.shelfapp.shelf` on Windows, `~/Library/Application Support/org.shelfapp.shelf` on macOS, `~/.local/share/org.shelfapp.shelf` on Linux).
- **On a Kobo**: books go into a `Shelf` folder on the device. Before each sync, a copy of the device's `KoboReader.sqlite` is saved in the app data folder under `device-backups/`.

## Licence

Shelf is free software, released under the [GNU General Public License v3.0 or later](LICENSE).

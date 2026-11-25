# Shelf

A fast, modern ebook library for your computer that keeps your e-reader in sync.

Drop in your EPUBs, and Shelf reads their covers and details, files them neatly, converts them for your device and copies them over. When you plug your e-reader back in, your reading progress comes back. Free, open source and local: your library never leaves your computer.

> **Status: early development (v0.1).** The Kobo driver is the first one; other e-readers are on the [roadmap](ROADMAP.md). Expect rough edges, and please report them.

## Download

Get Shelf for Windows or macOS from [itch.io](https://alexwaddell97.itch.io/shelf), where you can pay what you like (or nothing), or from [GitHub Releases](https://github.com/alexwaddell97/shelf/releases). Shelf tells you when a new version is out and installs it for you.

The downloads aren't code-signed yet:

- **Windows** may show "Windows protected your PC" the first time. Choose **More info**, then **Run anyway**.
- **macOS** may say Shelf "can't be opened" or is from an unidentified developer. Open **System Settings → Privacy & Security** and choose **Open Anyway** beside the message about Shelf. The Mac version is new: please report anything that doesn't work.

## Support Shelf

Shelf is free and always will be. If it saves you time, [paying what you like on itch.io](https://alexwaddell97.itch.io/shelf) keeps new versions coming. Bug reports and pull requests help just as much.

## Features

- **Library** — bookshelf, cover grid or a sortable table with the columns you choose; instant search; filters by reading status and device; sorting either way by title, author (by surname), series order, date, rating, progress and more. Browse by author, series, collection or tag.
- **Bulk editing** — select books in the list (Ctrl/Cmd-click, Shift-click, Ctrl/Cmd+A) to set the author or series (numbered in order), add or remove tags and collections, set a rating, fill in missing details, or send them all to your e-reader.
- **Details from the web** — look a book up on Open Library (by ISBN, or title and author) and pick which fields to take; or fill in only the missing details and covers for many books at once.
- **Duplicate finder** — finds books you have more than one copy of (same ISBN, or the same title and author) and merges them, keeping the tags, collections, reading progress and details of every copy.
- **Full metadata editing** — title, authors and sort name, series and number, rating, collections, tags, publisher, date, language, ISBN and description, with Previous/Next to step through books. Covers can come from an image file or Open Library's cover search. Your edits and covers are written into the copy sent to your e-reader (your library file is left as it is), and books already on the device are updated on the next sync.
- **Open ebooks with Shelf** — once installed and chosen as the app for `.epub`, `.mobi` or `.azw3`, double-clicking a book adds it to your library and shows it.
- **Import anything you drop** — drag files onto the window, use *Add books*, or let Shelf watch a folder (your Downloads folder by default) and import new ebooks as they arrive. Duplicates are skipped.
- **E-reader sync** — Shelf spots your e-reader when you plug it in over USB, sends queued books in the best format for it, and reads reading progress back. For Kobo, EPUBs are converted to kepub, Kobo's own format, for faster page turns, reading stats and better typography. The conversion is built in, so there's nothing extra to install.
- **Find new books** — search the [Open Library](https://openlibrary.org) catalogue from the same search box. Public-domain books from [Project Gutenberg](https://www.gutenberg.org) can be added to your library in one click; for everything else, Shelf shows links to search stores and libraries for that book.
- **Wishlist** — save books you don't have yet. When a matching file arrives, it's matched automatically.
- **Your own sources** — add any shop or library that has a search page, using a URL template like `https://example.com/search?q={query}`.
- **Bring your calibre library** — Shelf finds your calibre library and imports it with calibre's titles, authors, series, descriptions and covers. calibre's files are only read, never changed, so you can keep using both.
- **Goodreads** — import your Goodreads library export and your *Want to Read* shelf fills your wishlist, while *Read* and *Currently Reading* mark books in your library. Import a fresh export any time to bring Shelf up to date. It's one-way and needs no password. (Goodreads closed its API in 2020 and retired its shelf feeds in November 2026, so the export is the only way in.)

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

Shelf has no accounts, no telemetry and no analytics. It only goes online when you search for books (Open Library and Gutendex, a Project Gutenberg API), look up a book's details (Open Library, sending its title, author and ISBN), download a free book from Project Gutenberg, or open a store link in your browser. A Goodreads import only reads the file you choose. It also checks for a new version of Shelf when it starts and every few hours, by downloading a small file from this project's latest GitHub release; nothing about you or your library is sent.

## Optional tools

- **calibre** — if installed, its `ebook-convert` will be used for formats Shelf can't convert itself (coming with the Kindle driver).

## Building from source

Prerequisites: [Rust](https://rustup.rs) (stable), [Node.js](https://nodejs.org) 22 or newer, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS (WebView2 and the C++ build tools on Windows, WebKitGTK on Linux, Xcode command-line tools on macOS).

```bash
npm install
npm run tauri dev      # run with hot reload
npm run tauri build    # make an installer for your OS
```

To release for every platform, push a version tag (`git tag v0.2.0 && git push origin v0.2.0`): GitHub Actions builds Windows, macOS and Linux installers into a draft release, with `latest.json` for updates. It needs the updater key as the repository secret `TAURI_SIGNING_PRIVATE_KEY` (plus `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, which can be empty). For a Windows-only build on your own PC, run `npm run release` (add `-- --notes-file notes.md` for release notes). It builds a signed installer and the `latest.json` file that installed copies check for updates, then prints the commands to publish them. Signing needs the private updater key, kept outside the repository (by default in `~/.tauri/shelf-updater.key`); without it, copies already out there can't be sent updates.

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

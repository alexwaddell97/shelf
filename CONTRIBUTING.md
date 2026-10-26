# Contributing to Shelf

Thanks for helping. Bug reports, device test reports, translations, drivers and code are all welcome.

## Getting set up

See *Building from source* in the [README](README.md). The app is a [Tauri 2](https://tauri.app) app:

- `src/` — the interface (SvelteKit in single-page mode, Svelte 5, TypeScript).
  `src/lib/api.ts` is the only place the interface talks to the backend.
- `src-tauri/src/` — the Rust backend:
  - `commands.rs` — every command the interface can call (keep it in sync with `api.ts`).
  - `db.rs` — the SQLite library database and its migrations.
  - `library.rs`, `epub.rs` — importing files and reading EPUB metadata and covers.
  - `devices/` — e-reader drivers and the USB poller.
  - `catalog.rs` — Open Library and Project Gutenberg search.
  - `calibre.rs` — importing a calibre library (from a copy of its `metadata.db`).
    To try it on a real library: `SHELF_TEST_CALIBRE_LIBRARY=/path/to/library cargo test real_calibre -- --ignored --nocapture`.
  - `goodreads.rs` — Goodreads shelf feeds and CSV exports.
  - `watcher.rs` — watch folders.

## Before you open a pull request

- `npm run check` passes.
- `cargo test --manifest-path src-tauri/Cargo.toml` passes, and new behaviour has a test where it's practical (see the existing tests for EPUB parsing and the Kobo database).
- Database changes go in a new migration step in `db.rs` (bump `user_version`); never edit an existing schema step.
- Keep commits focused, and describe *why* in the message.

## Adding an e-reader driver

1. Add a module under `src-tauri/src/devices/` (see `kobo.rs`). A driver needs to:
   - recognise the device from its mount point and give it a stable id (a serial number is ideal);
   - copy a book onto it in the best format it reads, returning the path on the device;
   - remove a book;
   - read reading progress back, if the device stores it somewhere readable.
2. Call its `detect` from `devices::detect_all`, and route sends to it by `kind`.
3. Never write to a device's own database without saving a backup first, and prefer working on a copy.
4. Fill in the device report template with what you tested, on which model and firmware.

**Kobo model names.** Shelf names a Kobo from the model code at the end of `.kobo/version` (e.g. `...-000000000388` is a Libra 2). Only codes confirmed on a real device are listed in `model_name` in `devices/kobo.rs`; if yours shows as "Kobo eReader", add its code there with the model name.

No device to hand? Make a folder that looks like one (for a Kobo: a `.kobo/version` file whose first comma-separated field is a serial number) and point Shelf at it:

```bash
SHELF_DEVICE_ROOTS=/path/to/fake-kobo npm run tauri dev
```

Even if you can't write code, **device test reports are very valuable**: plug in your e-reader, try a sync, and open an issue with the *Device report* template.

## Book sources policy

The search links that ship with Shelf (`default_sources` in `src-tauri/src/state.rs`) are limited to **legitimate stores and libraries of free, legally shared books** (public domain or openly licensed). Pull requests adding presets for sites that distribute copyrighted books without permission won't be accepted. Users can still add any site they like to their own copy through *Settings*.

The same applies to direct downloads: Shelf only downloads files itself from Project Gutenberg. New free libraries can be added if their licensing is clear and they offer a stable API.

## Code of conduct

Be kind, assume good intent, and keep discussion about the work. Harassment of any kind isn't tolerated, and maintainers may remove comments or contributors that break this.

## The app icon

The icon's source drawings are in `src-tauri/icons/source/`: `app-icon.svg` for 48 px and up, and `app-icon-small.svg`, a simplified version for 16–32 px where detail turns to mush. To regenerate everything after changing them:

```bash
npx tauri icon src-tauri/icons/source/app-icon.svg
npx tauri icon src-tauri/icons/source/app-icon-small.svg -o /tmp/small -p 16 -p 24 -p 30 -p 32 -p 44
npx tauri icon src-tauri/icons/source/app-icon.svg -o /tmp/big -p 48 -p 64 -p 256
python scripts/make_ico.py src-tauri/icons/icon.ico /tmp/small/16x16.png /tmp/small/24x24.png /tmp/small/32x32.png /tmp/big/48x48.png /tmp/big/64x64.png /tmp/big/256x256.png
cp /tmp/small/32x32.png src-tauri/icons/32x32.png
cp /tmp/small/30x30.png src-tauri/icons/Square30x30Logo.png
cp /tmp/small/44x44.png src-tauri/icons/Square44x44Logo.png
cp /tmp/small/32x32.png static/favicon.png
rm -rf src-tauri/icons/android src-tauri/icons/ios
```

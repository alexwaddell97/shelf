mod calibre;
mod catalog;
mod commands;
mod covers;
mod db;
mod devices;
mod epub;
mod epub_write;
mod error;
mod goodreads;
mod kepub;
mod library;
mod model;
mod open_with;
mod state;
mod tools;
mod watcher;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    devices::quiet_drive_errors();
    tauri::Builder::default()
        // Must come first: a second launch (e.g. double-clicking another book) passes its
        // files to this window and exits.
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            open_with::open_files(app, open_with::ebook_args(args));
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = state::AppState::init(app.handle())?;
            // Let the interface load cover images from wherever the data folder is.
            app.asset_protocol_scope()
                .allow_directory(&state.paths.covers_dir, true)?;
            app.manage(state);
            watcher::restart(app.handle());
            devices::start_polling(app.handle().clone());
            goodreads::start_hourly(app.handle().clone());
            open_with::open_files(app.handle(), open_with::ebook_args(std::env::args()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_books,
            commands::get_book,
            commands::import_files,
            commands::update_book,
            commands::delete_book,
            commands::reveal_book,
            commands::open_book,
            commands::list_devices,
            commands::refresh_devices,
            commands::queue_for_device,
            commands::unqueue,
            commands::sync_device,
            commands::remove_from_device,
            commands::forget_device,
            commands::list_sources,
            commands::save_sources,
            commands::search_catalog,
            commands::download_free,
            commands::list_wishlist,
            commands::add_to_wishlist,
            commands::remove_from_wishlist,
            commands::open_url,
            commands::get_settings,
            commands::save_settings,
            commands::get_tools,
            commands::find_calibre_library,
            commands::inspect_calibre,
            commands::import_calibre,
            commands::get_goodreads,
            commands::connect_goodreads,
            commands::check_goodreads,
            commands::disconnect_goodreads,
            commands::import_goodreads_csv,
            commands::list_labels,
            commands::create_label,
            commands::rename_label,
            commands::delete_label,
            commands::label_books,
            commands::set_cover_from_file,
            commands::set_cover_from_url,
            commands::remove_cover,
            commands::find_covers,
            commands::take_pending_open,
            commands::open_default_apps_settings,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, _event| {
            // macOS delivers "open this file" as an event rather than as arguments.
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            if let tauri::RunEvent::Opened { urls } = _event {
                let files = urls
                    .iter()
                    .filter_map(|u| u.to_file_path().ok())
                    .map(|p| p.display().to_string());
                open_with::open_files(_app, open_with::ebook_args(files));
            }
        });
}

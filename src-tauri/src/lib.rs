mod commands;
mod db;
mod error;
mod markdown;
mod models;
mod state;
mod supabase;
mod sync;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let db = db::Db::open(&dir.join("stillnote.db"))?;
            app.manage(state::AppState::new(db));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_notebooks,
            commands::create_notebook,
            commands::rename_notebook,
            commands::delete_notebook,
            commands::list_tags,
            commands::list_notes,
            commands::get_note,
            commands::create_note,
            commands::update_note,
            commands::move_note,
            commands::set_note_tags,
            commands::set_note_pinned,
            commands::trash_note,
            commands::restore_note,
            commands::delete_note_forever,
            commands::empty_trash,
            commands::render_markdown,
            commands::get_settings,
            commands::save_settings,
            commands::auth_status,
            commands::sign_up,
            commands::sign_in,
            commands::sign_out,
            commands::sync_now,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

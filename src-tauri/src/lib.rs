mod commands;
mod error;
mod events;
mod state;
mod terminal;
mod types;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // 创建全局 AppState
            app.manage(state::AppState::new(app.handle().clone()));

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::create_local_session,
            commands::write_terminal_input,
            commands::close_terminal_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

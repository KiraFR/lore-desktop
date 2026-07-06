mod commands;
mod config;
mod lore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .setup(|app| {
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
        commands::ping,
        commands::lore_is_authenticated,
        commands::lore_sign_in,
        commands::lore_status,
        commands::lore_history,
        commands::lore_repositories,
        commands::lore_clone,
        config::config_load,
        config::config_save,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

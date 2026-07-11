mod commands;
mod config;
mod job;
mod lore;
mod notifications;
mod preview;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .manage(notifications::NotifState::default())
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      // Kill switch for the lore sidecars on ANY app death (P1 finding: hard
      // kill leaves `lore notification subscribe` orphans). Non-fatal on error.
      if let Err(e) = crate::job::init() {
          log::warn!("job object init failed — sidecars may outlive a hard kill: {e}");
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
        commands::lore_repository_info,
        commands::lore_clone,
        commands::lore_shared_store_status,
        commands::lore_shared_store_enable,
        commands::lore_shared_store_disable,
        config::config_load,
        config::config_save,
        commands::lore_commit,
        commands::lore_push,
        commands::lore_sync,
        commands::lore_sync_to,
        commands::lore_set_lock,
        commands::lore_locks,
        commands::lore_diff,
        commands::lore_branches,
        commands::lore_switch_branch,
        commands::lore_create_branch,
        commands::lore_archive_branch,
        commands::lore_pushed_lock_files,
        commands::lore_commit_files,
        commands::lore_merge_preview,
        commands::lore_merge,
        commands::lore_merge_start,
        commands::lore_merge_conflicts,
        commands::lore_merge_resolve,
        commands::lore_merge_commit,
        commands::lore_merge_abort,
        commands::lore_discard_file,
        commands::lore_undo_commit,
        commands::lore_amend,
        commands::lore_identity,
        commands::lore_sign_out,
        commands::lore_file_history,
        commands::lore_file_sizes,
        preview::lore_preview,
        notifications::lore_notifications_start,
        notifications::lore_notifications_stop,
        commands::os_reveal_path,
        commands::os_open_path,
        commands::os_path_exists,
        commands::lore_update_path,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

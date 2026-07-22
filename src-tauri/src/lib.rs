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
      // In-app auto-update (spec 2026-07-22): updater (check/download/install)
      // + process (relaunch after install on macOS/Linux — on Windows the NSIS
      // installer itself relaunches the app, see installUpdate in tauri.ts).
      // Both are desktop-only plugins.
      #[cfg(desktop)]
      {
        app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;
        app.handle().plugin(tauri_plugin_process::init())?;
      }
      // Logs to stdout AND a file in the app's log dir
      // (%LOCALAPPDATA%/studio.soonerorlater.lore-desktop/logs on Windows).
      // Registered unconditionally: it used to be debug-only, so installed
      // builds wrote no logs at all — which is exactly when we need them
      // (updater debugging). The updater plugin traces at debug level
      // (tauri-plugin-updater 2.10.1: check/response/install are log::debug!),
      // hence the level_for override. Rotation: the plugin's default file cap
      // is 40 KB with KeepOne — bump the cap to 2 MB, still one rotated file.
      app.handle().plugin(
        tauri_plugin_log::Builder::default()
          .level(log::LevelFilter::Info)
          .level_for("tauri_plugin_updater", log::LevelFilter::Debug)
          .targets([
            tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
            tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
              file_name: Some("lore-desktop".into()),
            }),
          ])
          .max_file_size(2_000_000)
          .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
          .build(),
      )?;
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
        commands::lore_restore_file,
        commands::lore_set_lock,
        commands::lore_locks,
        commands::lore_diff,
        commands::lore_diff_revs,
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
        commands::lore_logfile_location,
        commands::app_log_dir,
        commands::prepare_update_breakaway,
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}

//! Sidecar lifetime: ties the app process — and every child it spawns, in
//! particular the `lore notification subscribe` subscribers of
//! notifications.rs — to a Windows Job object with
//! JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE. When the app dies, even a hard kill,
//! the OS closes the job's last handle and terminates every member, so no
//! orphan `lore` process survives. Children join the job automatically at
//! spawn (no per-spawn bookkeeping, respawns included).
//!
//! Auto-update exception: that same kill switch used to kill the NSIS
//! updater. tauri-plugin-updater's install() spawns the new setup.exe
//! (ShellExecuteW) and then calls std::process::exit(0); the installer,
//! being a child of this process, was a job member — closing the job's last
//! handle on exit TERMINATED it before it could install anything. Every
//! in-app update failed silently, while the very same installer with the
//! same arguments succeeded when launched by hand. The fix is
//! [`enable_silent_breakaway`], called right before download+install: it
//! adds JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK to the job so processes
//! created AFTER the call are born outside the job (the installer survives
//! the app's exit), while processes already inside (the lore sidecars) stay
//! members and still die with the app.

/// The job handle, kept alive for the whole process lifetime: dropping it
/// would close the job and, with kill-on-close set, terminate us. Stored
/// (rather than `mem::forget`-ed, as it used to be) so
/// `enable_silent_breakaway` can amend the job's limits later.
#[cfg(windows)]
static JOB: std::sync::OnceLock<win32job::Job> = std::sync::OnceLock::new();

/// Install the kill-on-close job and put the current process in it. Called
/// once at startup; failure is non-fatal (the app runs, orphan cleanup just
/// isn't guaranteed — the caller logs and moves on). Idempotent: a second
/// call is a no-op.
#[cfg(windows)]
pub fn init() -> Result<(), String> {
    if JOB.get().is_some() {
        return Ok(());
    }
    let job = win32job::Job::create().map_err(|e| e.to_string())?;
    let mut info = job.query_extended_limit_info().map_err(|e| e.to_string())?;
    info.limit_kill_on_job_close();
    job.set_extended_limit_info(&info).map_err(|e| e.to_string())?;
    job.assign_current_process().map_err(|e| e.to_string())?;
    // A lost set() race just drops the extra Job handle; the process is
    // already assigned to the stored job, which is what matters.
    let _ = JOB.set(job);
    Ok(())
}

/// Add JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK to the live job, ON TOP of the
/// kill-on-close limit (query_extended_limit_info returns the current flags,
/// and limit_kill_on_job_close is re-applied defensively anyway). From this
/// point on, new children are created OUTSIDE the job — which is exactly
/// what the auto-update needs (see the module docs for the bug this fixes):
/// the NSIS installer spawned by the updater plugin is no longer a job
/// member, so it survives the app's exit(0). Existing members (the lore
/// sidecars) are unaffected and still die with the app.
///
/// Deliberately never undone: this runs only on the update path, where the
/// app's next act is to exit and be reinstalled.
#[cfg(windows)]
pub fn enable_silent_breakaway() -> Result<(), String> {
    let job = JOB
        .get()
        .ok_or_else(|| "job object was never installed".to_string())?;
    let mut info = job.query_extended_limit_info().map_err(|e| e.to_string())?;
    info.limit_kill_on_job_close();
    info.limit_silent_breakaway_ok();
    job.set_extended_limit_info(&info).map_err(|e| e.to_string())
}

/// POSIX: no Job objects; orphaned children are reparented and the P1 kill
/// path (generation bump + child.kill) already covers clean shutdowns.
#[cfg(not(windows))]
pub fn init() -> Result<(), String> {
    Ok(())
}

/// POSIX: nothing to break away from — the updater's install path never
/// competes with a job object here.
#[cfg(not(windows))]
pub fn enable_silent_breakaway() -> Result<(), String> {
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn job_installs_without_error() {
        // Assigning the test process to a kill-on-close job is harmless: the
        // job's last handle closes when the test process exits anyway.
        assert_eq!(super::init(), Ok(()));
        // Idempotent — tests share one process, init must tolerate reruns.
        assert_eq!(super::init(), Ok(()));
    }

    #[test]
    fn breakaway_flips_on_the_live_job() {
        // init() first: tests run in one process, so the job may or may not
        // already exist depending on ordering — init() is idempotent.
        assert_eq!(super::init(), Ok(()));
        assert_eq!(super::enable_silent_breakaway(), Ok(()));
        // Setting an already-set flag must also succeed (the update UI could
        // retry a failed install).
        assert_eq!(super::enable_silent_breakaway(), Ok(()));
    }
}

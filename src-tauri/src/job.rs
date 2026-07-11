//! Sidecar lifetime: ties the app process — and every child it spawns, in
//! particular the `lore notification subscribe` subscribers of
//! notifications.rs — to a Windows Job object with
//! JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE. When the app dies, even a hard kill,
//! the OS closes the job's last handle and terminates every member, so no
//! orphan `lore` process survives. Children join the job automatically at
//! spawn (no per-spawn bookkeeping, respawns included).

/// Install the kill-on-close job and put the current process in it. Called
/// once at startup; failure is non-fatal (the app runs, orphan cleanup just
/// isn't guaranteed — the caller logs and moves on).
#[cfg(windows)]
pub fn init() -> Result<(), String> {
    let job = win32job::Job::create().map_err(|e| e.to_string())?;
    let mut info = job.query_extended_limit_info().map_err(|e| e.to_string())?;
    info.limit_kill_on_job_close();
    job.set_extended_limit_info(&mut info).map_err(|e| e.to_string())?;
    job.assign_current_process().map_err(|e| e.to_string())?;
    // Keep the handle open for the whole process lifetime: dropping it would
    // close the job and kill us. The OS reclaims it at process death — which
    // is exactly the kill switch we want.
    std::mem::forget(job);
    Ok(())
}

/// POSIX: no Job objects; orphaned children are reparented and the P1 kill
/// path (generation bump + child.kill) already covers clean shutdowns.
#[cfg(not(windows))]
pub fn init() -> Result<(), String> {
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn job_installs_without_error() {
        // Assigning the test process to a kill-on-close job is harmless: the
        // job's last handle closes when the test process exits anyway.
        assert_eq!(super::init(), Ok(()));
    }
}

export type RestoreLock = 'free' | 'mine' | 'teammate'

export interface RestoreAvailability {
  /** Whether the "Restore this version" action is offered. */
  canRestore: boolean
  /** Why it's disabled (tooltip), or null when enabled. */
  reason: string | null
  /** Lock category of the file — drives whether we acquire the lock + the note. */
  lock: RestoreLock
}

export interface RestoreContext {
  /** This revision is already the working-copy tip — restoring it is a no-op. */
  isCurrent: boolean
  /** Any pending working changes — a restore's sync round-trip would clobber them. */
  dirtyTree: boolean
  /** Working copy sits below the local head (P3 time-travel). */
  timeTraveled: boolean
  /** Lock holder of THIS file: 'you', a teammate's name, or null (unlocked). */
  lockHolder: string | null
}

/**
 * Can this file revision be restored, and how? A teammate lock does NOT disable
 * it — the restore is local (the file lands in the teammate-locked, non-committable
 * section). The hard guards are: not the current revision, a clean working tree
 * (no stash in Lore), and not time-traveled.
 */
export function restoreAvailability(ctx: RestoreContext): RestoreAvailability {
  const lock: RestoreLock = ctx.lockHolder == null ? 'free' : ctx.lockHolder === 'you' ? 'mine' : 'teammate'
  if (ctx.isCurrent) return { canRestore: false, reason: 'This is the current version', lock }
  if (ctx.dirtyTree) return { canRestore: false, reason: 'Commit or discard your pending changes first', lock }
  if (ctx.timeTraveled) return { canRestore: false, reason: 'Sync back to the latest first', lock }
  return { canRestore: true, reason: null, lock }
}

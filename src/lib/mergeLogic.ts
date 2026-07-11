export interface MergeWording {
  /** Resolution-phase banner. */
  banner: string
  /** Message passed to `mergeCommit`. */
  commitMessage: string
  /** Done-view sentence. */
  done: string
  /** "Theirs" conflict-card title. */
  theirsCard: string
}

/**
 * UI wording for a merge whose source branch may be unknown. A merge resumed
 * from outside the app has no reliable source (Merge.svelte defaults the
 * select to the first non-current branch) — a guessed branch name must NEVER
 * be displayed, so every label falls back to a neutral form.
 */
export function mergeWording(source: string | null, target: string): MergeWording {
  if (!source) {
    return {
      banner: `Resolving merge into ${target}`,
      commitMessage: `Merge into ${target}`,
      done: `A branch was merged into ${target}.`,
      theirsCard: 'Theirs · incoming',
    }
  }
  return {
    banner: `Merging ${source} into ${target}`,
    commitMessage: `Merge ${source} into ${target}`,
    done: `${source} was merged into ${target}.`,
    theirsCard: `Theirs · ${source}`,
  }
}

/**
 * One step of the external-abort watcher. `saw` latches once the backend
 * status confirms the merge (`mergeInProgress === true`); a later `false`
 * while the view is still resolving means `branch merge abort` happened
 * outside the app → `aborted`. `undefined` (no status yet) changes nothing.
 */
export function externalAbortStep(
  resolving: boolean,
  mergeInProgress: boolean | undefined,
  saw: boolean,
): { saw: boolean; aborted: boolean } {
  if (!resolving) return { saw: false, aborted: false }
  if (mergeInProgress === true) return { saw: true, aborted: false }
  if (mergeInProgress === false && saw) return { saw: false, aborted: true }
  return { saw, aborted: false }
}

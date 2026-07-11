import type { Commit } from './types'

/** Client-side commit filter over the LOADED commits only — a deliberate v1:
 *  History preloading + pagination already hold hundreds of commits in memory,
 *  and the server-side `lore revision find` has unknown match semantics (single?
 *  multi?) that would need its own capture for a marginal gain. Full-history
 *  server search is a future lot.
 *  (Spec: docs/superpowers/specs/2026-07-11-lore-desktop-p5-search-design.md)
 *
 *  Matching, case-insensitive: substring on message, author and short hash;
 *  revision numbers match by PREFIX — "42" or "#42" hits #42, #420, #4211…
 *  (substring on numbers would be noisy). Criteria are OR-ed. Blank query =
 *  everything (same array, so `===` checks stay cheap). */
export function filterCommits(commits: Commit[], query: string): Commit[] {
  const q = query.trim().toLowerCase()
  if (!q) return commits
  const digits = q.startsWith('#') ? q.slice(1) : q
  const isRevQuery = /^\d+$/.test(digits)
  return commits.filter((c) =>
    c.message.toLowerCase().includes(q) ||
    c.author.toLowerCase().includes(q) ||
    c.id.toLowerCase().includes(q) ||
    (isRevQuery && String(c.rev).startsWith(digits)),
  )
}

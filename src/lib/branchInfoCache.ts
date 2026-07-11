import type { BranchInfo } from './types'

/** « ↑2 ↓5 » discret ; null quand il n'y a rien à montrer (branche à jour). */
export function formatAheadBehind(info?: BranchInfo): string | null {
  if (!info) return null
  const parts: string[] = []
  if (info.ahead) parts.push(`↑${info.ahead}`)
  if (info.behind) parts.push(`↓${info.behind}`)
  return parts.length > 0 ? parts.join(' ') : null
}

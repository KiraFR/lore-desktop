/** The stored commit message: summary, plus the description as body when present. */
export function composeCommitMessage(summary: string, description: string): string {
  const s = summary.trim()
  const d = description.trim()
  return d ? `${s}\n\n${d}` : s
}

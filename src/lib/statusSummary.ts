export interface StatusSummary {
  adds: number
  mods: number
  dels: number
}

export interface SummaryPart {
  text: string
  cls: 'added' | 'modified' | 'deleted'
}

/**
 * Parties colorées des compteurs de l'en-tête de Changes (« +3 ~2 −1 »).
 * Vide quand le summary est absent (CLI plus ancien) ou tout à zéro — la
 * feature disparaît, elle ne montre jamais de faux zéros.
 */
export function summaryParts(s?: StatusSummary | null): SummaryPart[] {
  if (!s) return []
  const parts: SummaryPart[] = []
  if (s.adds > 0) parts.push({ text: `+${s.adds}`, cls: 'added' })
  if (s.mods > 0) parts.push({ text: `~${s.mods}`, cls: 'modified' })
  if (s.dels > 0) parts.push({ text: `−${s.dels}`, cls: 'deleted' })
  return parts
}

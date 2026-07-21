/** Pure helpers behind the Changes list keyboard selection and the grouped
 *  stage/unstage context-menu items. Kept out of the component so vitest can
 *  cover them (no Svelte plugin in the test runner). */

/** Path reached by ↑/↓ over the DISPLAYED order. From no current path (or one
 *  filtered out of view), ↓ lands on the first row and ↑ on the last. Clamps
 *  at the edges instead of wrapping. */
export function stepPath(order: string[], current: string | null, delta: -1 | 1): string | null {
  if (order.length === 0) return null
  const i = current === null ? -1 : order.indexOf(current)
  if (i < 0) return delta === 1 ? order[0] : order[order.length - 1]
  return order[Math.min(order.length - 1, Math.max(0, i + delta))]
}

/** Contiguous range between anchor and focus (both inclusive) in displayed
 *  order, or null when either end is not displayed (e.g. filtered out). */
export function rangePaths(order: string[], anchor: string, focus: string): string[] | null {
  const a = order.indexOf(anchor)
  const b = order.indexOf(focus)
  if (a < 0 || b < 0) return null
  const [lo, hi] = a <= b ? [a, b] : [b, a]
  return order.slice(lo, hi + 1)
}

/** Splits a selection into what a bulk action can stage vs unstage. Iterates
 *  the committable paths, so teammate-locked files can never be staged (they
 *  are simply not in the list) and both results keep displayed order. */
export function stagePartition(
  selected: ReadonlySet<string>,
  committablePaths: string[],
  staged: ReadonlySet<string>,
): { toStage: string[]; toUnstage: string[] } {
  const toStage: string[] = []
  const toUnstage: string[] = []
  for (const p of committablePaths) {
    if (!selected.has(p)) continue
    if (staged.has(p)) toUnstage.push(p)
    else toStage.push(p)
  }
  return { toStage, toUnstage }
}

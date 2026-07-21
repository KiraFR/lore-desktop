// .loreignore — app-side ignore filtering (the Lore CLI has none, see the
// 2026-07-21 design spec). Common gitignore syntax WITHOUT negation:
// `#` comments, blank lines, `*` / `?` / `**` globs, trailing `/` = whole
// directory, a leading or interior `/` anchors the pattern to the repo root,
// otherwise it matches at any depth.
//
// Matching is CASE-SENSITIVE (deliberate v1 choice): paths come from the CLI
// with their on-disk casing and rules are applied verbatim.

export interface IgnoreRule {
  /** The original .loreignore line (trimmed) — kept for debugging/future UI. */
  source: string
  re: RegExp
}

/** Glob → RegExp source: `**` traverses `/`, `*` and `?` don't, rest escaped. */
function globToRegExpSource(glob: string): string {
  let out = ''
  for (let i = 0; i < glob.length; i++) {
    const c = glob[i]
    if (c === '*') {
      if (glob[i + 1] === '*') { out += '.*'; i++ }
      else out += '[^/]*'
    } else if (c === '?') {
      out += '[^/]'
    } else {
      out += /[.*+?^${}()|[\]\\]/.test(c) ? `\\${c}` : c
    }
  }
  return out
}

function compileRule(line: string): RegExp | null {
  // Trailing `/` marks "whole directory" — no extra effect in v1 (the status
  // only lists files and the `(/.*)?` suffix already covers everything below).
  let pat = line.endsWith('/') ? line.slice(0, -1) : line
  // A `/` anywhere (leading or interior) anchors the pattern to the repo root;
  // the leading `/` itself is simply dropped before conversion.
  const anchored = pat.includes('/')
  if (pat.startsWith('/')) pat = pat.slice(1)
  if (!pat) return null
  const glob = globToRegExpSource(pat)
  // The `(/.*)?` suffix makes a matched directory name ignore its whole
  // content, like gitignore.
  return anchored ? new RegExp(`^${glob}(/.*)?$`) : new RegExp(`(^|/)${glob}(/.*)?$`)
}

export function parseIgnore(text: string): IgnoreRule[] {
  const rules: IgnoreRule[] = []
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim()
    if (!line || line.startsWith('#')) continue
    const re = compileRule(line)
    if (re) rules.push({ source: line, re })
  }
  return rules
}

/** `path` is repo-relative with `/` separators (the `ChangedFile.path` format). */
export function isIgnored(path: string, rules: IgnoreRule[]): boolean {
  // The ignore file itself is never ignored, whatever the rules say — hiding
  // it would make its own edits invisible and uncommittable.
  if (path === '.loreignore') return false
  return rules.some((r) => r.re.test(path))
}

/** Split a change list into kept / ignored (the store filters BEFORE storing). */
export function filterIgnored<T extends { path: string }>(
  files: T[],
  rules: IgnoreRule[],
): { kept: T[]; ignored: T[] } {
  if (rules.length === 0) return { kept: files, ignored: [] }
  const kept: T[] = []
  const ignored: T[] = []
  for (const f of files) (isIgnored(f.path, rules) ? ignored : kept).push(f)
  return { kept, ignored }
}

/**
 * Wire summary minus the ignored files' contribution, so the header counters
 * reflect the filtered list. Clamped at 0 in case the wire counters and the
 * file list ever disagree.
 */
export function adjustSummary(
  summary: { adds: number; mods: number; dels: number } | undefined,
  ignored: { action: string }[],
): { adds: number; mods: number; dels: number } | undefined {
  if (!summary || ignored.length === 0) return summary
  let { adds, mods, dels } = summary
  for (const f of ignored) {
    if (f.action === 'add') adds--
    else if (f.action === 'delete') dels--
    else mods-- // modify / move / copy all count as mods (same mapping as the mock)
  }
  return { adds: Math.max(adds, 0), mods: Math.max(mods, 0), dels: Math.max(dels, 0) }
}

import type { Branch } from './types'

/**
 * Ligne du BranchMenu virtualisé : une branche, ou l'en-tête de la section
 * « Remote ». Tout est aplati en lignes de hauteur fixe pour que la
 * virtualisation existante (2000+ branches du stress-test) reste inchangée.
 */
export type BranchRow =
  | { kind: 'branch'; branch: Branch }
  | { kind: 'header'; label: string }

/**
 * Branches locales d'abord (section actuelle, sans titre), puis un séparateur
 * « Remote » et les branches remote-only. Le filtre s'applique aux deux
 * groupes ; une section Remote vidée par le filtre disparaît (en-tête compris).
 * `location` absente = locale (défaut sûr).
 */
export function groupBranches(list: Branch[], filter: string): BranchRow[] {
  const q = filter.trim().toLowerCase()
  const match = (b: Branch) => b.name.toLowerCase().includes(q)
  const locals = list.filter((b) => (b.location ?? 'local') === 'local' && match(b))
  const remotes = list.filter((b) => b.location === 'remote' && match(b))
  const rows: BranchRow[] = locals.map((branch) => ({ kind: 'branch' as const, branch }))
  if (remotes.length > 0) {
    rows.push({ kind: 'header', label: 'Remote' })
    for (const branch of remotes) rows.push({ kind: 'branch', branch })
  }
  return rows
}

# Lore Desktop — Lot P5 « recherche & navigation » Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Livrer les 2 items du lot P5 (spec `docs/superpowers/specs/2026-07-11-lore-desktop-p5-search-design.md`) : un filtre dans la vue Locks (chemin + détenteur, compteur « N of M »), puis une recherche CLIENT dans History (filtrage des commits déjà chargés sur message/auteur/hash/#rev, liste plate sans lanes, hint « loaded commits only », bouton Load more accessible sous filtre, survie/reset de la sélection, Escape qui vide le champ).

**Architecture:** Frontend Svelte 5 pur — aucun Rust, aucune capture wire. La logique nouvelle vit dans des modules purs testés vitest : `filterByText` (généralisation de `filterByQuery` dans `changesPartition.ts`), `filterCommits` (nouveau module `historyFilter.ts`), `selectionAfterFilter` (ajout à `historySelection.ts`). Les composants (`Locks.svelte`, `History.svelte`) ne font que le wiring + markup, vérifié navigateur mock. En mode filtre, History garde SA virtualisation existante (fenêtre `first/last` sur `ROW_H`) mais bascule sur la liste filtrée et masque le SVG des lanes.

**Tech Stack:** Svelte 5 runes, TypeScript, vitest (jsdom), mock API (`src/lib/mock.ts`, BIG_HISTORY 5000 commits / page 200).

---

## Contexte & conventions (à lire avant toute tâche)

- **Contrainte vitest** : `vitest.config.ts` n'a PAS le plugin Svelte — les tests ne peuvent importer NI un `.svelte` NI un `.svelte.ts` à runes. Toute logique testée vit dans des modules purs (`.ts` sans runes) ; le wiring composants est vérifié navigateur (Task 9).
- **Commandes de test** :
  - Vitest ciblé : `npx vitest run src/lib/<fichier>.test.ts` — attendu `Test Files  1 passed`.
  - Vitest complet : `npx vitest run` — attendu `Test Files  N passed`, 0 failed.
  - Typecheck : `npm run check` — attendu `svelte-check found 0 errors and 0 warnings`.
  - Dev navigateur (mock) : `npm run dev` → http://localhost:5173 (l'API mock est active hors Tauri).
- **Données mock utiles** : Locks = 3 entrées (`Level_01.umap`/you, `T_Cliff_D.uasset`/Maya R, `SK_Hero.uasset`/Alex L). History = `BIG_HISTORY` de 5000 commits (`buildBigHistory` dans `mock.ts`), chargés par pages de 200 (`HISTORY_PAGE` dans `repo.svelte.ts:9`) ; les revs vont de #5000 (le plus récent) vers #1, les ids sont `c100000`, `c100001`, …
- **Décisions tranchées (à respecter, ne pas re-débattre)** :
  1. **Virtualisation sous filtre** : on RÉUTILISE la fenêtre virtuelle existante (`first`/`last`/`ROW_H`) en la faisant pointer sur un tableau dérivé `rows = filterActive ? filtered : commits`. Justification : une requête large (« a », « fix ») sur 1000+ commits chargés rendrait des milliers de lignes DOM en mode non-virtualisé ; réutiliser la machinerie déjà en place coûte 4 lignes (échanger `commits` pour `rows` dans les dérivés) et évite un second markup de liste complet. Seul le SVG des lanes est conditionnel — le markup des lignes reste le même `.grow` positionné, avec un `padding-left` réduit et sans headpill en mode filtre.
  2. **`filterByText` (pas `filterByPath`)** : la spec suggère une surcharge générique `filterByPath` si `filterByQuery` ne s'applique pas tel quel à `LockEntry` — or Locks doit matcher chemin ET détenteur, donc un helper path-only ne suffit pas. On généralise en `filterByText<T>(items, query, haystack)` (extraction de champs texte par item) et `filterByQuery` devient une délégation une-ligne. Comportement Changes strictement inchangé (les tests existants le verrouillent).
  3. **Match `#N` en PRÉFIXE** : « 42 » ou « #42 » matche #42, #420, #4211… (spec : « le match par numéro « 42 » → #42 en préfixe ») ; une sous-chaîne sur les numéros serait bruitée. Une requête numérique matche AUSSI les hash/messages par sous-chaîne (les critères sont en OU).
  4. **Auto-load-more au scroll DÉSACTIVÉ sous filtre** : une liste de matches courte aurait `scrollHeight` minuscule → le déclencheur bas-de-liste fetcherait tout l'historique en boucle. Sous filtre, le chargement passe par le bouton « Load more » explicite (spec). Le hint garde le texte verbatim de la spec.
  5. **Escape** : géré sur l'input filtre lui-même (`onkeydown`). La garde existante du preview panel (`History.svelte` — le listener window ignore Escape quand `e.target instanceof HTMLInputElement`) fait que le panel ne se ferme PAS quand Escape est tapé dans le champ filtre, et l'éditeur de message garde son propre Escape. Ne pas toucher à cette garde.
  6. **Reset scroll sur changement de requête** : la position de scroll n'a pas de sens contre un jeu de lignes différent — on remonte en haut à chaque changement de `query` (y compris au clear).
- Ordre de livraison imposé par la spec : Item 2 Locks (Tasks 1–2) → Item 1 History (Tasks 3–8) → vérification finale (Task 9).
- **Hors périmètre** (spec) : `revision find` serveur, recherche dans Changes, plein-texte dans les diffs, tri des résultats. Ne rien ajouter de tout ça.

## Carte des fichiers

**Créés :**
- `src/lib/historyFilter.ts` (+ `historyFilter.test.ts`) — `filterCommits(commits, query)` pur, documente le choix « client-side only ».

**Modifiés :**
- `src/lib/changesPartition.ts` (+ `changesPartition.test.ts`) — `filterByText<T>` générique ; `filterByQuery` délègue.
- `src/lib/Locks.svelte` — champ filtre + compteur « N of M » + état vide « No locks match. ».
- `src/lib/historySelection.ts` (+ `historySelection.test.ts`) — `selectionAfterFilter`.
- `src/lib/History.svelte` — input filtre débouncé 150 ms, compteur/hint, liste plate virtualisée sous filtre, survie/reset sélection, Escape, bouton Load more.

---

## Item 2 — Filtre dans Locks

### Task 1: Généraliser le filtre texte (`filterByText`)

**Files:**
- Modify: `src/lib/changesPartition.ts:20-25`
- Test: `src/lib/changesPartition.test.ts`

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `src/lib/changesPartition.test.ts`, remplacer la ligne d'import 2-3 :

```ts
import { partitionByLock, filterByQuery } from './changesPartition'
import type { ChangedFile } from './types'
```

par :

```ts
import { partitionByLock, filterByQuery, filterByText } from './changesPartition'
import type { ChangedFile, LockEntry } from './types'
```

et ajouter à la fin du fichier :

```ts
describe('filterByText', () => {
  const lock = (path: string, holder: string): LockEntry => ({ path, holder, when: '1 h ago' })
  const locks = [
    lock('Content/Maps/Level_01.umap', 'you'),
    lock('Content/Environment/T_Cliff_D.uasset', 'Maya R'),
    lock('Content/Characters/Hero/SK_Hero.uasset', 'Alex L'),
  ]
  const fields = (l: LockEntry) => [l.path, l.holder]

  it('returns the same array for a blank query', () => {
    expect(filterByText(locks, '  ', fields)).toBe(locks)
  })
  it('matches the path case-insensitively', () => {
    expect(filterByText(locks, 'LEVEL', fields).map((l) => l.path)).toEqual(['Content/Maps/Level_01.umap'])
  })
  it('matches the holder too', () => {
    expect(filterByText(locks, 'maya', fields).map((l) => l.holder)).toEqual(['Maya R'])
  })
  it('spans both fields with one query, order preserved', () => {
    expect(filterByText(locks, 'e', fields).map((l) => l.holder)).toEqual(['you', 'Maya R', 'Alex L'])
  })
  it('returns nothing when neither field matches', () => {
    expect(filterByText(locks, 'zzz', fields)).toEqual([])
  })
})
```

- [ ] **Step 2: Vérifier l'échec**

Run: `npx vitest run src/lib/changesPartition.test.ts`
Expected: FAIL — `filterByText` is not exported / not a function.

- [ ] **Step 3: Implémenter `filterByText` et déléguer `filterByQuery`**

Dans `src/lib/changesPartition.ts`, remplacer les lignes 20-25 :

```ts
/** Case-insensitive substring filter on the path; blank query = everything. */
export function filterByQuery(files: ChangedFile[], query: string): ChangedFile[] {
  const q = query.trim().toLowerCase()
  if (!q) return files
  return files.filter((f) => f.path.toLowerCase().includes(q))
}
```

par :

```ts
/** Case-insensitive substring filter over the text fields `haystack` extracts
 *  from each item (e.g. path + lock holder); blank query = everything. */
export function filterByText<T>(items: T[], query: string, haystack: (item: T) => string[]): T[] {
  const q = query.trim().toLowerCase()
  if (!q) return items
  return items.filter((it) => haystack(it).some((s) => s.toLowerCase().includes(q)))
}

/** Case-insensitive substring filter on the path; blank query = everything. */
export function filterByQuery(files: ChangedFile[], query: string): ChangedFile[] {
  return filterByText(files, query, (f) => [f.path])
}
```

- [ ] **Step 4: Vérifier que tout passe (y compris les tests existants de `filterByQuery`)**

Run: `npx vitest run src/lib/changesPartition.test.ts`
Expected: PASS — tous les tests, anciens ET nouveaux (le comportement de `filterByQuery` côté Changes est inchangé).

- [ ] **Step 5: Commit**

```bash
git add src/lib/changesPartition.ts src/lib/changesPartition.test.ts
git commit -m "feat(locks): generic filterByText helper (path + holder), filterByQuery delegates"
```

### Task 2: Champ filtre dans la vue Locks

**Files:**
- Modify: `src/lib/Locks.svelte` (script : imports + état ; markup : `lhead` compteur, input, liste ; styles : `.filter`)

- [ ] **Step 1: Ajouter l'état et le dérivé filtré**

Dans `src/lib/Locks.svelte`, après la ligne 8 (`import ContextMenu from './ContextMenu.svelte'`), ajouter :

```ts
  import { filterByText } from './changesPartition'
```

et après le `$effect` des thumbnails (ligne 15), ajouter :

```ts
  let filter = $state('')
  const shown = $derived(filterByText(locks.list, filter, (l) => [l.path, l.holder]))
```

- [ ] **Step 2: Markup — compteur « N of M », input, liste filtrée, état « no match »**

Remplacer le bloc `lhead` + liste (lignes 72-100 actuelles) :

```svelte
  <div class="lhead">
    <span class="title"><Icon name="lock" size={16} /> Locks <span class="count">{locks.list.length} held</span></span>
    <button class="ghost" onclick={lockNewFile} disabled={locking || !!repo.busy}>{locking ? 'Locking…' : '+ Lock a file…'}</button>
  </div>

  {#if locks.list.length === 0}
    <div class="empty muted">No files are locked.</div>
  {:else}
    <div class="list" role="list">
      {#each locks.list as l (l.path)}
```

par :

```svelte
  <div class="lhead">
    <span class="title"><Icon name="lock" size={16} /> Locks <span class="count">{filter.trim() ? `${shown.length} of ${locks.list.length}` : `${locks.list.length} held`}</span></span>
    <button class="ghost" onclick={lockNewFile} disabled={locking || !!repo.busy}>{locking ? 'Locking…' : '+ Lock a file…'}</button>
  </div>

  <input class="filter" bind:value={filter} placeholder="Filter locks" />

  {#if locks.list.length === 0}
    <div class="empty muted">No files are locked.</div>
  {:else if shown.length === 0}
    <div class="empty muted">No locks match.</div>
  {:else}
    <div class="list" role="list">
      {#each shown as l (l.path)}
```

Le reste du `{#each}` (contenu de `.lrow`, `{/each}`, `{/if}`) est inchangé.

- [ ] **Step 3: Style de l'input (pattern du filtre Changes)**

Dans le `<style>` de `Locks.svelte`, après la règle `.lhead .ghost { margin-left: auto; }`, ajouter :

```css
  .filter { display: block; width: 100%; margin: 0 0 12px; padding: 6px 9px; background: var(--bg); border: 1px solid var(--border); border-radius: 6px; color: var(--text); font-size: 12px; }
```

- [ ] **Step 4: Typecheck**

Run: `npm run check`
Expected: `svelte-check found 0 errors and 0 warnings`.

- [ ] **Step 5: Sanity navigateur mock**

Run: `npm run dev` → http://localhost:5173, vue Locks (mock : 3 verrous).
Vérifier : « maya » → compteur « 1 of 3 », seule la ligne `T_Cliff_D.uasset` (détenteur) reste ; « hero » → « 1 of 3 » (`SK_Hero.uasset`, match chemin) ; « zzz » → « No locks match. » ; champ vidé → « 3 held » et les 3 lignes.

- [ ] **Step 6: Commit**

```bash
git add src/lib/Locks.svelte
git commit -m "feat(locks): filter field over path and holder with N-of-M counter"
```

---

## Item 1 — Recherche dans History

### Task 3: Module pur `filterCommits`

**Files:**
- Create: `src/lib/historyFilter.ts`
- Test: `src/lib/historyFilter.test.ts`

- [ ] **Step 1: Écrire les tests qui échouent**

Créer `src/lib/historyFilter.test.ts` :

```ts
import { describe, it, expect } from 'vitest'
import { filterCommits } from './historyFilter'
import type { Commit } from './types'

const c = (over: Pick<Commit, 'id' | 'rev' | 'message' | 'author'>): Commit => ({
  when: '1 min ago', whenMs: 0, lane: 0, parents: [], files: [], ...over,
})

const commits = [
  c({ id: 'cafe12', rev: 421, message: 'Fix player movement', author: 'Maya R' }),
  c({ id: 'cbb001', rev: 42, message: 'Add loot tables', author: 'you' }),
  c({ id: 'cbb002', rev: 7, message: 'Tune audio mix', author: 'Alex L' }),
]

describe('filterCommits', () => {
  it('returns the same array for a blank or whitespace query', () => {
    expect(filterCommits(commits, '')).toBe(commits)
    expect(filterCommits(commits, '   ')).toBe(commits)
  })
  it('matches the message case-insensitively', () => {
    expect(filterCommits(commits, 'PLAYER').map((x) => x.rev)).toEqual([421])
    expect(filterCommits(commits, 'loot').map((x) => x.rev)).toEqual([42])
  })
  it('preserves commit order across multiple matches', () => {
    // 'i' hits "Fix player movement" and "Tune audio mix", not "Add loot tables".
    expect(filterCommits(commits, 'i').map((x) => x.rev)).toEqual([421, 7])
  })
  it('matches the author case-insensitively', () => {
    expect(filterCommits(commits, 'maya').map((x) => x.rev)).toEqual([421])
    expect(filterCommits(commits, 'ALEX').map((x) => x.rev)).toEqual([7])
  })
  it('matches the short hash case-insensitively', () => {
    expect(filterCommits(commits, 'AFE1').map((x) => x.rev)).toEqual([421])
    expect(filterCommits(commits, 'cbb0').map((x) => x.rev)).toEqual([42, 7])
  })
  it('matches revision numbers by prefix — "42" hits #421 and #42', () => {
    expect(filterCommits(commits, '42').map((x) => x.rev)).toEqual([421, 42])
  })
  it('accepts the "#N" form with the same prefix semantics', () => {
    expect(filterCommits(commits, '#42').map((x) => x.rev)).toEqual([421, 42])
    expect(filterCommits(commits, '#421').map((x) => x.rev)).toEqual([421])
    expect(filterCommits(commits, '#7').map((x) => x.rev)).toEqual([7])
  })
  it('a digit query also hits hashes by substring (criteria are OR-ed)', () => {
    expect(filterCommits(commits, '001').map((x) => x.rev)).toEqual([42])
  })
  it('rejects non-matches: unknown text, "#" alone, "#" + non-digits, too-long rev', () => {
    expect(filterCommits(commits, 'zzz')).toEqual([])
    expect(filterCommits(commits, '#')).toEqual([])
    expect(filterCommits(commits, '#abc')).toEqual([])
    expect(filterCommits(commits, '#4211')).toEqual([])
  })
})
```

- [ ] **Step 2: Vérifier l'échec**

Run: `npx vitest run src/lib/historyFilter.test.ts`
Expected: FAIL — `Cannot find module './historyFilter'` (ou équivalent).

- [ ] **Step 3: Implémenter `filterCommits`**

Créer `src/lib/historyFilter.ts` :

```ts
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
```

- [ ] **Step 4: Vérifier que tout passe**

Run: `npx vitest run src/lib/historyFilter.test.ts`
Expected: PASS — 9 tests.

- [ ] **Step 5: Commit**

```bash
git add src/lib/historyFilter.ts src/lib/historyFilter.test.ts
git commit -m "feat(history): pure filterCommits — message/author/hash substring, #N prefix"
```

### Task 4: `selectionAfterFilter` (survie/reset de la sélection)

**Files:**
- Modify: `src/lib/historySelection.ts`
- Test: `src/lib/historySelection.test.ts`

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `src/lib/historySelection.test.ts`, remplacer la ligne 2 :

```ts
import { toggleFilePath, selectionAfterCommitChange, isLocalTip } from './historySelection'
```

par :

```ts
import { toggleFilePath, selectionAfterCommitChange, selectionAfterFilter, isLocalTip } from './historySelection'
```

et ajouter à la fin du fichier :

```ts
describe('selectionAfterFilter', () => {
  const visible = [{ id: 'c2' }, { id: 'c5' }]
  it('keeps the selection while the commit is still visible', () => {
    expect(selectionAfterFilter('c5', visible)).toBe('c5')
  })
  it('resets when the commit is filtered out', () => {
    expect(selectionAfterFilter('c9', visible)).toBeNull()
  })
  it('stays null when nothing was selected, and resets on an empty match list', () => {
    expect(selectionAfterFilter(null, visible)).toBeNull()
    expect(selectionAfterFilter('c2', [])).toBeNull()
  })
})
```

- [ ] **Step 2: Vérifier l'échec**

Run: `npx vitest run src/lib/historySelection.test.ts`
Expected: FAIL — `selectionAfterFilter` is not exported.

- [ ] **Step 3: Implémenter**

Dans `src/lib/historySelection.ts`, ajouter après `selectionAfterCommitChange` (ligne 11) :

```ts
/** Commit selection under an active History filter: it survives while the
 *  commit is still in the visible (filtered) list, and resets otherwise —
 *  same idea as selectionAfterCommitChange for the file preview. */
export function selectionAfterFilter(selectedId: string | null, visible: { id: string }[]): string | null {
  return selectedId !== null && visible.some((c) => c.id === selectedId) ? selectedId : null
}
```

- [ ] **Step 4: Vérifier que tout passe**

Run: `npx vitest run src/lib/historySelection.test.ts`
Expected: PASS — anciens et nouveaux tests.

- [ ] **Step 5: Commit**

```bash
git add src/lib/historySelection.ts src/lib/historySelection.test.ts
git commit -m "feat(history): selectionAfterFilter — selection survives while visible, else resets"
```

### Task 5: History — champ filtre débouncé, compteur, hint

**Files:**
- Modify: `src/lib/History.svelte` (script : import + état filtre ; markup : `ghead` + input + hint ; styles)

Cette tâche pose l'état et l'affichage ; la liste elle-même bascule en Task 6.

- [ ] **Step 1: Import + état débouncé**

Dans `src/lib/History.svelte`, ajouter après la ligne 13 (`import type { CommitFile } from './types'`) :

```ts
  import { filterCommits } from './historyFilter'
```

puis, juste après `const commits = $derived(history.commits)` (ligne 17), ajouter :

```ts
  // Client-side commit filter (P5). Deliberately searches the LOADED commits
  // only — full-history server search (`lore revision find`) is a future lot
  // (see docs/superpowers/specs/2026-07-11-lore-desktop-p5-search-design.md).
  let filterInput = $state('')
  let query = $state('') // debounced copy of filterInput (150 ms)
  $effect(() => {
    const v = filterInput
    const t = setTimeout(() => (query = v), 150)
    return () => clearTimeout(t)
  })
  const filterActive = $derived(query.trim() !== '')
  const filtered = $derived(filterCommits(commits, query))
```

- [ ] **Step 2: Markup — compteur « N of M loaded commits », input, hint**

Remplacer la ligne 225 :

```svelte
    <div class="ghead">History <span class="cnt">{commits.length.toLocaleString()} commits</span></div>
```

par :

```svelte
    <div class="ghead">History <span class="cnt">{filterActive ? `${filtered.length.toLocaleString()} of ${commits.length.toLocaleString()} loaded commits` : `${commits.length.toLocaleString()} commits`}</span></div>
    <input class="filter" bind:value={filterInput} placeholder="Filter commits" />
    {#if filterActive}
      <p class="hint">Searching loaded commits only — scroll History to load more</p>
    {/if}
```

- [ ] **Step 3: Styles**

Dans le `<style>` de `History.svelte`, après la règle `.ghead .cnt { … }`, ajouter :

```css
  .filter { flex: none; display: block; margin: 8px 12px; width: calc(100% - 24px); padding: 6px 9px; background: var(--bg); border: 1px solid var(--border); border-radius: 6px; color: var(--text); font-size: 12px; }
  .hint { flex: none; margin: -2px 14px 6px; font-size: 11px; color: var(--text-dim); }
```

(`.leftcol` est une colonne flex : `flex: none` empêche l'input et le hint d'être compressés par `.glist { flex: 1 }`.)

- [ ] **Step 4: Typecheck**

Run: `npm run check`
Expected: `svelte-check found 0 errors and 0 warnings`.

- [ ] **Step 5: Sanity navigateur mock**

http://localhost:5173, vue History. Taper « loot » : après ~150 ms le compteur devient « N of 200 loaded commits » et le hint apparaît sous le champ (la liste, elle, montre encore tout — elle bascule en Task 6). Vider : compteur « 200 commits », hint disparu.

- [ ] **Step 6: Commit**

```bash
git add src/lib/History.svelte
git commit -m "feat(history): debounced filter field with N-of-M counter and loaded-only hint"
```

### Task 6: History — liste plate filtrée (virtualisation conservée, lanes masquées)

**Files:**
- Modify: `src/lib/History.svelte` (script : dérivés de fenêtre + `win` + `onScroll` + reset scroll ; markup : viewport)

- [ ] **Step 1: Basculer la fenêtre virtuelle sur `rows`**

Remplacer les lignes 155-158 :

```ts
  const total = $derived(commits.length * ROW_H)
  const first = $derived(Math.max(0, Math.floor(scrollTop / ROW_H) - BUFFER))
  const last = $derived(Math.min(commits.length, Math.ceil((scrollTop + viewH) / ROW_H) + BUFFER))
  const windowCommits = $derived(commits.slice(first, last))
```

par :

```ts
  // Under an active filter the list is FLAT (no graph): same virtual window,
  // but over the filtered rows and without the lanes SVG — edges are
  // meaningless on a filtered list.
  const rows = $derived(filterActive ? filtered : commits)
  const total = $derived(rows.length * ROW_H)
  const first = $derived(Math.max(0, Math.floor(scrollTop / ROW_H) - BUFFER))
  const last = $derived(Math.min(rows.length, Math.ceil((scrollTop + viewH) / ROW_H) + BUFFER))
  const windowCommits = $derived(rows.slice(first, last))
```

- [ ] **Step 2: Court-circuiter le calcul des lanes sous filtre**

Dans le `$derived.by` de `win` (ligne 160), ajouter le court-circuit juste après les deux déclarations :

```ts
  const win = $derived.by(() => {
    const edges: { d: string; col: string; dashed: boolean }[] = []
    const dots: { x: number; y: number; color: string; merge: boolean }[] = []
    if (filterActive) return { edges, dots } // flat mode — no lanes
    for (let i = first; i < last; i++) {
```

(le reste de la boucle est inchangé ; en mode normal `rows === commits`, donc les index `first/last` restent alignés avec `commits[i]`).

- [ ] **Step 3: `onScroll` — pas d'auto-load-more sous filtre, et reset scroll sur changement de requête**

Remplacer la fonction `onScroll` (lignes 201-205) :

```ts
  function onScroll() {
    if (!glistEl) return
    scrollTop = glistEl.scrollTop
    if (glistEl.scrollTop + glistEl.clientHeight > commits.length * ROW_H - viewH * 2) loadMoreHistory()
  }
```

par :

```ts
  function onScroll() {
    if (!glistEl) return
    scrollTop = glistEl.scrollTop
    // Infinite scroll only without a filter: a short match list would otherwise
    // sit at the bottom and fetch page after page. Filtered mode loads via the
    // explicit « Load more » button instead.
    if (!filterActive && glistEl.scrollTop + glistEl.clientHeight > rows.length * ROW_H - viewH * 2) loadMoreHistory()
  }

  // A query change re-anchors the list at the top — the previous scroll offset
  // is meaningless against a different row set (also applies when clearing).
  $effect(() => {
    query
    if (glistEl) { glistEl.scrollTop = 0; scrollTop = 0 }
  })
```

- [ ] **Step 4: Markup du viewport — SVG conditionnel, lignes plates, état « no match »**

Remplacer le bloc lignes 227-254 :

```svelte
      {#if loading && !commits.length}
        <p class="muted pad">Loading history…</p>
      {:else}
        <div class="viewport" style="height:{total}px">
          <svg class="graph" style="top:{first * ROW_H}px" width={graphWidth} height={(last - first) * ROW_H} fill="none">
            {#each win.edges as e}<path d={e.d} stroke={e.col} stroke-width="2" stroke-dasharray={e.dashed ? '4 3' : undefined} />{/each}
            {#each win.dots as dt}
              {#if dt.merge}
                <circle cx={dt.x} cy={dt.y} r="6" fill="var(--bg)" stroke={dt.color} stroke-width="2" />
              {:else}
                <circle cx={dt.x} cy={dt.y} r="4.5" fill={dt.color} />
              {/if}
            {/each}
          </svg>
          {#each windowCommits as c, k (c.id)}
            {@const i = first + k}
            {@const av = avatar(c.author)}
            <div class="grow" class:sel={c.id === history.selectedId} role="button" tabindex="0"
                 style="top:{i * ROW_H}px; height:{ROW_H}px; padding-left:{graphWidth + 10}px"
                 onclick={() => (history.selectedId = c.id)}
                 onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); history.selectedId = c.id } }}>
              {#if c.head}<span class="headpill" style="color:{laneColor(c.lane)};border-color:{laneColor(c.lane)}55;background:{laneColor(c.lane)}1f">{c.head}</span>{/if}
              <span class="ava" style="background:{av.bg};color:{av.fg}" title={c.author}>{av.initials}</span>
              <span class="cmid"><span class="cmsg">{c.message}</span><span class="csub" title={new Date(c.whenMs).toLocaleString()}>{shortName(c.author)} · {c.when}</span></span>
            </div>
          {/each}
        </div>
      {/if}
```

par :

```svelte
      {#if loading && !commits.length}
        <p class="muted pad">Loading history…</p>
      {:else if filterActive && filtered.length === 0}
        <p class="muted pad">No commits match.</p>
      {:else}
        <div class="viewport" style="height:{total}px">
          {#if !filterActive}
            <svg class="graph" style="top:{first * ROW_H}px" width={graphWidth} height={(last - first) * ROW_H} fill="none">
              {#each win.edges as e}<path d={e.d} stroke={e.col} stroke-width="2" stroke-dasharray={e.dashed ? '4 3' : undefined} />{/each}
              {#each win.dots as dt}
                {#if dt.merge}
                  <circle cx={dt.x} cy={dt.y} r="6" fill="var(--bg)" stroke={dt.color} stroke-width="2" />
                {:else}
                  <circle cx={dt.x} cy={dt.y} r="4.5" fill={dt.color} />
                {/if}
              {/each}
            </svg>
          {/if}
          {#each windowCommits as c, k (c.id)}
            {@const i = first + k}
            {@const av = avatar(c.author)}
            <div class="grow" class:sel={c.id === history.selectedId} role="button" tabindex="0"
                 style="top:{i * ROW_H}px; height:{ROW_H}px; padding-left:{filterActive ? 14 : graphWidth + 10}px"
                 onclick={() => (history.selectedId = c.id)}
                 onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); history.selectedId = c.id } }}>
              {#if !filterActive && c.head}<span class="headpill" style="color:{laneColor(c.lane)};border-color:{laneColor(c.lane)}55;background:{laneColor(c.lane)}1f">{c.head}</span>{/if}
              <span class="ava" style="background:{av.bg};color:{av.fg}" title={c.author}>{av.initials}</span>
              <span class="cmid"><span class="cmsg">{c.message}</span><span class="csub" title={new Date(c.whenMs).toLocaleString()}>{shortName(c.author)} · {c.when}</span></span>
            </div>
          {/each}
        </div>
      {/if}
```

(Lignes plates = avatar + message + auteur·date, conformes à la spec « avatar + message + date » ; le headpill et le gutter du graphe disparaissent en mode filtre.)

- [ ] **Step 5: Typecheck**

Run: `npm run check`
Expected: `svelte-check found 0 errors and 0 warnings`.

- [ ] **Step 6: Sanity navigateur mock**

http://localhost:5173, History. « loot » → liste PLATE (pas de lanes/points SVG, pas de headpill, lignes alignées à gauche), scroll fluide dans les matches, compteur cohérent ; scroller tout en bas de la liste filtrée ne charge PAS de page (compteur M reste 200). « zzz » → « No commits match. ». Vider → graphe + lanes de retour, scroll infini de nouveau actif, liste remontée en haut.

- [ ] **Step 7: Commit**

```bash
git add src/lib/History.svelte
git commit -m "feat(history): flat virtualized filtered list — lanes hidden, no auto-load under filter"
```

### Task 7: History — survie/reset de la sélection + Escape

**Files:**
- Modify: `src/lib/History.svelte` (script : import + effect sélection ; markup : `onkeydown` de l'input)

- [ ] **Step 1: Étendre l'import de historySelection**

Remplacer la ligne 12 :

```ts
  import { toggleFilePath, selectionAfterCommitChange, isLocalTip } from './historySelection'
```

par :

```ts
  import { toggleFilePath, selectionAfterCommitChange, selectionAfterFilter, isLocalTip } from './historySelection'
```

- [ ] **Step 2: Effect de survie/reset**

Ajouter après le bloc `const filtered = $derived(filterCommits(commits, query))` (posé en Task 5) :

```ts
  // Selection survives filtering while the commit stays visible, resets
  // otherwise (spec — pattern selectionAfterCommitChange). Guarded write so
  // the effect settles instead of looping.
  $effect(() => {
    if (!filterActive) return
    const next = selectionAfterFilter(history.selectedId, filtered)
    if (next !== history.selectedId) history.selectedId = next
  })
```

- [ ] **Step 3: Escape vide le filtre (focus dans le champ uniquement)**

Remplacer l'input posé en Task 5 :

```svelte
    <input class="filter" bind:value={filterInput} placeholder="Filter commits" />
```

par :

```svelte
    <input class="filter" bind:value={filterInput} placeholder="Filter commits"
           onkeydown={(e) => { if (e.key === 'Escape') { filterInput = ''; query = '' } }} />
```

(`query = ''` court-circuite le débounce pour un clear instantané. La garde existante du preview panel — le listener window de `History.svelte` ignore Escape quand `e.target instanceof HTMLInputElement` — reste intacte : Escape dans le champ filtre ne ferme PAS le panel, Escape hors input le ferme toujours, et l'éditeur de message garde son propre Escape. NE PAS modifier ce listener.)

- [ ] **Step 4: Typecheck**

Run: `npm run check`
Expected: `svelte-check found 0 errors and 0 warnings`.

- [ ] **Step 5: Sanity navigateur mock**

http://localhost:5173, History :
- Filtrer « loot », cliquer un match → détail affiché ; affiner la requête pour que le commit sélectionné reste matché (ex. « loot t ») → sélection conservée (ligne surlignée) ; requête qui l'exclut (ex. « lighting ») → détail repasse à « Select a commit. ».
- Focus dans le champ + Escape → champ vidé, graphe de retour.
- Sélectionner un commit, ouvrir la preview d'un fichier, focus dans le champ filtre, Escape → le filtre se vide mais la preview RESTE ouverte ; Escape avec le focus ailleurs → la preview se ferme (comportement P2 inchangé).
- « Edit message » sur le commit de tête : Escape dans l'input d'édition annule l'édition sans toucher au filtre.

- [ ] **Step 6: Commit**

```bash
git add src/lib/History.svelte
git commit -m "feat(history): selection survives filter while visible; Escape clears the field"
```

### Task 8: History — bouton « Load more » sous filtre

**Files:**
- Modify: `src/lib/History.svelte` (script : garde in-flight ; markup : bouton ; styles)

- [ ] **Step 1: Garde in-flight locale**

Ajouter dans le script, après la fonction `onScroll` :

```ts
  // « Load more » under an active filter (spec: the button stays reachable at
  // the bottom of the filtered list; newly loaded commits enter the filter
  // automatically since `filtered` derives from `commits`). Local in-flight
  // guard: loadMoreHistory has none, and a double-click would append the same
  // page twice.
  let loadingMore = $state(false)
  async function clickLoadMore() {
    if (loadingMore) return
    loadingMore = true
    try { await loadMoreHistory() } finally { loadingMore = false }
  }
```

- [ ] **Step 2: Bouton en bas de la liste filtrée**

Dans le markup, à l'intérieur de `.glist`, juste après le `{/if}` qui clôt la chaîne `{#if loading …}{:else if …}{:else}…{/if}` (posée en Task 6), ajouter :

```svelte
      {#if filterActive && !loading && history.cursor}
        <button class="loadmore" onclick={clickLoadMore} disabled={loadingMore}>{loadingMore ? 'Loading…' : 'Load more'}</button>
      {/if}
```

(Placé HORS de la chaîne pour rester visible aussi sur « No commits match. » — on peut charger plus de commits pour chercher plus loin. `history.cursor` est `null` en fin d'historique : le bouton disparaît alors.)

- [ ] **Step 3: Style**

Dans le `<style>`, après la règle `.hint { … }`, ajouter :

```css
  .loadmore { display: block; margin: 10px auto 14px; padding: 5px 16px; font-size: 12px; }
```

- [ ] **Step 4: Typecheck**

Run: `npm run check`
Expected: `svelte-check found 0 errors and 0 warnings`.

- [ ] **Step 5: Sanity navigateur mock**

http://localhost:5173, History. Filtrer « recoil » (peu de matches) → bouton « Load more » sous la liste ; clic → « Loading… » bref, puis compteur « N of 400 loaded commits » et de nouveaux matches entrent dans la liste sans toucher au champ. Filtrer « zzz » → « No commits match. » + bouton toujours là. Sans filtre → aucun bouton (le scroll infini fait le travail).

- [ ] **Step 6: Commit**

```bash
git add src/lib/History.svelte
git commit -m "feat(history): explicit Load more under filter, in-flight guarded"
```

---

### Task 9: Vérification finale

**Files:** aucun (vérification pure).

- [ ] **Step 1: Suite vitest complète**

Run: `npx vitest run`
Expected: `Test Files  18 passed` (16 existants + `historyFilter.test.ts` ; `changesPartition.test.ts` et `historySelection.test.ts` étendus), 0 failed.

- [ ] **Step 2: Typecheck complet**

Run: `npm run check`
Expected: `svelte-check found 0 errors and 0 warnings` et `tsc` sans erreur.

- [ ] **Step 3: Parcours navigateur mock complet**

`npm run dev` → http://localhost:5173, puis dérouler et consigner :

1. **Locks** : compteur « 3 held » ; « maya » → « 1 of 3 » (détenteur) ; « hero » → « 1 of 3 » (chemin) ; « ZZZ » → « No locks match. » ; clear → 3 lignes. Le bouton « + Lock a file… » et le menu contextuel restent fonctionnels sous filtre.
2. **History — filtre de base** : « loot » → liste plate (aucune lane/point/headpill), compteur « N of 200 loaded commits », hint « Searching loaded commits only — scroll History to load more » sous le champ, liste remontée en haut ; débounce perceptible (~150 ms) en tapant vite.
3. **History — #N** : « #49 » → seuls des commits dont le rev commence par 49 (revs 4900-4999 dans la première page mock) ; l'entête du détail confirme le `#rev` d'un match cliqué.
4. **History — sélection** : match cliqué puis requête affinée le gardant visible → sélection conservée ; requête l'excluant → « Select a commit. » ; clear → graphe de retour.
5. **History — Escape** : Escape dans le champ → filtre vidé ; preview fichier ouverte + Escape dans le champ → preview INTACTE ; Escape hors input → preview fermée ; Escape dans l'éditeur de message → annule l'édition seulement.
6. **History — Load more sous filtre** : bouton visible en bas de liste filtrée (y compris sur zéro match), clic → compteur M passe à 400 et les nouveaux matches apparaissent ; scroller au fond de la liste filtrée ne déclenche PAS de chargement automatique ; sans filtre, le scroll infini fonctionne comme avant (M grimpe en scrollant).
7. **Non-régression** : vue Changes (filtre existant, staging, commit) et graphe History hors filtre (lanes, merge dots, scroll fluide sur 200+ commits) inchangés.

- [ ] **Step 4: Commit final (si des retouches de vérification ont eu lieu)**

```bash
git add -A
git commit -m "chore(p5): final verification pass for locks filter + history search"
```

---

## Self-review (fait à l'écriture du plan)

- **Couverture spec** : filtre Locks path+holder + compteur (Tasks 1-2) ; `filterCommits` pur + tests casse/hash/#N/vide (Task 3) ; champ débouncé 150 ms pattern Changes + compteur « N of M loaded commits » + hint verbatim (Task 5) ; liste plate sans lanes sous filtre (Task 6) ; survie/reset sélection pattern selectionAfterCommitChange (Tasks 4+7) ; Escape sans casser preview/éditeur (Task 7) ; Load more sous filtre alimentant le filtre (Task 8) ; documentation du choix client-side dans le doc-comment de `historyFilter.ts` + commentaire composant (Tasks 3+5) ; vérif navigateur des deux vues (Task 9). Hors périmètre respecté (pas de `revision find`, pas de tri).
- **Placeholders** : aucun — chaque étape code porte le code complet, chaque commande son résultat attendu.
- **Cohérence de types** : `filterByText<T>` (Task 1) utilisé avec `LockEntry` en Task 2 ; `filterCommits(commits, query)` (Task 3) consommé en Task 5 ; `selectionAfterFilter(selectedId, visible)` (Task 4) consommé en Task 7 ; `rows`/`filterActive` définis en Tasks 5-6 avant usage en Tasks 7-8.

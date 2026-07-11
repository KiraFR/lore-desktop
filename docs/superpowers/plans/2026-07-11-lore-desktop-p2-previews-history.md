# Lore Desktop — Lot P2 « previews & history » Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **STATUT : EXÉCUTÉ ET VÉRIFIÉ le 2026-07-11** (19/19 tâches, vérification finale
> Task 19 passée). Suites : vitest 119 (16 fichiers), cargo 98, svelte-check 0 erreur
> / 0 warning. Parcours navigateur mock complet : préchargement History (commits déjà
> là, pas de « Loading history… ») ; preview History (clic/Enter/Escape/re-clic,
> icône .cpp, mention working-copy sur commit ancien, absente sur le tip, « No longer
> in the working copy » sur delete, reset au changement de commit) ; vue Changes
> inchangée (diff texte, compare binaire avec vignettes, lock, discard, timeline) ;
> merge feature/loot → cartes binaires avec vignettes des DEUX côtés + conflit texte
> avec mini-diff, bannière « Merging feature/loot into main », reprise neutre
> « Resolving merge into main » + « Theirs · incoming », abort externe (levier
> localStorage) → toast « Merge was aborted outside the app » + retour setup, abort
> LOCAL sans ce toast ; clone « Cloning… 17% — 8.0 MB / 48.0 MB » progressant
> (17 % → 75 % → fini) + boutons de clone de l'autre surface désactivés pendant le
> vol (garde cross-surface RepoPicker ↔ RepoSwitcher).
> Déviations conscientes du lot : constat Task 12 — un merge conflictuel réel produit
> 3 sidecars pour un conflit texte et 2 pour un binaire, PAS de sidecar `~mine` côté
> binaire (le disque tient le rôle de mine) ; le mini-diff des cartes Merge est le
> patch avec marqueurs de conflit bruts (assumé) ; fix `decode()` ImageReader
> nécessaire hors plan dans `preview.rs` ; Task 11 : idiome `div.rowmain`
> `role="button"` pour l'a11y des lignes fichiers ; win32job 2.0.3 exige `&mut` sur
> `set_extended_limit_info` — signature ajustée. Limite mock relevée en Task 19 :
> vignette image / lecteur audio non exerçables dans la preview History (le
> générateur `buildBigHistory` ne produit que des fichiers `.cpp` — nf=1 donc ext
> k=0) ; le cas icône est vérifié en History, le cas vignette/audio est couvert côté
> Changes et Merge.
> Kill dur réel : `app.exe` (src-tauri/target/debug) tué par `Stop-Process -Force`
> avec le subscriber `lore.exe` actif (1 process avant) → 3 s après, AUCUN process
> `lore` (Job object kill-on-close OK), aucun app/lore/vite résiduel.

**Goal:** Livrer les 6 items du lot P2 (spec `docs/superpowers/specs/2026-07-11-lore-desktop-p2-previews-history-design.md`) : préchargement de l'History, finitions progression (libellé « X / Y » + garde globale anti double-clone), corrections de la vue Merge (libellés neutres + récupération d'un abort externe), preview de fichier dans History (avec extraction de composants partagés depuis FilePreview), unification des previews Merge (vignettes réelles Mine/Theirs via le sidecar `~theirs`, mini-diff texte), et Job object Windows pour que les sidecars `lore notification subscribe` meurent avec l'app.

**Architecture:** Frontend Svelte 5 : la logique nouvelle vit dans des modules purs testés vitest (`progress.ts`, `mergeLogic.ts`, `historySelection.ts`, `fileTypes.ts`, `previewKind.ts`), le markup dans les composants. FilePreview.svelte est découpé en trois composants partagés (`MediaPreview`, `FileHistorySection`, `DiffBlock`) SANS changement de comportement côté Changes ; `HistoryFilePreview.svelte` et les cartes Merge les recomposent. Backend Rust : une seule vraie nouveauté (`src-tauri/src/job.rs`, Job object kill-on-close via la crate `win32job` — `windows-rs` n'est pas dans l'arbre de dépendances, vérifié dans Cargo.toml) + un strip du suffixe `~theirs` dans `preview.rs`. Le mock fait vivre chaque nouveauté en dev navigateur.

**Tech Stack:** Svelte 5 runes, TypeScript, vitest (jsdom), Rust (Tauri v2), crate `win32job` (Windows uniquement, no-op ailleurs), PowerShell pour les vérifications réelles.

---

## Contexte & conventions (à lire avant toute tâche)

- **Repo de test réel** : working copy `C:\Users\jimmy\lore-test-repo` (repo `desktoptest1`) sur `lore://lore.example.com:41337`. Binaire CLI : `C:\Users\jimmy\bin\lore.exe`. Le scénario de merge conflictuel (Task 12) est entièrement local (branch create/commit/merge start/abort) — il marche même serveur down.
- **Gotcha cwd** : toute commande `lore` file-scoped (`diff`, `file history`, `merge resolve`…) résout les chemins relatifs contre le cwd du process, PAS contre `--repository`. Toujours passer des chemins **absolus** (voir `lore_diff`, `lore_merge_resolve` dans `src-tauri/src/commands.rs`).
- **Contrainte vitest** : `vitest.config.ts` n'a PAS le plugin Svelte — les tests ne peuvent importer NI un `.svelte` NI un `.svelte.ts` à runes (`$state` ne serait pas compilé). Toute logique à tester vit donc dans des modules purs (`progress.ts`, `mergeLogic.ts`, etc.) ; le wiring composants/stores est vérifié navigateur (Task 19).
- **Commandes de test** :
  - Vitest ciblé : `npx vitest run src/lib/<fichier>.test.ts` — attendu `Test Files  1 passed`.
  - Vitest complet : `npx vitest run` — attendu `Test Files  N passed`, 0 failed.
  - Rust : `cargo test --manifest-path src-tauri/Cargo.toml <filtre>` — attendu `test result: ok`.
  - Typecheck : `npm run check` — attendu `svelte-check found 0 errors and 0 warnings`.
  - Dev navigateur (mock) : `npm run dev` → http://localhost:5173 (l'API mock est active hors Tauri).
- **Hypothèse pinnée en Task 12** : le nommage exact du sidecar « theirs » (`<nom>~theirs`, constaté lors des captures P1). Les Tasks 13, 14, 15 et 17 utilisent la constante `~theirs` — **si la Task 12 constate un autre nommage, adapter ces quatre tâches avant de les exécuter** (une constante par couche : `THEIRS_SUFFIX` dans `previewKind.ts` et dans `preview.rs`).
- Ordre de livraison imposé par la spec : Item 3 (Task 1), Item 5 (Tasks 2–3), Item 4 (Tasks 4–5), Item 1 (Tasks 6–11), Item 2 (Tasks 12–17), Item 6 (Task 18), vérification finale (Task 19).

## Carte des fichiers

**Créés :**
- `src/lib/fileTypes.ts` (+ `fileTypes.test.ts`) — classification type humain d'un chemin (extrait de FilePreview).
- `src/lib/historySelection.ts` (+ `historySelection.test.ts`) — sélection d'un fichier de commit (toggle, reset, tip local).
- `src/lib/mergeLogic.ts` (+ `mergeLogic.test.ts`) — libellés de merge à source inconnue + détecteur d'abort externe.
- `src/lib/FileHistorySection.svelte` — timeline de révisions d'un fichier (extrait de FilePreview).
- `src/lib/MediaPreview.svelte` — rendu média image/audio/3D de la copie de travail (extrait de FilePreview).
- `src/lib/DiffBlock.svelte` — rendu de lignes de diff (extrait de FilePreview).
- `src/lib/HistoryFilePreview.svelte` — panneau preview allégé de la vue History.
- `src-tauri/src/job.rs` — Job object Windows kill-on-close (no-op non-Windows).

**Modifiés :**
- `src/App.svelte` — préchargement History au changement de repo.
- `src/lib/progress.ts` (+ test) — `cloneProgressLabel` (« X / Y »), `cloneInFlight` (garde globale).
- `src/lib/repoActions.ts`, `src/lib/RepoPicker.svelte`, `src/lib/RepoSwitcher.svelte` — garde globale + libellé octets.
- `src/lib/Merge.svelte` — libellés neutres, abort externe, vignettes réelles, mini-diff.
- `src/lib/History.svelte` — lignes de fichiers sélectionnables + panneau.
- `src/lib/FilePreview.svelte` — recomposé sur les extraits (comportement Changes inchangé).
- `src/lib/previewKind.ts` (+ test) — helpers `~theirs`.
- `src/lib/mock.ts` (+ `mock.test.ts`) — conflit texte, previews sidecar, levier « abort externe ».
- `src-tauri/src/preview.rs` — `preview_ext` (strip `~theirs`).
- `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml` — module job + dépendance win32job.
- `src-tauri/tests/fixtures/README.md` — nommage sidecar pinné (Task 12).

---

## Item 3 — Préchargement de l'History

### Task 1: Précharger l'History à l'ouverture du repo

**Files:**
- Modify: `src/App.svelte:5` (import) et `src/App.svelte:38-42` (effect changement de repo)

- [ ] **Step 1: Ajouter le préchargement dans l'effect de changement de repo**

Dans `src/App.svelte`, remplacer l'import ligne 5 :

```ts
import { repo, refreshStatus } from './lib/repo.svelte'
```

par :

```ts
import { repo, refreshStatus, refreshHistory } from './lib/repo.svelte'
```

et remplacer l'effect lignes 38-42 :

```ts
  // Reload whenever the selected repository changes. refreshStatus also refreshes
  // locks + branches in the background, so they never block the initial render.
  $effect(() => {
    session.config.currentRepo
    refreshStatus()
    loadIdentity()
  })
```

par :

```ts
  // Reload whenever the selected repository changes. refreshStatus also refreshes
  // locks + branches in the background, so they never block the initial render.
  // refreshHistory warms the History cache so entering that view is instant —
  // silent: one extra background CLI call, a failure just falls back to the
  // in-view load (History.svelte keeps its own refresh effect).
  $effect(() => {
    session.config.currentRepo
    refreshStatus()
    loadIdentity()
    refreshHistory(true)
  })
```

`refreshHistory` gère déjà le changement de repo (drop du cache périmé quand `history.repoPath !== path`, voir `src/lib/repo.svelte.ts:41-44`) — rien d'autre à toucher.

- [ ] **Step 2: Typecheck**

Run: `npm run check`
Expected: `svelte-check found 0 errors and 0 warnings`

- [ ] **Step 3: Commit**

```bash
git add src/App.svelte
git commit -m "feat(history): preload history when a repository opens"
```

---

## Item 5 — Finitions progression

### Task 2: Helpers purs `cloneProgressLabel` + `cloneInFlight` (TDD)

**Files:**
- Modify: `src/lib/progress.ts`
- Test: `src/lib/progress.test.ts`

- [ ] **Step 1: Écrire les tests qui échouent**

Ajouter à la fin de `src/lib/progress.test.ts` (et étendre l'import existant ligne 2) :

```ts
import { pct, cloneLabel, cloneProgressLabel, cloneInFlight } from './progress'
```

```ts
describe('cloneProgressLabel', () => {
  it('appends done/total sizes for byte-counted progress', () => {
    expect(cloneProgressLabel({ done: 12 * 1024 * 1024, total: 48 * 1024 * 1024, unit: 'bytes' }))
      .toBe('Cloning… 25% — 12.0 MB / 48.0 MB')
  })
  it('falls back to the plain label for non-byte or indeterminate progress', () => {
    expect(cloneProgressLabel({ done: 3, total: 6, unit: 'files' })).toBe('Cloning… 50%')
    expect(cloneProgressLabel({ done: 0 })).toBe('Cloning…')
    expect(cloneProgressLabel(null)).toBe('Cloning…')
  })
})

describe('cloneInFlight', () => {
  it('is true for any occupied slot — including the pre-first-tick sentinel', () => {
    expect(cloneInFlight(null)).toBe(false)
    expect(cloneInFlight({ done: 0 })).toBe(true)
    expect(cloneInFlight({ done: 1, total: 2, unit: 'bytes' })).toBe(true)
  })
})
```

- [ ] **Step 2: Vérifier l'échec**

Run: `npx vitest run src/lib/progress.test.ts`
Expected: FAIL — `cloneProgressLabel is not a function` (ou erreur d'import équivalente).

- [ ] **Step 3: Implémenter**

Dans `src/lib/progress.ts`, ajouter l'import et les deux fonctions à la fin du fichier :

```ts
import { fmtSize } from './sizeFormat'
```

```ts
/** Full clone label: « Cloning… 42% — 12.0 MB / 48.0 MB » when the progress is
 *  byte-counted with a known total; plain percentage/indeterminate otherwise.
 *  Sync/push deliberately keep a bar without text (P1 decision, see TitleBar). */
export function cloneProgressLabel(p: OpProgress | null): string {
  const percent = pct(p)
  if (percent === null || !p || p.unit !== 'bytes' || !p.total) return cloneLabel(percent)
  return `${cloneLabel(percent)} — ${fmtSize(p.done)} / ${fmtSize(p.total)}`
}

/** Global anti-double-clone guard: any occupied slot means a clone is in
 *  flight somewhere (RepoPicker or RepoSwitcher). repoActions parks a
 *  `{ done: 0 }` sentinel in the slot before the folder pick, so the guard
 *  holds even before the first real tick. */
export function cloneInFlight(p: OpProgress | null): boolean {
  return p !== null
}
```

- [ ] **Step 4: Vérifier le passage**

Run: `npx vitest run src/lib/progress.test.ts`
Expected: `Test Files  1 passed` (tous les tests, anciens inclus).

- [ ] **Step 5: Commit**

```bash
git add src/lib/progress.ts src/lib/progress.test.ts
git commit -m "feat(progress): clone byte label and global clone-guard helpers"
```

### Task 3: Câbler le libellé « X / Y » et la garde globale dans les deux surfaces

**Files:**
- Modify: `src/lib/repoActions.ts:29-45` (cloneServerRepo)
- Modify: `src/lib/RepoPicker.svelte:7,36-44,69-70`
- Modify: `src/lib/RepoSwitcher.svelte:8,55-63,114-118`

- [ ] **Step 1: Poser la garde dans `cloneServerRepo`**

Dans `src/lib/repoActions.ts`, ajouter l'import :

```ts
import { cloneInFlight } from './progress'
```

et remplacer intégralement `cloneServerRepo` (lignes 29-45) par :

```ts
/**
 * Pick a destination parent folder, clone the server repo into it, then add it
 * to the known list and switch to it. Returns true when the app switched repos.
 *
 * Global anti-double-clone guard: one clone at a time across ALL surfaces
 * (RepoPicker and RepoSwitcher can coexist and used to race each other into
 * the single `opProgress.clone` slot). The slot doubles as the flag — a
 * `{ done: 0 }` sentinel occupies it from before the folder pick, so a second
 * surface can't slip in while the dialog is open.
 */
export async function cloneServerRepo(entry: RepoEntry): Promise<boolean> {
  if (cloneInFlight(opProgress.clone)) return false
  opProgress.clone = { done: 0 } // indeterminate until the first real tick
  try {
    const parent = await api.pickFolder()
    if (!parent) return false // cancelled
    const path = await api.cloneRepo(
      session.config.serverUrl!, entry.id, entry.name, parent,
      (p) => { opProgress.clone = p },
    )
    await selectRepo(path)
    return true
  } catch (e) {
    toastError('Clone failed', e)
    return false
  } finally {
    opProgress.clone = null
  }
}
```

(Note : `await api.pickFolder()` monte DANS le try pour que le finally libère le slot même sur annulation.)

- [ ] **Step 2: RepoPicker — libellé + désactivation croisée**

Dans `src/lib/RepoPicker.svelte` :

1. Ligne 7, remplacer `import { pct, cloneLabel } from './progress'` par :

```ts
import { pct, cloneProgressLabel, cloneInFlight } from './progress'
```

2. Le bouton Clone (lignes 69-71) devient :

```svelte
        <button onclick={() => cloneRepo(r)} disabled={!!busy || cloneInFlight(opProgress.clone)}>
          {busy === `clone:${r.id}` ? cloneProgressLabel(opProgress.clone) : 'Clone…'}
        </button>
```

(La `.clonebar` existante juste en dessous reste inchangée — `pct` est toujours importé pour elle.)

- [ ] **Step 3: RepoSwitcher — libellé + désactivation croisée**

Dans `src/lib/RepoSwitcher.svelte` :

1. Ligne 8, remplacer `import { pct, cloneLabel } from './progress'` par :

```ts
import { pct, cloneProgressLabel, cloneInFlight } from './progress'
```

2. Le bouton de clone (ligne 114) devient :

```svelte
          <button class="item" onclick={() => onClone(r)} disabled={!!busy || cloneInFlight(opProgress.clone)}>
```

3. La ligne du libellé (ligne 118) devient :

```svelte
              <span class="rp">{busy === `clone:${r.id}` ? cloneProgressLabel(opProgress.clone) : r.id.slice(0, 12) + '…'}</span>
```

- [ ] **Step 4: Typecheck + suite**

Run: `npm run check && npx vitest run`
Expected: `svelte-check found 0 errors and 0 warnings` puis toutes les suites vertes.

- [ ] **Step 5: Commit**

```bash
git add src/lib/repoActions.ts src/lib/RepoPicker.svelte src/lib/RepoSwitcher.svelte
git commit -m "feat(progress): clone X/Y byte label and cross-surface clone guard"
```

---

## Item 4 — Corrections de la vue Merge

### Task 4: Module pur `mergeLogic.ts` (TDD)

**Files:**
- Create: `src/lib/mergeLogic.ts`
- Test: `src/lib/mergeLogic.test.ts`

- [ ] **Step 1: Écrire les tests qui échouent**

Créer `src/lib/mergeLogic.test.ts` :

```ts
import { describe, it, expect } from 'vitest'
import { mergeWording, externalAbortStep } from './mergeLogic'

describe('mergeWording', () => {
  it('uses the real source when it is known', () => {
    const w = mergeWording('feature/loot', 'main')
    expect(w.banner).toBe('Merging feature/loot into main')
    expect(w.commitMessage).toBe('Merge feature/loot into main')
    expect(w.done).toBe('feature/loot was merged into main.')
    expect(w.theirsCard).toBe('Theirs · feature/loot')
  })
  it('never shows a guessed name when the source is unknown', () => {
    const w = mergeWording(null, 'main')
    expect(w.banner).toBe('Resolving merge into main')
    expect(w.commitMessage).toBe('Merge into main')
    expect(w.done).toBe('A branch was merged into main.')
    expect(w.theirsCard).toBe('Theirs · incoming')
  })
})

describe('externalAbortStep', () => {
  it('latches once the backend confirms the merge, then flags a later false', () => {
    let s = externalAbortStep(true, true, false)
    expect(s).toEqual({ saw: true, aborted: false })
    s = externalAbortStep(true, false, s.saw)
    expect(s).toEqual({ saw: false, aborted: true })
  })
  it('ignores a false before the merge was ever confirmed (status lag)', () => {
    expect(externalAbortStep(true, false, false)).toEqual({ saw: false, aborted: false })
    expect(externalAbortStep(true, undefined, false)).toEqual({ saw: false, aborted: false })
  })
  it('resets outside the resolving phase', () => {
    expect(externalAbortStep(false, true, true)).toEqual({ saw: false, aborted: false })
  })
})
```

- [ ] **Step 2: Vérifier l'échec**

Run: `npx vitest run src/lib/mergeLogic.test.ts`
Expected: FAIL — `Cannot find module './mergeLogic'`.

- [ ] **Step 3: Implémenter**

Créer `src/lib/mergeLogic.ts` :

```ts
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
```

- [ ] **Step 4: Vérifier le passage**

Run: `npx vitest run src/lib/mergeLogic.test.ts`
Expected: `Test Files  1 passed`.

- [ ] **Step 5: Commit**

```bash
git add src/lib/mergeLogic.ts src/lib/mergeLogic.test.ts
git commit -m "feat(merge): pure wording and external-abort helpers"
```

### Task 5: Câbler Merge.svelte (libellés neutres + abort externe) et le levier mock

**Files:**
- Modify: `src/lib/Merge.svelte`
- Modify: `src/lib/mock.ts:230-243` (getStatus)
- Test: `src/lib/mock.test.ts`

- [ ] **Step 1: Test mock qui échoue (levier abort externe)**

Ajouter à la fin du describe `'mock status flags'` de `src/lib/mock.test.ts` :

```ts
  it('external-abort dev lever clears the merge on the next status', async () => {
    await mock.mergeStart('C:/repos/extabort', 'feature/loot')
    localStorage.setItem('loredesktop.mock.externalAbort', '1')
    const s = await mock.getStatus('C:/repos/extabort')
    expect(s.mergeInProgress).toBe(false)
    expect(localStorage.getItem('loredesktop.mock.externalAbort')).toBeNull()
  })
```

Run: `npx vitest run src/lib/mock.test.ts`
Expected: FAIL — `expected true to be false` (le levier n'existe pas encore).

- [ ] **Step 2: Implémenter le levier dans le mock**

Dans `src/lib/mock.ts`, au début de `async getStatus(repoPath: string)` (juste après `await delay(250)`) :

```ts
    // Dev lever: simulate an out-of-app `branch merge abort` — run
    // `localStorage.setItem('loredesktop.mock.externalAbort', '1')` in the
    // devtools, then refocus the window (the focus refresh picks it up).
    if (localStorage.getItem('loredesktop.mock.externalAbort') === '1') {
      localStorage.removeItem('loredesktop.mock.externalAbort')
      mergeConflictState = []
    }
```

Run: `npx vitest run src/lib/mock.test.ts`
Expected: `Test Files  1 passed`.

- [ ] **Step 3: Câbler Merge.svelte**

Dans `src/lib/Merge.svelte` :

1. Remplacer les imports lignes 4-5 :

```ts
import { refreshStatus } from './repo.svelte'
import { toastError } from './toast'
```

par :

```ts
import { repo, refreshStatus } from './repo.svelte'
import { toastError, toastInfo } from './toast'
import { mergeWording, externalAbortStep } from './mergeLogic'
```

2. Après la déclaration `let busy = ...` (ligne 20), ajouter :

```ts
  // Whether `source` is trustworthy: confirmed by the user in THIS session's
  // setup phase. A merge resumed from outside the app starts with a guessed
  // default — mergeWording() then falls back to neutral labels.
  let sourceKnown = $state(false)
```

3. Après la ligne `const others = $derived(...)` (ligne 24), ajouter :

```ts
  const wording = $derived(mergeWording(sourceKnown ? source : null, target))
```

4. Dans `doMerge()`, première ligne du `try` : remplacer

```ts
      await api.mergeBranch(p, source, `Merge ${source} into ${target}`)
```

par :

```ts
      sourceKnown = true
      await api.mergeBranch(p, source, `Merge ${source} into ${target}`)
```

5. Dans `startMerge()`, première ligne du `try` : ajouter `sourceKnown = true` avant `await api.mergeStart(p, source)` :

```ts
      sourceKnown = true
      await api.mergeStart(p, source)
```

6. Dans `complete()` : remplacer

```ts
      await api.mergeCommit(p, `Merge ${source} into ${target}`)
```

par :

```ts
      await api.mergeCommit(p, wording.commitMessage)
```

7. Après l'effect de reprise de merge (celui qui appelle `api.mergeConflicts`, lignes 40-51), ajouter le watcher d'abort externe :

```ts
  // If `branch merge abort` lands at the CLI while this view is resolving,
  // the conflicts are gone server-side but the view would stay stuck. Watch
  // the shared status: a confirmed merge that flips to false without a local
  // action (busy) means it ended outside the app → back to setup + toast.
  let sawMerge = false
  $effect(() => {
    const step = externalAbortStep(phase === 'resolving', repo.status?.mergeInProgress, sawMerge)
    sawMerge = step.saw
    if (step.aborted && !busy) {
      phase = 'setup'
      conflicts = []
      resolvedSide = {}
      selectedPath = null
      toastInfo('Merge was aborted outside the app')
    }
  })
```

(Le retour en phase `setup` relance `loadPreview()` tout seul : l'effect existant `$effect(() => { source; target; if (phase === 'setup') loadPreview() })` lit `phase`.)

8. Dans le markup, la warnbar (ligne 184) devient :

```svelte
        <Icon name="merge" size={15} /> {wording.banner} — {unresolvedCount} of {conflicts.length} to resolve.
```

9. Le titre de la carte Theirs (ligne 214) devient :

```svelte
                <div class="vhd">{wording.theirsCard}</div>
```

10. Dans la doneview, le paragraphe (ligne 240) devient :

```svelte
        <p class="muted">{wording.done}</p>
```

(Le `<h3>Merged into {target}</h3>` reste tel quel — `target` est toujours fiable.)

- [ ] **Step 4: Typecheck + suite**

Run: `npm run check && npx vitest run`
Expected: 0 erreur svelte-check, toutes suites vertes.

- [ ] **Step 5: Commit**

```bash
git add src/lib/Merge.svelte src/lib/mergeLogic.ts src/lib/mock.ts src/lib/mock.test.ts
git commit -m "fix(merge): neutral labels for resumed merges, recover from external abort"
```

---

## Item 1 — Preview de fichier dans la vue History

### Task 6: Extraire `fileTypes.ts` (TDD)

**Files:**
- Create: `src/lib/fileTypes.ts`
- Test: `src/lib/fileTypes.test.ts`
- Modify: `src/lib/FilePreview.svelte:86-98` (supprimer TYPES/ext/typeName, importer)

- [ ] **Step 1: Écrire les tests qui échouent**

Créer `src/lib/fileTypes.test.ts` :

```ts
import { describe, it, expect } from 'vitest'
import { ext, typeName } from './fileTypes'

describe('ext', () => {
  it('lowercases the extension and handles missing ones', () => {
    expect(ext('Content/T_Rock.PNG')).toBe('png')
    expect(ext('Makefile')).toBe('')
  })
})

describe('typeName', () => {
  it('maps known asset extensions to human names', () => {
    expect(typeName('Content/Maps/Level_01.umap')).toBe('Level (map)')
    expect(typeName('Content/Hero/SK_Hero.uasset')).toBe('Unreal asset')
    expect(typeName('Audio/sfx_hit.wav')).toBe('Audio')
  })
  it('falls back to "<EXT> file" then "File"', () => {
    expect(typeName('data.xyz')).toBe('XYZ file')
    expect(typeName('LICENSE')).toBe('File')
  })
})
```

- [ ] **Step 2: Vérifier l'échec**

Run: `npx vitest run src/lib/fileTypes.test.ts`
Expected: FAIL — `Cannot find module './fileTypes'`.

- [ ] **Step 3: Implémenter (extraction à l'identique depuis FilePreview)**

Créer `src/lib/fileTypes.ts` (le contenu de la map vient de `FilePreview.svelte:87-97`, inchangé) :

```ts
/** Human-readable asset type names, shared by the Changes and History previews. */
const TYPES: Record<string, string> = {
  uasset: 'Unreal asset', umap: 'Level (map)', pak: 'Unreal package',
  cpp: 'C++ source', h: 'C++ header', cs: 'C# source', ini: 'Config', md: 'Markdown', json: 'JSON',
  png: 'Texture', tga: 'Texture', dds: 'Texture', tif: 'Texture', tiff: 'Texture', jpg: 'Texture', jpeg: 'Texture', webp: 'Texture',
  exr: 'HDR texture', hdr: 'HDR texture', psd: 'Photoshop document',
  fbx: 'Mesh', obj: 'Mesh', abc: 'Alembic cache', gltf: 'Mesh', glb: 'Mesh',
  blend: 'Blender scene', ma: 'Maya scene', mb: 'Maya scene', max: '3ds Max scene', ztl: 'ZBrush tool',
  sbs: 'Substance graph', sbsar: 'Substance archive', spp: 'Substance Painter project',
  wav: 'Audio', ogg: 'Audio', mp3: 'Audio', flac: 'Audio', bank: 'Audio bank',
  anim: 'Animation',
}

export const ext = (p: string): string => {
  const i = p.lastIndexOf('.')
  return i < 0 ? '' : p.slice(i + 1).toLowerCase()
}

export function typeName(p: string): string {
  return TYPES[ext(p)] ?? (ext(p) ? ext(p).toUpperCase() + ' file' : 'File')
}
```

Puis dans `src/lib/FilePreview.svelte` :
1. Supprimer les lignes 86-98 (`const ext = …` jusqu'à `const typeName = …` inclus).
2. Ajouter dans le bloc d'imports :

```ts
import { typeName } from './fileTypes'
```

(Le markup ligne 195 `{typeName(file.path)}` compile tel quel.)

- [ ] **Step 4: Vérifier le passage**

Run: `npx vitest run src/lib/fileTypes.test.ts && npm run check`
Expected: `Test Files  1 passed` et 0 erreur svelte-check.

- [ ] **Step 5: Commit**

```bash
git add src/lib/fileTypes.ts src/lib/fileTypes.test.ts src/lib/FilePreview.svelte
git commit -m "refactor(preview): extract shared fileTypes module"
```

### Task 7: Module pur `historySelection.ts` (TDD)

**Files:**
- Create: `src/lib/historySelection.ts`
- Test: `src/lib/historySelection.test.ts`

- [ ] **Step 1: Écrire les tests qui échouent**

Créer `src/lib/historySelection.test.ts` :

```ts
import { describe, it, expect } from 'vitest'
import { toggleFilePath, selectionAfterCommitChange, isLocalTip } from './historySelection'

describe('toggleFilePath', () => {
  it('opens a file, switches to another, closes on re-click', () => {
    expect(toggleFilePath(null, 'a.png')).toBe('a.png')
    expect(toggleFilePath('a.png', 'b.png')).toBe('b.png')
    expect(toggleFilePath('a.png', 'a.png')).toBeNull()
  })
})

describe('selectionAfterCommitChange', () => {
  it('keeps the selection on a same-commit refresh, resets on a commit change', () => {
    expect(selectionAfterCommitChange(true, 'a.png')).toBe('a.png')
    expect(selectionAfterCommitChange(false, 'a.png')).toBeNull()
    expect(selectionAfterCommitChange(false, null)).toBeNull()
  })
})

describe('isLocalTip', () => {
  it('is true only for the newest loaded commit', () => {
    const commits = [{ id: 'c2' }, { id: 'c1' }]
    expect(isLocalTip('c2', commits)).toBe(true)
    expect(isLocalTip('c1', commits)).toBe(false)
    expect(isLocalTip('c0', [])).toBe(false)
  })
})
```

- [ ] **Step 2: Vérifier l'échec**

Run: `npx vitest run src/lib/historySelection.test.ts`
Expected: FAIL — `Cannot find module './historySelection'`.

- [ ] **Step 3: Implémenter**

Créer `src/lib/historySelection.ts` :

```ts
/** Click/Enter on a commit-file row: the selected path toggles closed, any
 *  other path becomes the selection. Local to the History view — deliberately
 *  NOT a global store (spec: reset on commit change, gone on view leave). */
export function toggleFilePath(current: string | null, clicked: string): string | null {
  return current === clicked ? null : clicked
}

/** Selection surviving a detail refetch: only a same-commit refresh keeps it. */
export function selectionAfterCommitChange(sameCommit: boolean, current: string | null): string | null {
  return sameCommit ? current : null
}

/** True when `commitId` is the local tip (history is newest-first). Drives the
 *  « Preview of the current working copy » caveat: without `file cat <rev>`,
 *  any non-tip commit can only show the disk's current state. */
export function isLocalTip(commitId: string, commits: { id: string }[]): boolean {
  return commits[0]?.id === commitId
}
```

- [ ] **Step 4: Vérifier le passage**

Run: `npx vitest run src/lib/historySelection.test.ts`
Expected: `Test Files  1 passed`.

- [ ] **Step 5: Commit**

```bash
git add src/lib/historySelection.ts src/lib/historySelection.test.ts
git commit -m "feat(history): selection and local-tip helpers"
```

### Task 8: Extraire `FileHistorySection.svelte`

Refactor sans changement de comportement (couvert par le typecheck ici, puis la vérif navigateur de la Task 19 sur la vue Changes).

**Files:**
- Create: `src/lib/FileHistorySection.svelte`
- Modify: `src/lib/FilePreview.svelte` (supprimer l'état/effect/markup/styles de la timeline)

- [ ] **Step 1: Créer le composant (extraction à l'identique)**

Créer `src/lib/FileHistorySection.svelte` — l'effect vient de `FilePreview.svelte:59-77`, le markup des lignes 216-237, les styles des lignes 293-303 :

```svelte
<script lang="ts">
  import type { FileRevision } from './types'
  import { api } from './api'
  import { session } from './session.svelte'
  import { fmtSize } from './sizeFormat'

  // Per-asset revision timeline, fetched lazily on selection (anti-race).
  // `revisions` is bindable so a parent can read the head revision (the
  // History preview derives its Size row from it).
  let { path, revisions = $bindable([]) }: { path: string; revisions?: FileRevision[] } = $props()

  let loading = $state(false)
  let error = $state(false)
  let lastPath = ''

  $effect(() => {
    const p = path
    const repoPath = session.config.currentRepo
    if (!repoPath) { revisions = []; loading = false; error = false; lastPath = ''; return }
    const same = p === lastPath
    lastPath = p
    if (!same) { revisions = []; loading = true }
    error = false
    api.getFileHistory(repoPath, p)
      .then((revs) => { if (path === p) revisions = revs })
      .catch(() => { if (path === p) error = true })
      .finally(() => { if (path === p) loading = false })
  })

  const glyph: Record<string, { c: string; v: string }> = {
    add: { c: 'added', v: '+' }, modify: { c: 'modified', v: '~' }, delete: { c: 'deleted', v: '−' },
    move: { c: 'modified', v: 'R' }, copy: { c: 'modified', v: 'C' },
  }
  const authorLabel = (a: string) =>
    a === session.identity?.email ? 'you' : a.includes('@') ? a.split('@')[0] : a.slice(0, 8)
</script>

<div class="fhhead">History{#if revisions.length} · {revisions.length} {revisions.length === 1 ? 'revision' : 'revisions'}{/if}</div>
{#if loading}
  <p class="fhnote muted">Loading history…</p>
{:else if error}
  <p class="fhnote muted">Couldn't load file history.</p>
{:else if revisions.length === 0}
  <p class="fhnote muted">No committed revisions yet.</p>
{:else}
  <ul class="fhl">
    {#each revisions.slice(0, 30) as r (r.revision)}
      <li>
        <span class="tag {glyph[r.action]?.c}">{glyph[r.action]?.v ?? '?'}</span>
        <span class="frev">#{r.revisionNumber}</span>
        <span class="fmsg" title={r.message}>{r.message}</span>
        <span class="fwho">{authorLabel(r.author)}</span>
        <span class="fwhen" title={new Date(r.whenMs).toLocaleString()}>{r.when}</span>
        <span class="fsize">{fmtSize(r.size)}</span>
      </li>
    {/each}
  </ul>
  {#if revisions.length > 30}<p class="fhnote muted">…and {revisions.length - 30} more revisions</p>{/if}
{/if}

<style>
  .fhhead { font-size: 11px; color: var(--text-muted); text-transform: uppercase; letter-spacing: .04em; margin: 20px 0 6px; }
  .fhnote { font-size: 12px; margin: 4px 0; }
  .fhl { list-style: none; margin: 0; padding: 0; }
  .fhl li { display: flex; align-items: center; gap: 9px; padding: 6px 0; border-top: 1px solid var(--border); font-size: 12.5px; }
  .tag { width: 1.1em; text-align: center; font-weight: 500; flex-shrink: 0; }
  .tag.added { color: var(--added); } .tag.modified { color: var(--modified); } .tag.deleted { color: var(--deleted); }
  .frev { font-family: var(--font-mono); font-size: 11px; color: var(--text-muted); flex: none; min-width: 28px; }
  .fmsg { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .fwho { flex: none; font-size: 11px; color: var(--accent-text); }
  .fwhen { flex: none; font-size: 11px; color: var(--text-dim); }
  .fsize { flex: none; font-size: 11px; color: var(--text-muted); font-family: var(--font-mono); min-width: 58px; text-align: right; }
</style>
```

- [ ] **Step 2: Recomposer FilePreview dessus**

Dans `src/lib/FilePreview.svelte` :

1. Supprimer le bloc d'état + effect de la timeline (lignes 59-77 : `let fileHistory = …` jusqu'à la fin de l'effect `getFileHistory`).
2. Supprimer `glyph` (lignes 79-82) et `authorLabel` (lignes 83-84) — ils ne servaient qu'à la timeline.
3. Retirer `FileRevision` de l'import de types ligne 2 :

```ts
import type { ChangedFile, DiffLine, PreviewData } from './types'
```

4. Ajouter l'import :

```ts
import FileHistorySection from './FileHistorySection.svelte'
```

5. Remplacer tout le bloc markup de la timeline (lignes 216-237, du `<div class="fhhead">…` au `{/if}` du `…and N more revisions`) par :

```svelte
      <FileHistorySection path={file.path} />
```

6. Supprimer les styles devenus morts : `.tag` (3 lignes), `.fhhead`, `.fhnote`, `.fhl`, `.fhl li`, `.frev`, `.fmsg`, `.fwho`, `.fwhen`, `.fsize`.

- [ ] **Step 3: Typecheck**

Run: `npm run check`
Expected: 0 erreur, 0 warning (un warning « unused CSS selector » signale un style oublié à l'étape 6).

- [ ] **Step 4: Commit**

```bash
git add src/lib/FileHistorySection.svelte src/lib/FilePreview.svelte
git commit -m "refactor(preview): extract FileHistorySection component"
```

### Task 9: Extraire `MediaPreview.svelte`

Refactor sans changement de comportement en mode `compare` (défaut, utilisé par FilePreview) ; le mode simple (`compare={false}`) est le rendu History.

**Files:**
- Create: `src/lib/MediaPreview.svelte`
- Modify: `src/lib/FilePreview.svelte` (supprimer l'effect preview + le bloc média + ses styles)

- [ ] **Step 1: Créer le composant**

Créer `src/lib/MediaPreview.svelte` — l'effect vient de `FilePreview.svelte:47-57`, le markup des lignes 143-174, les styles des lignes 259-271 :

```svelte
<script lang="ts">
  import type { PreviewData } from './types'
  import { api } from './api'
  import { session } from './session.svelte'
  import { isPreviewableImage } from './previewKind'
  import Icon from './Icon.svelte'
  import AudioPlayer from './AudioPlayer.svelte'
  import ModelViewer from './ModelViewer.svelte'

  // Working-copy media preview (image thumbnail / audio / 3D) of one repo
  // file. Shared by FilePreview (Changes, compare mode) and HistoryFilePreview
  // (single-box mode). `preview` is bindable so parents can read dimensions.
  let { path, action, compare = true, preview = $bindable(null) }: {
    path: string
    action: 'add' | 'modify' | 'delete' | 'move' | 'copy'
    /** true = before/after compare boxes with the Changes captions and notes;
     *  false = a single working-copy box (the History panel has its own caveat). */
    compare?: boolean
    preview?: PreviewData | null
  } = $props()

  let lastPath = ''

  // Same anti-race pattern as the other lazy fetches: check the selection
  // still matches on arrival. Deleted files have no working copy to preview.
  $effect(() => {
    const p = path
    const repoPath = session.config.currentRepo
    if (action === 'delete' || !repoPath) { preview = null; lastPath = ''; return }
    const same = p === lastPath
    lastPath = p
    if (!same) preview = null
    api.getPreview(repoPath, p)
      .then((r) => { if (path === p) preview = r })
      .catch(() => { if (path === p) preview = null })
  })

  const baseName = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? p : p.slice(i + 1) }
</script>

{#if preview?.kind === 'audio' && preview.url}
  <AudioPlayer src={preview.url} name={baseName(path)} />
  {#if compare}<p class="note muted"><Icon name="info" size={14} /> Audio asset — plays the working copy.</p>{/if}
{:else if preview?.kind === 'model' && preview.url}
  <ModelViewer url={preview.url} name={baseName(path)} />
  {#if compare}<p class="note muted"><Icon name="info" size={14} /> 3D preview of the working copy — drag to orbit, scroll to zoom.</p>{/if}
{:else if compare}
  <div class="cmp">
    {#if action !== 'add'}
      <figure class="cbox">
        <div class="thumb before"><Icon name="image" size={26} /></div>
        <figcaption>Before · previous revision</figcaption>
      </figure>
    {/if}
    {#if action !== 'delete'}
      <figure class="cbox">
        {#if preview?.kind === 'image' && preview.url}
          <div class="thumb after img"><img src={preview.url} alt={baseName(path)} /></div>
        {:else}
          <div class="thumb after"><Icon name="image" size={26} /></div>
        {/if}
        <figcaption class="aft">{action === 'add' ? 'New file' : 'After · working copy'}</figcaption>
      </figure>
    {/if}
  </div>
  {#if preview?.kind === 'image'}
    <p class="note muted"><Icon name="info" size={14} /> Previous-revision preview needs server support — working copy only.</p>
  {:else}
    <p class="note muted"><Icon name="info" size={14} /> Binary asset — visual compare, no text diff.</p>
  {/if}
{:else}
  {#if preview?.kind === 'image' && preview.url}
    <div class="thumb single img"><img src={preview.url} alt={baseName(path)} /></div>
  {:else}
    <div class="thumb single"><Icon name={isPreviewableImage(path) ? 'image' : 'file'} size={26} /></div>
  {/if}
{/if}

<style>
  .cmp { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 12px; }
  figure { margin: 0; }
  .thumb { height: 150px; border-radius: 8px; display: grid; place-items: center; color: var(--text-dim); border: 1px solid var(--border); }
  .thumb.before { background: #2b2f35; }
  .thumb.after { background: #33475f; }
  .thumb.single { height: 190px; background: #2b2f35; }
  /* The box is a grid with auto rows, where a percentage height on the img
     resolves as auto (335×335 in a 335×149 box, then clipped). Absolute
     positioning sizes the img against the box itself instead. */
  .thumb.img { padding: 0; overflow: hidden; position: relative; background: repeating-conic-gradient(#2b2f35 0% 25%, #333a44 0% 50%) 50% / 24px 24px; }
  .thumb.img img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: contain; }
  figcaption { font-size: 11px; color: var(--text-muted); margin-top: 7px; text-align: center; }
  figcaption.aft { color: var(--accent-text); }
  .note { display: flex; align-items: center; gap: 7px; font-size: 11px; margin: 12px 0 4px; }
</style>
```

- [ ] **Step 2: Recomposer FilePreview dessus**

Dans `src/lib/FilePreview.svelte` :

1. Supprimer l'effect preview et `lastPreviewPath` (lignes 45-57 originales), en GARDANT la déclaration `let preview = $state<PreviewData | null>(null)` (la ligne Dimensions du `<dl>` la lit toujours).
2. Supprimer les imports devenus inutiles `AudioPlayer` et `ModelViewer` ; ajouter :

```ts
import MediaPreview from './MediaPreview.svelte'
```

3. Remplacer TOUT le contenu de la branche `{#if file.isBinary}` du markup (du `{#if preview?.kind === 'audio' && preview.url}` jusqu'au `{/if}` interne qui précède `{:else if diffLoading}` — les lignes `{#if file.isBinary}` et `{:else if diffLoading}` elles-mêmes ne bougent pas) par la seule ligne :

```svelte
        <MediaPreview path={file.path} action={file.action} bind:preview />
```

Résultat attendu :

```svelte
      {#if file.isBinary}
        <MediaPreview path={file.path} action={file.action} bind:preview />
      {:else if diffLoading}
```

4. Supprimer les styles devenus morts : `.cmp`, `figure`, `.thumb`, `.thumb.before`, `.thumb.after`, `.thumb.img`, `.thumb.img img`, `figcaption`, `figcaption.aft`, `.note` (et le commentaire CSS du positionnement absolu).

- [ ] **Step 3: Typecheck**

Run: `npm run check`
Expected: 0 erreur, 0 warning.

- [ ] **Step 4: Commit**

```bash
git add src/lib/MediaPreview.svelte src/lib/FilePreview.svelte
git commit -m "refactor(preview): extract MediaPreview component"
```

### Task 10: Composant `HistoryFilePreview.svelte`

**Files:**
- Create: `src/lib/HistoryFilePreview.svelte`

- [ ] **Step 1: Créer le composant**

Créer `src/lib/HistoryFilePreview.svelte` :

```svelte
<script lang="ts">
  import type { CommitFile, FileRevision, PreviewData } from './types'
  import Icon from './Icon.svelte'
  import MediaPreview from './MediaPreview.svelte'
  import FileHistorySection from './FileHistorySection.svelte'
  import { typeName } from './fileTypes'
  import { fmtSize } from './sizeFormat'

  // Lightweight preview panel for a file of a History commit: working-copy
  // media + type/size + the file's revision timeline. Deliberately NO Discard
  // and NO Lock — those act on the working copy, out of place next to an
  // arbitrary commit.
  let { file, isTip, onclose }: {
    file: CommitFile
    /** The selected commit is the local tip — the disk matches it, no caveat needed. */
    isTip: boolean
    onclose: () => void
  } = $props()

  const baseName = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? p : p.slice(i + 1) }
  const dirName = (p: string) => { const i = p.lastIndexOf('/'); return i < 0 ? '' : p.slice(0, i + 1) }

  const badge = $derived(
    file.action === 'add' ? { t: 'Added', c: 'added' } :
    file.action === 'delete' ? { t: 'Deleted', c: 'deleted' } :
    { t: 'Modified', c: 'modified' },
  )

  let preview = $state<PreviewData | null>(null)
  let revisions = $state<FileRevision[]>([])
  // Best available size without `file cat <rev>`: the newest committed revision.
  const sizeText = $derived(revisions[0] ? fmtSize(revisions[0].size) : '—')
</script>

<div class="hpreview">
  <div class="body">
    <header class="head">
      <div class="ic"><Icon name="file" size={20} /></div>
      <div class="ttl">
        <div class="fn">{baseName(file.path)}</div>
        <div class="fp muted">{dirName(file.path)}</div>
      </div>
      <span class="badge {badge.c}">{badge.t}</span>
      <button class="close" onclick={onclose} title="Close preview (Esc)" aria-label="Close preview">×</button>
    </header>

    {#if file.action === 'delete'}
      <div class="gone">
        <Icon name="file" size={26} />
        <p>No longer in the working copy</p>
      </div>
    {:else}
      {#if !isTip}
        <p class="wcnote" role="note"><Icon name="info" size={14} /> Preview of the current working copy — this commit's version can't be shown yet.</p>
      {/if}
      <MediaPreview path={file.path} action={file.action} compare={false} bind:preview />
    {/if}

    <dl class="meta">
      <div><dt>Type</dt><dd>{typeName(file.path)}</dd></div>
      <div><dt>Size</dt><dd>{sizeText}</dd></div>
      {#if preview?.width && preview?.height}
        <div><dt>Dimensions</dt><dd>{preview.width} × {preview.height}</dd></div>
      {/if}
    </dl>

    <FileHistorySection path={file.path} bind:revisions />
  </div>
</div>

<style>
  .hpreview { flex: 1; overflow: auto; min-width: 0; border-left: 1px solid var(--border); }
  .body { padding: 16px 18px; max-width: 720px; }
  .head { display: flex; align-items: center; gap: 11px; margin-bottom: 16px; }
  .ic { width: 34px; height: 34px; border-radius: 8px; background: var(--panel); display: grid; place-items: center; color: var(--text-muted); flex-shrink: 0; }
  .ttl { min-width: 0; flex: 1; }
  .fn { font-size: 14px; font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .fp { font-size: 11px; }
  .badge { border-radius: var(--radius); padding: 3px 9px; font-size: 11px; flex-shrink: 0; }
  .badge.modified { background: var(--warn-bg); color: var(--warn-text); }
  .badge.added { background: rgba(63, 185, 80, .15); color: var(--added); }
  .badge.deleted { background: rgba(248, 81, 73, .15); color: var(--deleted); }
  .close { width: 24px; height: 24px; padding: 0; line-height: 1; font-size: 15px; color: var(--text-muted); flex-shrink: 0; }
  .wcnote { display: flex; align-items: center; gap: 7px; font-size: 11px; color: var(--text-muted); margin: 0 0 10px; }
  .gone { display: flex; align-items: center; gap: 12px; padding: 22px; border: 1px dashed var(--border); border-radius: 8px; color: var(--text-muted); font-size: 12.5px; }
  .gone p { margin: 0; }
  .meta { margin: 18px 0 0; }
  .meta > div { display: flex; justify-content: space-between; align-items: center; gap: 12px; padding: 9px 0; border-top: 1px solid var(--border); font-size: 12.5px; }
  dt { color: var(--text-muted); }
  dd { margin: 0; display: inline-flex; align-items: center; gap: 10px; }
</style>
```

- [ ] **Step 2: Typecheck**

Run: `npm run check`
Expected: 0 erreur (le composant n'est pas encore monté — un warning « unused » n'apparaît pas pour un fichier entier).

- [ ] **Step 3: Commit**

```bash
git add src/lib/HistoryFilePreview.svelte
git commit -m "feat(history): HistoryFilePreview panel component"
```

### Task 11: Câbler la sélection dans History.svelte

**Files:**
- Modify: `src/lib/History.svelte` (imports, état local, effect détail, markup des lignes de fichiers, panneau, styles)

- [ ] **Step 1: Imports + état local**

Dans `src/lib/History.svelte`, ajouter aux imports :

```ts
import HistoryFilePreview from './HistoryFilePreview.svelte'
import { toggleFilePath, selectionAfterCommitChange, isLocalTip } from './historySelection'
```

Après `let lastDetailId = ''` (ligne 29), ajouter :

```ts
  // Selected commit-file path (opens the preview panel). Local state, NOT a
  // global store: it resets on commit change and evaporates on view leave.
  let previewPath = $state<string | null>(null)
  const previewFile = $derived(detailFiles.find((f) => f.path === previewPath) ?? null)
```

- [ ] **Step 2: Reset au changement de commit + Escape**

1. Dans l'effect de détail (lignes 31-45), remplacer les deux premières branches :

```ts
    if (!c || !repoPath) { detailFiles = []; detailLoading = false; detailError = false; lastDetailId = ''; return }
    const sameId = c.id === lastDetailId
    lastDetailId = c.id
    if (!sameId) { detailLoading = true; detailFiles = []; editing = false }
```

par :

```ts
    if (!c || !repoPath) { detailFiles = []; detailLoading = false; detailError = false; lastDetailId = ''; previewPath = null; return }
    const sameId = c.id === lastDetailId
    lastDetailId = c.id
    previewPath = selectionAfterCommitChange(sameId, previewPath)
    if (!sameId) { detailLoading = true; detailFiles = []; editing = false }
```

2. Après cet effect, ajouter la fermeture par Escape (les Escape tapés dans un input — édition du message de commit — gardent leur sens actuel) :

```ts
  // Escape closes the preview panel — unless focus is in a text input (the
  // commit-message editor already binds Escape to cancel).
  $effect(() => {
    if (previewPath === null) return
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape' && !(e.target instanceof HTMLInputElement)) previewPath = null
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  })
```

- [ ] **Step 3: Lignes de fichiers sélectionnables**

Remplacer le `<li>` de la liste de fichiers (ligne 279) :

```svelte
            <li oncontextmenu={(e) => { e.preventDefault(); ctxMenu = { x: e.clientX, y: e.clientY, path: f.path } }}><span class="tag {glyph[f.action]?.c}">{glyph[f.action]?.v ?? '?'}</span>{#if listThumbs.get(f.path)}<img class="rowthumb" src={listThumbs.get(f.path)} alt="" />{/if}<span class="path"><span class="fdir">{dir(f.path)}</span>{base(f.path)}</span></li>
```

par :

```svelte
            <li class:sel={f.path === previewPath} role="button" tabindex="0"
                onclick={() => (previewPath = toggleFilePath(previewPath, f.path))}
                onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); previewPath = toggleFilePath(previewPath, f.path) } }}
                oncontextmenu={(e) => { e.preventDefault(); ctxMenu = { x: e.clientX, y: e.clientY, path: f.path } }}>
              <span class="tag {glyph[f.action]?.c}">{glyph[f.action]?.v ?? '?'}</span>{#if listThumbs.get(f.path)}<img class="rowthumb" src={listThumbs.get(f.path)} alt="" />{/if}<span class="path"><span class="fdir">{dir(f.path)}</span>{base(f.path)}</span>
            </li>
```

- [ ] **Step 4: Monter le panneau**

Juste après le `</div>` fermant `.detail` (ligne 286), avant le bloc `{#if ctxMenu}`, ajouter :

```svelte
  {#if previewFile && selected}
    <HistoryFilePreview file={previewFile} isTip={isLocalTip(selected.id, commits)}
                        onclose={() => (previewPath = null)} />
  {/if}
```

- [ ] **Step 5: Styles des lignes**

Dans le `<style>` de History.svelte, remplacer :

```css
  .fl li { display: flex; align-items: center; gap: 8px; padding: 5px 0; font-size: 12.5px; }
```

par :

```css
  .fl li { display: flex; align-items: center; gap: 8px; padding: 5px 6px; margin: 0 -6px; border-radius: 6px; font-size: 12.5px; cursor: pointer; }
  .fl li:hover { background: var(--panel); }
  .fl li.sel { background: var(--accent-soft); }
```

- [ ] **Step 6: Typecheck + suite**

Run: `npm run check && npx vitest run`
Expected: 0 erreur, toutes suites vertes.

- [ ] **Step 7: Vérification navigateur rapide (mock)**

Lancer `npm run dev`, ouvrir http://localhost:5173, se connecter (mock), ouvrir un repo, aller dans History :
- clic sur une ligne de fichier d'un commit → panneau à droite (vignette mock pour un `.uasset`/`.png`, icône fichier pour un `.cpp`, timeline « History · 3 revisions ») ;
- re-clic sur la même ligne → panneau fermé ; Enter au clavier → rouvert ; Escape → fermé ;
- commit ancien (pas le premier) → mention « Preview of the current working copy… » visible ; premier commit → pas de mention ;
- ligne `delete` → « No longer in the working copy » + icône générique ;
- changer de commit avec un panneau ouvert → panneau fermé (reset).

- [ ] **Step 8: Commit**

```bash
git add src/lib/History.svelte
git commit -m "feat(history): selectable commit files open a preview panel"
```

---

## Item 2 — Unification des previews dans la vue Merge

### Task 12: VÉRIFICATION RÉELLE du sidecar `~theirs` (OBLIGATOIRE, en tête d'item)

Scénario offline-safe (100 % local, fonctionne serveur down) calqué sur la Task 9 du plan P1 (`docs/superpowers/plans/2026-07-10-lore-desktop-p1-artist-lot.md`). Objectif : confirmer l'existence et le **nommage exact** du sidecar « theirs » pour un binaire ET un texte pendant un merge conflictuel, et voir ce que `lore diff` renvoie pour un texte en conflit (décide le contenu du mini-diff de la Task 17).

**Files:**
- Modify: `src-tauri/tests/fixtures/README.md` (documenter le constat)

- [ ] **Step 1: Fabriquer un merge conflictuel binaire + texte**

Dans PowerShell (les trois écritures PNG produisent des contenus binaires distincts mais tous décodables — les octets après `IEND` sont ignorés par les décodeurs) :

```powershell
$lore = "C:\Users\jimmy\bin\lore.exe"
$repo = "C:\Users\jimmy\lore-test-repo"
# Base commune : un petit PNG committé + une ligne texte.
$png = [Convert]::FromBase64String('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==')
[IO.File]::WriteAllBytes("$repo\p2-theirs-test.png", $png)
Add-Content "$repo\README.md" "p2 base line"
& $lore stage . --scan --repository $repo
& $lore commit "p2 theirs test: base" --repository $repo
# Côté branche : modifier les DEUX fichiers.
& $lore branch create p2-theirs-src --repository $repo
[IO.File]::WriteAllBytes("$repo\p2-theirs-test.png", [byte[]]($png + 0x42))
Add-Content "$repo\README.md" "line from p2-theirs-src"
& $lore stage . --scan --repository $repo
& $lore commit "p2 theirs test: branch side" --repository $repo
# Côté main : modifier les DEUX fichiers différemment.
& $lore branch switch main --repository $repo
[IO.File]::WriteAllBytes("$repo\p2-theirs-test.png", [byte[]]($png + 0x99 + 0x99))
Add-Content "$repo\README.md" "line from main"
& $lore stage . --scan --repository $repo
& $lore commit "p2 theirs test: main side" --repository $repo
# Merge conflictuel.
& $lore branch merge start p2-theirs-src --repository $repo
```

Expected: le `merge start` signale des conflits (sur `p2-theirs-test.png` ET `README.md`).

- [ ] **Step 2: Lister le working copy pendant le merge et noter le nommage exact**

```powershell
Get-ChildItem $repo -Force | Select-Object Name
& $lore status --scan --repository $repo --json | Select-String -Pattern 'flagConflict'
```

Expected (hypothèse P1 à CONFIRMER) : le listing contient `p2-theirs-test.png~theirs` et `README.md~theirs` à côté des originaux ; le status JSON marque les deux fichiers `flagConflict`. **Noter le nommage exact observé** (suffixe, casse, binaire vs texte). Trois issues possibles :
- `<nom>~theirs` pour les deux → les Tasks 13-17 s'exécutent telles quelles ;
- autre suffixe/nommage → mettre à jour la constante `THEIRS_SUFFIX` dans les Tasks 13 (previewKind.ts), 14 (preview.rs), 15 (mock) et les libellés de la Task 17 ;
- sidecar absent pour certains types → la carte Theirs garde l'icône actuelle pour ces types (le fallback de la Task 17 le fait déjà : pas de vignette dans `listThumbs` → icône) ; documenter quels types en ont.

- [ ] **Step 3: Diff texte pendant le merge (décide le mini-diff)**

```powershell
& $lore diff "$repo\README.md" --repository $repo --json
```

Expected: un événement `fileDiff` avec un champ `patch` non vide (c'est l'appel exact de `lore_diff`, `src-tauri/src/commands.rs:2205-2217`). **Noter ce que montre le patch** (la version mine vs base ? des marqueurs de conflit ?). Si le patch est vide ou en erreur pour un fichier en conflit, le mini-diff de la Task 17 s'affichera simplement vide (fallback déjà prévu : cartes sans vignette) — le noter pour adapter/simplifier la Task 17.

- [ ] **Step 4: Abort + cleanup complet**

```powershell
& $lore branch merge abort --repository $repo
Get-ChildItem $repo -Force | Select-Object Name        # les sidecars doivent avoir disparu
& $lore branch archive p2-theirs-src --repository $repo
& $lore status --scan --repository $repo               # attendu : aucun changement pendant
```

Expected: plus aucun fichier `*~theirs`, branche archivée, status propre. (Les commits de test restent dans l'historique du repo de test — même pratique que le P1.)

- [ ] **Step 5: Documenter le constat**

Ajouter à la fin de `src-tauri/tests/fixtures/README.md` (adapter au constat réel) :

```markdown

**Sidecar « theirs » d'un merge conflictuel** (vérifié le <date> sur un merge réel,
binaire + texte) : pendant `branch merge start`, le CLI matérialise la version
entrante de chaque fichier en conflit sous `<nom>~theirs` à côté de l'original
(ex. `p2-theirs-test.png~theirs`, `README.md~theirs`). `branch merge abort`
supprime les sidecars. `lore diff <abs>` pendant le merge : <ce qui a été
observé au Step 3>.
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/tests/fixtures/README.md
git commit -m "docs(fixtures): pin ~theirs sidecar naming from a real conflicted merge"
```

### Task 13: Helpers `~theirs` côté front (TDD)

**Files:**
- Modify: `src/lib/previewKind.ts`
- Test: `src/lib/previewKind.test.ts`

- [ ] **Step 1: Écrire les tests qui échouent**

Ajouter à la fin de `src/lib/previewKind.test.ts` (compléter l'import existant avec `theirsSidecar, stripTheirsSuffix`) :

```ts
describe('theirs sidecar helpers', () => {
  it('builds and strips the sidecar path', () => {
    expect(theirsSidecar('Content/T_Rock.png')).toBe('Content/T_Rock.png~theirs')
    expect(stripTheirsSuffix('Content/T_Rock.png~theirs')).toBe('Content/T_Rock.png')
    expect(stripTheirsSuffix('Content/T_Rock.png')).toBe('Content/T_Rock.png')
  })
  it('classifies a sidecar like its base file', () => {
    expect(isPreviewableImage('Content/T_Rock.png~theirs')).toBe(true)
    expect(isPreviewableImage('Source/main.cpp~theirs')).toBe(false)
  })
})
```

(Si `previewKind.test.ts` n'a pas d'import `describe/it/expect`, il en a déjà — le fichier existe.)

- [ ] **Step 2: Vérifier l'échec**

Run: `npx vitest run src/lib/previewKind.test.ts`
Expected: FAIL — `theirsSidecar` n'existe pas.

- [ ] **Step 3: Implémenter**

Remplacer le contenu de `src/lib/previewKind.ts` par :

```ts
/** Shared classifier: files whose rows/panels can show an image thumbnail. */
const IMAGE_RE = /\.(png|jpe?g|webp|bmp|gif|tga|tiff?|dds|exr|hdr|psd|blend|uasset|umap|sbsar|spp)$/i

/** Exact sidecar naming the CLI materializes for the incoming version of a
 *  conflicted file during a merge — pinned by the P2 item-2 real-merge
 *  verification (plan Task 12). Mirrored by THEIRS_SUFFIX in preview.rs. */
const THEIRS_SUFFIX = '~theirs'

export function theirsSidecar(path: string): string {
  return path + THEIRS_SUFFIX
}

export function stripTheirsSuffix(path: string): string {
  return path.endsWith(THEIRS_SUFFIX) ? path.slice(0, -THEIRS_SUFFIX.length) : path
}

export function isPreviewableImage(path: string): boolean {
  return IMAGE_RE.test(stripTheirsSuffix(path))
}
```

- [ ] **Step 4: Vérifier le passage**

Run: `npx vitest run src/lib/previewKind.test.ts && npx vitest run`
Expected: `1 passed` puis toute la suite verte (aucun test existant ne casse — `isPreviewableImage` est inchangé pour les chemins sans suffixe).

- [ ] **Step 5: Commit**

```bash
git add src/lib/previewKind.ts src/lib/previewKind.test.ts
git commit -m "feat(preview): classify ~theirs sidecars as their base type (frontend)"
```

### Task 14: Servir les sidecars `~theirs` côté Rust (TDD)

**Files:**
- Modify: `src-tauri/src/preview.rs` (helper `preview_ext` + `lore_preview` + tests)

- [ ] **Step 1: Écrire les tests qui échouent**

Dans le module `tests` de `src-tauri/src/preview.rs`, ajouter :

```rust
    #[test]
    fn theirs_sidecar_uses_base_extension() {
        assert_eq!(preview_ext("Content/T_Rock.png~theirs"), "png");
        assert_eq!(preview_ext("Content/T_Rock.png"), "png");
        assert_eq!(preview_ext("noext~theirs"), "");
    }

    #[test]
    fn theirs_sidecar_image_decodes() {
        let d = dir("theirs");
        // A real PNG saved under the sidecar name — exactly what a conflicted
        // merge leaves on disk next to the original.
        let p = d.join("tex.png~theirs");
        image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(8, 8, image::Rgba([1, 2, 3, 255])))
            .save_with_format(&p, image::ImageFormat::Png)
            .unwrap();
        let out = image_preview(&p, &preview_ext("tex.png~theirs"), 64, None);
        assert_eq!(out.kind, "image");
    }
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test --manifest-path src-tauri/Cargo.toml preview`
Expected: FAIL à la compilation — `cannot find function preview_ext`.

- [ ] **Step 3: Implémenter**

Dans `src-tauri/src/preview.rs`, après les constantes `MODEL_EXTS` (~ligne 23), ajouter :

```rust
/// A conflicted merge materializes the incoming version of each conflicted
/// file as `<name>~theirs` next to the original (naming pinned by the P2
/// item-2 real-merge verification). Classify those sidecars as their base
/// type; the file on disk is read as-is. Mirrored in previewKind.ts.
const THEIRS_SUFFIX: &str = "~theirs";

pub(crate) fn preview_ext(path: &str) -> String {
    ext_of(path.strip_suffix(THEIRS_SUFFIX).unwrap_or(path))
}
```

Puis dans `lore_preview` (~ligne 577), remplacer :

```rust
        let ext = ext_of(&path);
```

par :

```rust
        let ext = preview_ext(&path);
```

- [ ] **Step 4: Vérifier le passage**

Run: `cargo test --manifest-path src-tauri/Cargo.toml preview`
Expected: `test result: ok` (les nouveaux tests + tous les tests preview existants).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/preview.rs
git commit -m "feat(preview): serve ~theirs sidecars as their base type (rust)"
```

### Task 15: Mock — conflit texte + previews sidecar (TDD)

**Files:**
- Modify: `src/lib/mock.ts` (mergeStart ~ligne 355, getPreview ~ligne 199)
- Test: `src/lib/mock.test.ts`

- [ ] **Step 1: Écrire les tests qui échouent**

Ajouter dans `src/lib/mock.test.ts`, à la fin du describe `'mock api'` :

```ts
  it('mergeStart raises binary AND text conflicts', async () => {
    await mock.mergeStart('C:/repos/mixed', 'feature/loot')
    const conflicts = await mock.mergeConflicts('C:/repos/mixed')
    expect(conflicts.some((c) => c.isBinary)).toBe(true)
    expect(conflicts.some((c) => !c.isBinary)).toBe(true)
    await mock.mergeAbort('C:/repos/mixed')
  })

  it('getPreview serves a ~theirs sidecar as its base type', async () => {
    const p = await mock.getPreview('C:/repos/game', 'Content/UI/T_Icon_Sword.png~theirs')
    expect(p.kind).toBe('image')
  })
```

Run: `npx vitest run src/lib/mock.test.ts`
Expected: FAIL — les deux nouveaux tests (pas de conflit texte, kind 'none' pour le sidecar).

- [ ] **Step 2: Implémenter**

Dans `src/lib/mock.ts` :

1. Ajouter à l'import de `previewKind` (ligne 1) :

```ts
import { isPreviewableImage, stripTheirsSuffix } from './previewKind'
```

2. Dans `mergeStart`, remplacer l'affectation de `mergeConflictState` par :

```ts
    mergeConflictState = [
      { path: 'Content/Environment/T_Cliff_D.uasset', isBinary: true, unresolved: true },
      { path: 'Content/Maps/Arena.umap', isBinary: true, unresolved: true },
      { path: 'Source/Player/PlayerCharacter.cpp', isBinary: false, unresolved: true },
    ]
```

3. Au début de `getPreview` (juste après `await delay(200)`) :

```ts
    // A merge's ~theirs sidecar previews like its base file (dev parity with
    // preview_ext in preview.rs).
    path = stripTheirsSuffix(path)
```

- [ ] **Step 3: Vérifier le passage**

Run: `npx vitest run src/lib/mock.test.ts`
Expected: `Test Files  1 passed` (les tests existants du flow merge restent verts — ils ne comptent pas les conflits à l'unité).

- [ ] **Step 4: Commit**

```bash
git add src/lib/mock.ts src/lib/mock.test.ts
git commit -m "feat(mock): text merge conflict and ~theirs sidecar previews"
```

### Task 16: Extraire `DiffBlock.svelte`

Refactor sans changement de comportement (FilePreview), en préparation du mini-diff des cartes Merge.

**Files:**
- Create: `src/lib/DiffBlock.svelte`
- Modify: `src/lib/FilePreview.svelte` (remplacer le markup du diff + supprimer ses styles)

- [ ] **Step 1: Créer le composant (extraction à l'identique + cap optionnel)**

Créer `src/lib/DiffBlock.svelte` — markup depuis `FilePreview.svelte` (bloc `.diff`), styles idem :

```svelte
<script lang="ts">
  import type { DiffLine } from './types'

  // Unified-diff line rendering, shared by FilePreview (full diff) and the
  // Merge conflict cards (mini-diff capped by `maxLines`; 0 = no cap).
  let { lines, maxLines = 0 }: { lines: DiffLine[]; maxLines?: number } = $props()

  const shown = $derived(maxLines > 0 ? lines.slice(0, maxLines) : lines)
</script>

<div class="diff">
  {#each shown as line, i (i)}
    <div class="dl {line.kind}">
      <span class="ln">{line.oldLine ?? ''}</span>
      <span class="ln">{line.newLine ?? ''}</span>
      <span class="mk">{line.kind === 'add' ? '+' : line.kind === 'del' ? '-' : ''}</span>
      <span class="tx">{line.text}</span>
    </div>
  {/each}
  {#if maxLines > 0 && lines.length > maxLines}
    <div class="more">… {lines.length - maxLines} more lines</div>
  {/if}
</div>

<style>
  .diff { font-family: var(--font-mono); font-size: 12px; line-height: 1.55; border: 1px solid var(--border); border-radius: 8px; overflow-x: auto; margin: 4px 0; }
  .dl { display: flex; }
  .ln { flex: 0 0 44px; text-align: right; padding: 0 8px; color: var(--text-dim); user-select: none; }
  .mk { flex: 0 0 16px; text-align: center; color: var(--text-dim); user-select: none; }
  .tx { flex: 1; white-space: pre; padding-right: 12px; }
  .dl.add { background: rgba(63, 185, 80, .12); }
  .dl.add .mk, .dl.add .tx { color: var(--added); }
  .dl.del { background: rgba(248, 81, 73, .12); }
  .dl.del .mk, .dl.del .tx { color: var(--deleted); }
  .dl.context .tx { color: var(--text-muted); }
  .dl.hunk { background: var(--panel); }
  .dl.hunk .tx { color: var(--accent-text); }
  .more { padding: 3px 12px; font-size: 11px; color: var(--text-dim); border-top: 1px solid var(--border); }
</style>
```

- [ ] **Step 2: Recomposer FilePreview dessus**

Dans `src/lib/FilePreview.svelte` :

1. Ajouter l'import :

```ts
import DiffBlock from './DiffBlock.svelte'
```

2. Remplacer le bloc markup du diff :

```svelte
        <div class="diff">
          {#each diff as line, i (i)}
            <div class="dl {line.kind}">
              <span class="ln">{line.oldLine ?? ''}</span>
              <span class="ln">{line.newLine ?? ''}</span>
              <span class="mk">{line.kind === 'add' ? '+' : line.kind === 'del' ? '-' : ''}</span>
              <span class="tx">{line.text}</span>
            </div>
          {/each}
        </div>
```

par :

```svelte
        <DiffBlock lines={diff} />
```

3. Supprimer les styles devenus morts : `.diff`, `.dl`, `.ln`, `.mk`, `.tx`, `.dl.add` (2 règles), `.dl.del` (2 règles), `.dl.context .tx`, `.dl.hunk` (2 règles).

- [ ] **Step 3: Typecheck**

Run: `npm run check`
Expected: 0 erreur, 0 warning.

- [ ] **Step 4: Commit**

```bash
git add src/lib/DiffBlock.svelte src/lib/FilePreview.svelte
git commit -m "refactor(preview): extract DiffBlock component"
```

### Task 17: Cartes Merge — vignettes réelles + mini-diff texte

**Files:**
- Modify: `src/lib/Merge.svelte` (imports, effects de fetch, markup des cartes, styles)

- [ ] **Step 1: Imports + fetchs**

Dans `src/lib/Merge.svelte` :

1. Ajouter aux imports :

```ts
import { listThumbs, requestThumb } from './thumbs.svelte'
import { theirsSidecar } from './previewKind'
import DiffBlock from './DiffBlock.svelte'
```

et compléter la ligne de types existante (ligne 6) :

```ts
import type { Branch, MergePreview, MergeConflict, DiffLine } from './types'
```

2. Après la déclaration de `sawMerge`/l'effect d'abort externe (Task 5), ajouter :

```ts
  // Queue real working-copy thumbnails for every binary conflict: Mine = the
  // file itself, Theirs = the `<name>~theirs` sidecar the CLI materializes
  // during a conflicted merge (verified on a real merge — see the fixtures
  // README). A path with no thumbnail stays null in listThumbs → icon fallback.
  $effect(() => {
    if (phase !== 'resolving') return
    for (const c of conflicts) {
      if (!c.isBinary) continue
      requestThumb(c.path)
      requestThumb(theirsSidecar(c.path))
    }
  })

  // Mini-diff for the selected TEXT conflict (same anti-race pattern as
  // FilePreview). What `lore diff` shows during a merge was pinned by the
  // plan's verification task; an empty/failed diff just renders nothing.
  let miniDiff = $state<DiffLine[]>([])
  let miniLoading = $state(false)
  let lastMiniPath = ''

  $effect(() => {
    const c = selected
    const p = session.config.currentRepo
    if (!c || c.isBinary || !p || phase !== 'resolving') { miniDiff = []; miniLoading = false; lastMiniPath = ''; return }
    const same = c.path === lastMiniPath
    lastMiniPath = c.path
    if (!same) { miniDiff = []; miniLoading = true }
    api.getDiff(p, c.path)
      .then((d) => { if (selectedPath === c.path) miniDiff = d })
      .catch(() => { if (selectedPath === c.path) miniDiff = [] })
      .finally(() => { if (selectedPath === c.path) miniLoading = false })
  })
```

- [ ] **Step 2: Markup des cartes**

Remplacer le bloc `.vs` (lignes 205-220 originales) par :

```svelte
            {#if !selected.isBinary}
              {#if miniLoading}
                <p class="note"><Icon name="info" size={14} /> Loading changes…</p>
              {:else if miniDiff.length > 0}
                <div class="minidiff"><DiffBlock lines={miniDiff} maxLines={8} /></div>
              {/if}
            {/if}
            <div class="vs">
              <div class="vcard" class:pick={resolvedSide[selected.path] === 'mine'}>
                <div class="vhd">Mine · {target}</div>
                {#if selected.isBinary}
                  <div class="vthumb before">
                    {#if listThumbs.get(selected.path)}
                      <img class="vimg" src={listThumbs.get(selected.path)} alt="Mine — working copy" />
                    {:else}
                      <Icon name="image" size={24} />
                    {/if}
                  </div>
                {/if}
                <button class="keep" class:done={resolvedSide[selected.path] === 'mine'} disabled={busy !== ''} onclick={() => resolve(selected.path, 'mine')}>
                  {resolvedSide[selected.path] === 'mine' ? 'Kept mine' : 'Keep mine'}
                </button>
              </div>
              <div class="vcard" class:pick={resolvedSide[selected.path] === 'theirs'}>
                <div class="vhd">{wording.theirsCard}</div>
                {#if selected.isBinary}
                  <div class="vthumb after">
                    {#if listThumbs.get(theirsSidecar(selected.path))}
                      <img class="vimg" src={listThumbs.get(theirsSidecar(selected.path))} alt="Theirs — incoming version" />
                    {:else}
                      <Icon name="image" size={24} />
                    {/if}
                  </div>
                {/if}
                <button class="keep" class:done={resolvedSide[selected.path] === 'theirs'} disabled={busy !== ''} onclick={() => resolve(selected.path, 'theirs')}>
                  {resolvedSide[selected.path] === 'theirs' ? 'Kept theirs' : 'Keep theirs'}
                </button>
              </div>
            </div>
```

- [ ] **Step 3: Styles**

Dans le `<style>` de Merge.svelte, après la règle `.vthumb.after`, ajouter :

```css
  .vthumb { overflow: hidden; }
  .vimg { max-width: 100%; max-height: 100%; object-fit: contain; border-radius: 6px; }
  .minidiff { margin: 0 0 14px; }
```

(La première ligne complète la règle `.vthumb` existante — la garder en règle séparée pour ne pas retoucher l'existante.)

- [ ] **Step 4: Typecheck + suite**

Run: `npm run check && npx vitest run`
Expected: 0 erreur, toutes suites vertes.

- [ ] **Step 5: Vérification navigateur rapide (mock)**

`npm run dev` → Merge (via l'onglet Merge, source `feature/loot` → « Resolve & merge ») :
- conflits binaires : cartes Mine/Theirs avec vignettes damier mock (les DEUX — le sidecar passe par `stripTheirsSuffix` dans le mock) ;
- conflit texte `PlayerCharacter.cpp` : mini-diff (5 lignes mock) au-dessus des cartes, cartes sans vignette ;
- résolution → complete merge fonctionne comme avant.

- [ ] **Step 6: Commit**

```bash
git add src/lib/Merge.svelte
git commit -m "feat(merge): real previews in conflict cards (thumbs + mini-diff)"
```

---

## Item 6 — Cycle de vie du sidecar notifications (Job object)

### Task 18: Job object Windows kill-on-close

`windows-rs` n'est PAS dans `src-tauri/Cargo.toml` → crate `win32job` (choix léger prévu par la spec). Stratégie : assigner le **process app lui-même** à un job `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` au setup — tous les enfants spawnés ensuite (les `lore notification subscribe` de notifications.rs, y compris les respawns de la boucle de reconnexion) héritent du job automatiquement, sans bookkeeping par spawn. À la mort du process (même kill dur), l'OS ferme le dernier handle du job et tue tous les membres.

**Files:**
- Create: `src-tauri/src/job.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs` (module + setup)

- [ ] **Step 1: Ajouter la dépendance (Windows uniquement)**

Dans `src-tauri/Cargo.toml`, après le bloc `[dependencies]` :

```toml
[target.'cfg(windows)'.dependencies]
win32job = "2"
```

- [ ] **Step 2: Écrire le test qui échoue**

Créer `src-tauri/src/job.rs` avec SEULEMENT le module de test :

```rust
#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn job_installs_without_error() {
        // Assigning the test process to a kill-on-close job is harmless: the
        // job's last handle closes when the test process exits anyway.
        assert_eq!(super::init(), Ok(()));
    }
}
```

et déclarer le module dans `src-tauri/src/lib.rs` (après `mod config;`) :

```rust
mod job;
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml job`
Expected: FAIL à la compilation — `cannot find function init in module super`.

- [ ] **Step 3: Implémenter**

Compléter `src-tauri/src/job.rs` (au-dessus du module de test) :

```rust
//! Sidecar lifetime: ties the app process — and every child it spawns, in
//! particular the `lore notification subscribe` subscribers of
//! notifications.rs — to a Windows Job object with
//! JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE. When the app dies, even a hard kill,
//! the OS closes the job's last handle and terminates every member, so no
//! orphan `lore` process survives. Children join the job automatically at
//! spawn (no per-spawn bookkeeping, respawns included).

/// Install the kill-on-close job and put the current process in it. Called
/// once at startup; failure is non-fatal (the app runs, orphan cleanup just
/// isn't guaranteed — the caller logs and moves on).
#[cfg(windows)]
pub fn init() -> Result<(), String> {
    let job = win32job::Job::create().map_err(|e| e.to_string())?;
    let mut info = job.query_extended_limit_info().map_err(|e| e.to_string())?;
    info.limit_kill_on_job_close();
    job.set_extended_limit_info(&mut info).map_err(|e| e.to_string())?;
    job.assign_current_process().map_err(|e| e.to_string())?;
    // Keep the handle open for the whole process lifetime: dropping it would
    // close the job and kill us. The OS reclaims it at process death — which
    // is exactly the kill switch we want.
    std::mem::forget(job);
    Ok(())
}

/// POSIX: no Job objects; orphaned children are reparented and the P1 kill
/// path (generation bump + child.kill) already covers clean shutdowns.
#[cfg(not(windows))]
pub fn init() -> Result<(), String> {
    Ok(())
}
```

(Note : selon la version exacte de `win32job` 2.x résolue, `set_extended_limit_info` prend `&mut info` ou `&info` — le compilateur tranche ; ajuster l'emprunt si besoin, rien d'autre ne change.)

Puis dans `src-tauri/src/lib.rs`, dans le `.setup(|app| { … })`, après le bloc du plugin log :

```rust
      // Kill switch for the lore sidecars on ANY app death (P1 finding: hard
      // kill leaves `lore notification subscribe` orphans). Non-fatal on error.
      if let Err(e) = crate::job::init() {
          log::warn!("job object init failed — sidecars may outlive a hard kill: {e}");
      }
```

- [ ] **Step 4: Vérifier le passage + suite Rust complète**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: `test result: ok` (le smoke test `job_installs_without_error` inclus).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/job.rs src-tauri/src/lib.rs src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "fix(sidecar): kill-on-close job object ties lore children to the app (windows)"
```

(La vérification manuelle kill-dur est scriptée en Task 19 — pas d'unit test possible au-delà du smoke test.)

---

## Vérification finale

### Task 19: Suites complètes + navigateur mock + kill dur réel

(La vérification réelle `~theirs` a déjà été faite en tête d'item 2 — Task 12.)

- [ ] **Step 1: Les trois suites**

```powershell
npx vitest run
cargo test --manifest-path src-tauri/Cargo.toml
npm run check
```

Expected: vitest tous fichiers `passed` 0 failed ; cargo `test result: ok` ; svelte-check `0 errors and 0 warnings`.

- [ ] **Step 2: Parcours navigateur mock complet**

`npm run dev` → http://localhost:5173 (se connecter, ouvrir un repo mock) :

1. **Préchargement (item 3)** : ouvrir le repo, attendre ~1 s sur Changes, entrer dans History → les commits sont DÉJÀ là (pas de « Loading history… »).
2. **History preview (item 1)** : sélection/désélection au clic et à Enter, Escape, panneau avec vignette mock (fichier image), lecteur audio (`sfx_hit.wav` s'il apparaît dans un commit) ou icône fichier (`.cpp`), mention working-copy sur un commit ancien, absence de mention sur le commit de tête, « No longer in the working copy » sur une ligne delete, reset au changement de commit. Vérifier aussi que la vue **Changes** est inchangée (diff texte, compare binaire, lock, discard, timeline) — c'est la couverture navigateur des refactors Tasks 6/8/9/16.
3. **Merge (items 4 + 2)** :
   - setup → source `feature/loot` → « Resolve & merge into main » → cartes binaires avec vignettes mock des deux côtés, conflit texte avec mini-diff ;
   - bannière « Merging feature/loot into main » (source choisie ici → connue) ;
   - quitter la vue Merge (nav History) puis y revenir → reprise : bannière neutre « Resolving merge into main », carte « Theirs · incoming » ;
   - abort externe : dans la console devtools `localStorage.setItem('loredesktop.mock.externalAbort', '1')`, cliquer hors de la fenêtre puis refocus → toast « Merge was aborted outside the app » + retour phase setup ;
   - refaire un merge conflictuel → Abort local → comportement inchangé (pas de toast « outside the app »).
4. **Progression (item 5)** : RepoPicker (retirer le repo courant via le switcher ou utiliser l'écran initial) → Clone… → libellé « Cloning… 25% — 12.0 MB / 48.0 MB » qui progresse ; pendant un clone, ouvrir le RepoSwitcher → ses boutons de clone sont désactivés (et réciproquement).

- [ ] **Step 3: Vérification réelle kill dur (item 6)**

Dans un terminal (l'app réelle, repo réel ouvert pour que le subscriber tourne) :

```powershell
npm run tauri dev        # laisser démarrer, ouvrir C:\Users\jimmy\lore-test-repo dans l'app
```

Puis dans un second terminal PowerShell :

```powershell
Get-Process lore -ErrorAction SilentlyContinue | Format-Table Id, ProcessName   # attendu : ≥ 1 process (notification subscribe)
Stop-Process -Name app -Force                                                   # kill DUR du binaire dev (src-tauri/target/debug/app.exe)
Start-Sleep -Seconds 3
Get-Process lore -ErrorAction SilentlyContinue                                  # attendu : AUCUNE sortie
```

Expected: après le kill dur, `Get-Process lore` ne renvoie rien — zéro orphelin. Arrêter ensuite le process `npm run tauri dev` restant. (Si le binaire dev n'apparaît pas sous le nom `app`, vérifier avec `Get-Process | Where-Object { $_.Path -like '*lore-desktop*target*' }` et tuer ce process-là.)

- [ ] **Step 4: Rapport**

Consigner les écarts éventuels (nommage sidecar différent, contenu du diff en merge, signature win32job ajustée) en tête de ce plan, comme pour le P1.

# Lore Desktop — Lot P1 « artiste au quotidien » Implementation Plan

> **STATUT : EXÉCUTÉ ET VÉRIFIÉ le 2026-07-11** (subagent-driven, 18/18 tâches, double
> revue par tâche + revue finale d'ensemble : Ready). Suites : cargo 95, vitest 99,
> check 0. Vérification E2E réelle (Task 18) passée contre le serveur — delta réel,
> chip merge/staged en merge conflictuel réel, clone/push streaming sans timeout.
> Déviations conscientes vs plan : capture Task 13 exécutée APRÈS 14-17 (serveur
> indisponible une partie de la journée — les hypothèses slice B se sont révélées
> fausses, parseur réécrit sur les encodages réels) ; libellé clone en pourcentage
> seul (le « X / Y » de la spec n'est pas affiché — à acter ou ajouter, le champ
> `unit` est câblé bout en bout mais inutilisé) ; fixtures push/sync non committées
> (shapes pinnées par samples inline + README). Follow-ups consignés dans le rapport
> de session : garde globale anti double-clone cross-surface (low), vue Merge
> (nom de branche source erroné si merge démarré hors app + vue stale après abort
> externe — préexistants), sémantique du hash staged « vide » côté CLI (chip staged
> possiblement persistant — à remonter à l'équipe Lore), fenêtre de stall 60 s à
> discuter avant déploiement studio avec assets multi-GB, subscribers notifications
> orphelins après kill dur de l'app (préexistant).

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Livrer les 4 items du lot P1 (spec `docs/superpowers/specs/2026-07-10-lore-desktop-p1-artist-lot-design.md`) : delta de poids d'asset, section « Locked by teammates » stricte, chip StatusBar merge/staged, et progression streaming clone/sync/push avec détection de blocage remplaçant le timeout 45 s.

**Architecture:** Backend Rust (Tauri v2) : une commande batch `lore_file_sizes`, deux booléens dans le DTO status, et un nouveau runner `run_lore_streaming` (stall timeout 60 s, événement Tauri `lore://op-progress`) sur lequel basculent clone/sync/push uniquement. Frontend Svelte 5 : fonctions pures testées vitest (`sizeFormat`, `oldSizes`, `changesPartition`, `statusChip`, `progress`) + wiring dans `repo.svelte.ts` et markup dans Changes/FilePreview/StatusBar/TitleBar/RepoPicker/RepoSwitcher. Le mock est enrichi pour que les 4 features vivent en dev navigateur.

**Tech Stack:** Rust (serde_json, std::process + mpsc), Tauri v2 (Emitter, listen), Svelte 5 runes, TypeScript, vitest, cargo test.

---

## Contexte & conventions (à lire avant toute tâche)

- **Repo de test réel** : working copy `C:\Users\jimmy\lore-test-repo` (repo `desktoptest1`, id `019f333af5e073d28bb117ad1596784a`) sur `lore://lore.example.com:41337`. Binaire CLI : `C:\Users\jimmy\bin\lore.exe`. Même convention que les plans Slice A/B.
- **Gotcha cwd** : toute commande `lore` file-scoped (`file info`, `lock`, `diff`, `reset`…) résout les chemins relatifs contre le cwd du process, PAS contre `--repository`. Toujours passer des chemins **absolus** (voir `lore_set_lock`, `lore_diff` dans `src-tauri/src/commands.rs`).
- **Fixtures** : NDJSON réels dans `src-tauri/tests/fixtures/`, documentés dans `src-tauri/tests/fixtures/README.md`. Chaque tâche de capture DOIT (1) capturer, (2) inspecter, (3) adapter le code/les samples inline du plan si les noms wire diffèrent des hypothèses, (4) documenter l'encodage dans le README des fixtures.
- **Commandes de test** :
  - Rust : `cargo test --manifest-path src-tauri/Cargo.toml <filtre>` — attendu `test result: ok`.
  - Vitest : `npx vitest run <fichier>` — attendu `Test Files 1 passed`.
  - Typecheck : `npm run check` — attendu `svelte-check found 0 errors`.
- **CSS** : la variable d'avertissement du codebase est `--warn-bg` / `--warn-text` (la spec dit `--bg-warning` — c'est la même intention, utiliser `--warn-bg`).
- Ordre de livraison imposé : Item 1 (Tasks 1–5), Item 2 (Tasks 6–8), Item 3 (Tasks 9–12), Item 4 (Tasks 13–17), vérification finale (Task 18).

---

## Item 1 — Delta de poids d'asset

### Task 1: Capture réelle du `file info` batch

**Files:**
- Create: `src-tauri/tests/fixtures/file_info.ndjson`
- Modify: `src-tauri/tests/fixtures/README.md`

- [ ] **Step 1: Capturer la sortie batch réelle**

Dans PowerShell, à la racine du repo (`C:\Users\jimmy\Documents\SoonerOrLater\lore-desktop`). `README.md` et `notes.txt` existent dans le repo de test (plans file-history et fixtures locks) ; si l'un manque, lister le contenu du repo (`Get-ChildItem C:\Users\jimmy\lore-test-repo`) et prendre deux fichiers committés.

```powershell
$lore = "C:\Users\jimmy\bin\lore.exe"
$repo = "C:\Users\jimmy\lore-test-repo"
& $lore file info "$repo\README.md" "$repo\notes.txt" --repository $repo --json |
  Out-File -Encoding utf8NoBOM src-tauri\tests\fixtures\file_info.ndjson
Get-Content src-tauri\tests\fixtures\file_info.ndjson
```

Expected: plusieurs lignes NDJSON (une par fichier), terminées par `{"tagName":"complete","data":{"status":0}}`.

- [ ] **Step 2: Inspecter et pinner l'encodage**

Ouvrir la capture et noter : le `tagName` exact de l'événement par fichier (hypothèse du plan : `fileInfo`), le champ chemin (hypothèse : `path` — absolu ou relatif ?), le champ taille au dépôt (hypothèse : `size`, u64). **Si les noms réels diffèrent, adapter en Task 2** : la fonction `file_sizes_from` (tag + champs) et le sample inline `SAMPLE` du test `maps_reported_paths_back_to_relative`.

- [ ] **Step 3: Documenter dans le README des fixtures**

Ajouter à la fin de `src-tauri/tests/fixtures/README.md` (adapter les noms à la capture réelle) :

```markdown

**`fileInfo`** (`file info <paths…> --json`, batch — un événement par fichier) :
`path` (chemin tel qu'écho par le CLI), `size` (u64, taille à la révision courante
du dépôt). C'est le côté « ancien » du delta de poids ; le « nouveau » est le
`size` local de `repositoryStatusFile`.
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/tests/fixtures/file_info.ndjson src-tauri/tests/fixtures/README.md
git commit -m "test(fixtures): capture lore file info batch output"
```

### Task 2: Commande Rust `lore_file_sizes`

**Files:**
- Modify: `src-tauri/src/commands.rs` (ajouter après le bloc `lore_file_history` / `file_history_tests`, ~ligne 1165)
- Modify: `src-tauri/src/lib.rs` (invoke_handler)

- [ ] **Step 1: Écrire les tests qui échouent**

Ajouter dans `src-tauri/src/commands.rs`, après le module `file_history_tests` (adapter tag/champs du `SAMPLE` à la fixture de Task 1 si besoin) :

```rust
#[cfg(test)]
mod file_sizes_tests {
    use super::*;
    use crate::lore::parse_events;

    #[test]
    fn parses_file_info_fixture() {
        let events = parse_events(include_str!("../tests/fixtures/file_info.ndjson")).unwrap();
        let sizes = file_sizes_from(&events);
        assert!(!sizes.is_empty(), "the captured fixture must yield at least one size");
    }

    const SAMPLE: &str = concat!(
        r#"{"tagName":"fileInfo","data":{"path":"C:/Users/jimmy/lore-test-repo/notes.txt","size":420}}"#, "\n",
        r#"{"tagName":"fileInfo","data":{"path":"C:/Users/jimmy/lore-test-repo/Content/T_Cliff.uasset","size":4093640}}"#, "\n",
        r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
    );

    #[test]
    fn maps_reported_paths_back_to_relative() {
        let events = parse_events(SAMPLE).unwrap();
        let reported = file_sizes_from(&events);
        let out = relative_sizes(
            &reported,
            &["notes.txt".to_string(), "Content/T_Cliff.uasset".to_string(), "gone.txt".to_string()],
        );
        assert_eq!(out.get("notes.txt"), Some(&420));
        assert_eq!(out.get("Content/T_Cliff.uasset"), Some(&4093640));
        // Unreported file → absent from the map, never a fake 0.
        assert!(!out.contains_key("gone.txt"));
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test --manifest-path src-tauri/Cargo.toml file_sizes`
Expected: FAIL à la compilation — `error[E0425]: cannot find function 'file_sizes_from'`.

- [ ] **Step 3: Implémenter parse + commande**

Ajouter juste au-dessus du module `file_sizes_tests` :

```rust
/// Sizes at the current repository revision, from `lore file info <paths…>`
/// (batch). Keyed by the path as the event reports it, `\` normalized to `/`.
/// Event/field names pinned against tests/fixtures/file_info.ndjson.
fn file_sizes_from(events: &[LoreEvent]) -> std::collections::HashMap<String, u64> {
    let mut out = std::collections::HashMap::new();
    for d in events_with_tag(events, "fileInfo") {
        if let (Some(path), Some(size)) = (
            d.get("path").and_then(|v| v.as_str()),
            d.get("size").and_then(|v| v.as_u64()),
        ) {
            out.insert(path.replace('\\', "/"), size);
        }
    }
    out
}

/// Match each requested repo-relative path against the reported sizes: exact
/// key first, else a suffix match (the CLI echoes back the absolute paths it
/// was given). Unmatched paths are simply absent — never a fake 0.
fn relative_sizes(
    reported: &std::collections::HashMap<String, u64>,
    paths: &[String],
) -> std::collections::HashMap<String, u64> {
    let mut out = std::collections::HashMap::new();
    for rel in paths {
        let norm = rel.replace('\\', "/");
        let found = reported.get(&norm).copied().or_else(|| {
            let suffix = format!("/{norm}");
            reported.iter().find(|(k, _)| k.ends_with(&suffix)).map(|(_, v)| *v)
        });
        if let Some(size) = found {
            out.insert(rel.clone(), size);
        }
    }
    out
}

/// Repository-revision ("old") sizes of the given files, in ONE batch
/// `lore file info` call. Paths are passed absolute (`lore file` resolves
/// relative paths against the process cwd — same gotcha as lock/diff/reset).
/// Pure enrichment: the frontend calls it fire-and-forget after status.
#[tauri::command]
pub async fn lore_file_sizes(
    repo_path: String,
    paths: Vec<String>,
) -> Result<std::collections::HashMap<String, u64>, String> {
    if paths.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    blocking(move || {
        let abs: Vec<String> = paths
            .iter()
            .map(|p| std::path::Path::new(&repo_path).join(p).to_string_lossy().into_owned())
            .collect();
        let mut args: Vec<&str> = vec!["file", "info"];
        args.extend(abs.iter().map(|s| s.as_str()));
        args.extend(["--repository", &repo_path]);
        let events = run_lore(&args)?;
        Ok(relative_sizes(&file_sizes_from(&events), &paths))
    })
    .await
}
```

- [ ] **Step 4: Enregistrer la commande**

Dans `src-tauri/src/lib.rs`, ajouter dans le `invoke_handler`, après `commands::lore_file_history,` :

```rust
        commands::lore_file_sizes,
```

- [ ] **Step 5: Vérifier le PASS**

Run: `cargo test --manifest-path src-tauri/Cargo.toml file_sizes`
Expected: `test result: ok. 2 passed`.

- [ ] **Step 6: Test manuel contre le repo réel (sanity)**

```powershell
cargo build --manifest-path src-tauri/Cargo.toml
```

Expected: compile sans warning nouveau. (L'appel réel bout-en-bout est vérifié en Task 18.)

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(sizes): lore_file_sizes batch command (repository-revision sizes)"
```

### Task 3: Contrat frontend `fileSizes` + fusion `oldSize` + mock

**Files:**
- Create: `src/lib/oldSizes.ts`
- Create: `src/lib/oldSizes.test.ts`
- Create: `src/lib/mockOps.test.ts`
- Modify: `src/lib/types.ts` (interface `LoreApi`)
- Modify: `src/lib/tauri.ts`
- Modify: `src/lib/mock.ts`

- [ ] **Step 1: Écrire les tests qui échouent**

Créer `src/lib/oldSizes.test.ts` :

```ts
import { describe, it, expect } from 'vitest'
import { mergeOldSizes, sizeLookupPaths } from './oldSizes'
import type { ChangedFile } from './types'

const f = (path: string, action: ChangedFile['action'], size = 100): ChangedFile =>
  ({ path, action, isBinary: false, size })

describe('sizeLookupPaths', () => {
  it('keeps only modify and delete', () => {
    const files = [f('a.txt', 'add'), f('b.txt', 'modify'), f('c.txt', 'delete'), f('d.txt', 'move')]
    expect(sizeLookupPaths(files)).toEqual(['b.txt', 'c.txt'])
  })
})

describe('mergeOldSizes', () => {
  it('annotates modify and delete rows with oldSize', () => {
    const out = mergeOldSizes([f('a.txt', 'modify', 120), f('b.txt', 'delete', 0)], { 'a.txt': 100, 'b.txt': 3400 })
    expect(out[0].oldSize).toBe(100)
    expect(out[1].oldSize).toBe(3400)
  })
  it('never annotates adds, even if a size is reported', () => {
    const out = mergeOldSizes([f('a.txt', 'add')], { 'a.txt': 50 })
    expect(out[0].oldSize).toBeUndefined()
  })
  it('leaves files without a reported size untouched', () => {
    const out = mergeOldSizes([f('a.txt', 'modify')], {})
    expect(out[0].oldSize).toBeUndefined()
  })
  it('ignores reported paths that vanished from the list (status/file-info race)', () => {
    const out = mergeOldSizes([f('a.txt', 'modify', 120)], { 'a.txt': 100, 'gone.txt': 999 })
    expect(out).toHaveLength(1)
    expect(out[0].oldSize).toBe(100)
  })
})
```

Créer `src/lib/mockOps.test.ts` :

```ts
import { describe, it, expect } from 'vitest'
import { mock } from './mock'

describe('mock.fileSizes', () => {
  it('returns old sizes for known modified files only', async () => {
    const sizes = await mock.fileSizes('C:/repos/game', [
      'Content/Maps/Level_01.umap',
      'Content/Characters/Hero/SK_Hero.uasset', // add — no old size seeded
    ])
    expect(sizes['Content/Maps/Level_01.umap']).toBe(2100480)
    expect(sizes['Content/Characters/Hero/SK_Hero.uasset']).toBeUndefined()
  })
})
```

- [ ] **Step 2: Vérifier l'échec**

Run: `npx vitest run src/lib/oldSizes.test.ts src/lib/mockOps.test.ts`
Expected: FAIL — `Cannot find module './oldSizes'` et `mock.fileSizes is not a function`.

- [ ] **Step 3: Implémenter**

Créer `src/lib/oldSizes.ts` :

```ts
import type { ChangedFile } from './types'

/** The paths worth a `file info` lookup: only modify/delete have an "old" size. */
export function sizeLookupPaths(files: ChangedFile[]): string[] {
  return files.filter((f) => f.action === 'modify' || f.action === 'delete').map((f) => f.path)
}

/**
 * Merge repository-revision sizes into the change list as `oldSize`.
 * Only modify/delete rows are enriched; paths missing from `sizes` (fetch
 * failed for that file, or the file vanished between status and file info)
 * are left untouched. Pure — used by the fire-and-forget enrichment.
 */
export function mergeOldSizes(files: ChangedFile[], sizes: Record<string, number>): ChangedFile[] {
  return files.map((f) =>
    (f.action === 'modify' || f.action === 'delete') && sizes[f.path] != null
      ? { ...f, oldSize: sizes[f.path] }
      : f,
  )
}
```

Dans `src/lib/types.ts`, ajouter à l'interface `LoreApi`, juste après la ligne `getStatus(repoPath: string): Promise<StatusResult>` :

```ts
  /** Repository-revision sizes of the given files (ONE batch `file info` call) — the "old" side of the size delta. */
  fileSizes(repoPath: string, paths: string[]): Promise<Record<string, number>>
```

Dans `src/lib/tauri.ts`, ajouter après la ligne `getStatus: ...` :

```ts
  fileSizes: (repoPath, paths) => invoke<Record<string, number>>('lore_file_sizes', { repoPath, paths }),
```

Dans `src/lib/mock.ts` :

1. Dans `seedFiles()`, **supprimer tous les `oldSize`** (le join fire-and-forget doit être exercé en dev aussi). La fonction devient :

```ts
function seedFiles(): ChangedFile[] {
  return [
    { path: 'Content/Maps/Level_01.umap', action: 'modify', isBinary: true, size: 2359296, lockedBy: 'you' },
    { path: 'Content/Characters/Hero/SK_Hero.uasset', action: 'add', isBinary: true, size: 4718592 },
    { path: 'Content/Environment/T_Cliff_D.uasset', action: 'modify', isBinary: true, size: 4404019, lockedBy: 'Maya R' },
    { path: 'Content/UI/T_Icon_Sword.png', action: 'modify', isBinary: true, size: 182044 },
    { path: 'Audio/sfx_hit.wav', action: 'add', isBinary: true, size: 912384 },
    { path: 'Content/Props/SM_Crate.obj', action: 'add', isBinary: true, size: 20480 },
    { path: 'Source/Player/PlayerCharacter.cpp', action: 'modify', isBinary: false, size: 8241 },
    { path: 'Source/Player/PlayerCharacter.h', action: 'modify', isBinary: false, size: 1204 },
    { path: 'Config/DefaultInput.ini', action: 'modify', isBinary: false, size: 512 },
    { path: 'Docs/old-notes.md', action: 'delete', isBinary: false, size: 0 },
  ]
}
```

2. Ajouter après `seedFiles()` :

```ts
// "Old" (repository-revision) sizes served by fileSizes, so the browser dev
// exercises the same fire-and-forget enrichment as the real app (deltas pop
// in ~400 ms after the status). T_Icon_Sword old == new → no delta shown.
const MOCK_OLD_SIZES: Record<string, number> = {
  'Content/Maps/Level_01.umap': 2100480,
  'Content/Environment/T_Cliff_D.uasset': 4093640,
  'Content/UI/T_Icon_Sword.png': 182044,
  'Source/Player/PlayerCharacter.cpp': 7980,
  'Source/Player/PlayerCharacter.h': 1180,
  'Config/DefaultInput.ini': 500,
  'Docs/old-notes.md': 3400,
}
```

3. Ajouter dans l'objet `mock`, après `getStatus` :

```ts
  async fileSizes(_repoPath: string, paths: string[]) {
    await delay(400)
    const out: Record<string, number> = {}
    for (const p of paths) if (MOCK_OLD_SIZES[p] != null) out[p] = MOCK_OLD_SIZES[p]
    return out
  },
```

- [ ] **Step 4: Vérifier le PASS**

Run: `npx vitest run src/lib/oldSizes.test.ts src/lib/mockOps.test.ts`
Expected: `Test Files 2 passed`.

Run: `npm run check`
Expected: 0 errors (le contrat `LoreApi` est satisfait par mock ET tauri).

- [ ] **Step 5: Commit**

```bash
git add src/lib/oldSizes.ts src/lib/oldSizes.test.ts src/lib/mockOps.test.ts src/lib/types.ts src/lib/tauri.ts src/lib/mock.ts
git commit -m "feat(sizes): fileSizes API contract, oldSize merge helper, mock enrichment"
```

### Task 4: `sizeFormat.ts` — delta signé + fmtSize partagé

**Files:**
- Create: `src/lib/sizeFormat.ts`
- Create: `src/lib/sizeFormat.test.ts`
- Modify: `src/lib/FilePreview.svelte` (remplacer le fmtSize local)

- [ ] **Step 1: Écrire les tests qui échouent**

Créer `src/lib/sizeFormat.test.ts` :

```ts
import { describe, it, expect } from 'vitest'
import { fmtSize, formatDelta } from './sizeFormat'

describe('fmtSize', () => {
  it('formats bytes, KB and MB', () => {
    expect(fmtSize(512)).toBe('512 B')
    expect(fmtSize(2048)).toBe('2.0 KB')
    expect(fmtSize(2359296)).toBe('2.3 MB')
  })
})

describe('formatDelta', () => {
  it('shows a signed compact delta for a grown modified file', () => {
    // delta = 1 MiB exactly → "+1.0 MB"
    expect(formatDelta({ action: 'modify', size: 3 * 1048576, oldSize: 2 * 1048576 })).toBe('+1.0 MB')
    // delta = 258 816 B → formatted in KB, not MB
    expect(formatDelta({ action: 'modify', size: 2359296, oldSize: 2100480 })).toBe('+252.8 KB')
  })
  it('shows a signed compact delta for a shrunk modified file', () => {
    expect(formatDelta({ action: 'modify', size: 2 * 1048576, oldSize: 3 * 1048576 })).toBe('−1.0 MB')
  })
  it('returns null when the delta is zero', () => {
    expect(formatDelta({ action: 'modify', size: 100, oldSize: 100 })).toBeNull()
  })
  it('returns null when the old size is unknown', () => {
    expect(formatDelta({ action: 'modify', size: 100 })).toBeNull()
  })
  it('shows the old size alone for a delete (no sign, no arrow)', () => {
    expect(formatDelta({ action: 'delete', size: 0, oldSize: 2097152 })).toBe('2.0 MB')
  })
  it('returns null for a delete without a known old size', () => {
    expect(formatDelta({ action: 'delete', size: 0 })).toBeNull()
  })
  it('returns null for adds (unchanged from today)', () => {
    expect(formatDelta({ action: 'add', size: 4718592, oldSize: 10 })).toBeNull()
  })
})
```

- [ ] **Step 2: Vérifier l'échec**

Run: `npx vitest run src/lib/sizeFormat.test.ts`
Expected: FAIL — `Cannot find module './sizeFormat'`.

- [ ] **Step 3: Implémenter**

Créer `src/lib/sizeFormat.ts` :

```ts
import type { ChangedFile } from './types'

const KB = 1024
const MB = 1024 * 1024

export function fmtSize(n: number): string {
  if (n >= MB) return (n / MB).toFixed(1) + ' MB'
  if (n >= KB) return (n / KB).toFixed(1) + ' KB'
  return n + ' B'
}

/**
 * Compact end-of-row size annotation for the Changes list.
 * - modify: signed delta ("+0.3 MB" / "−0.1 MB"), null when unknown or zero —
 *   neutral secondary color in the UI (growing is not a fault).
 * - delete: the old size alone ("2.0 MB"), no arrow, no sign.
 * - add/move/copy: null (unchanged from today).
 */
export function formatDelta(f: Pick<ChangedFile, 'action' | 'size' | 'oldSize'>): string | null {
  if (f.action === 'delete') return f.oldSize != null ? fmtSize(f.oldSize) : null
  if (f.action !== 'modify' || f.oldSize == null) return null
  const delta = f.size - f.oldSize
  if (delta === 0) return null
  return (delta > 0 ? '+' : '−') + fmtSize(Math.abs(delta))
}
```

- [ ] **Step 4: Vérifier le PASS**

Run: `npx vitest run src/lib/sizeFormat.test.ts`
Expected: `8 passed`.

- [ ] **Step 5: DRY — FilePreview importe le fmtSize partagé + affiche l'ancienne taille des deletes**

Dans `src/lib/FilePreview.svelte` :

1. Ajouter l'import en tête de script : `import { fmtSize } from './sizeFormat'`
2. Supprimer les lignes locales :

```ts
  const KB = 1024, MB = 1024 * 1024
  function fmtSize(n: number): string {
    if (n >= MB) return (n / MB).toFixed(1) + ' MB'
    if (n >= KB) return (n / KB).toFixed(1) + ' KB'
    return n + ' B'
  }
```

3. Remplacer le derived `sizeText` par :

```ts
  const sizeText = $derived(
    !file ? ''
    : file.action === 'delete' && file.oldSize != null ? fmtSize(file.oldSize)
    : file.action === 'modify' && file.oldSize != null ? `${fmtSize(file.oldSize)} → ${fmtSize(file.size)}`
    : fmtSize(file.size),
  )
```

- [ ] **Step 6: Vérifier**

Run: `npm run check && npx vitest run`
Expected: 0 erreurs svelte-check, toutes les suites vitest passent.

- [ ] **Step 7: Commit**

```bash
git add src/lib/sizeFormat.ts src/lib/sizeFormat.test.ts src/lib/FilePreview.svelte
git commit -m "feat(sizes): shared fmtSize + signed compact delta formatter"
```

### Task 5: Wiring fire-and-forget + delta dans la liste Changes

**Files:**
- Modify: `src/lib/repo.svelte.ts`
- Modify: `src/lib/Changes.svelte`

- [ ] **Step 1: Wiring dans repo.svelte.ts**

1. Ajouter aux imports : `import { mergeOldSizes, sizeLookupPaths } from './oldSizes'`
2. Ajouter après la fonction `refreshBranches` :

```ts
// Fire-and-forget enrichment: fetch the repository-revision sizes of the
// modified/deleted files (ONE batch call) and merge them in as `oldSize`.
// Failure or timeout is TOTAL silence — the deltas simply don't appear.
// Never a toast for enrichment.
async function refreshFileSizes() {
  const path = session.config.currentRepo
  if (!path || !repo.status) return
  const paths = sizeLookupPaths(repo.status.files)
  if (paths.length === 0) return
  let sizes: Record<string, number>
  try { sizes = await api.fileSizes(path, paths) } catch { return }
  // The status may have been replaced while the sizes were in flight — only
  // annotate the current one (paths that vanished are ignored by the merge).
  if (repo.status && session.config.currentRepo === path)
    repo.status.files = mergeOldSizes(repo.status.files, sizes)
}
```

3. Dans `refreshStatus`, après `refreshBranches(true)`, ajouter :

```ts
  refreshFileSizes()
```

- [ ] **Step 2: Delta dans Changes.svelte**

1. Ajouter l'import : `import { formatDelta } from './sizeFormat'`
2. Dans le markup de la ligne fichier, insérer le delta juste après le `<span class="path">…</span>` (avant le bloc `{#if f.lockedBy === 'you'}`) :

```svelte
              {#if formatDelta(f)}<span class="delta">{formatDelta(f)}</span>{/if}
```

3. Dans le `<style>` : modifier la règle `.path` pour qu'elle pousse les badges à droite (le delta n'a pas besoin de `margin-left: auto`), et ajouter `.delta` :

```css
  .path { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; font-size: 12.5px; }
  .delta { flex-shrink: 0; font-size: 10.5px; font-family: var(--font-mono); color: var(--text-muted); }
```

- [ ] **Step 3: Vérifier (suites + dev navigateur)**

Run: `npm run check && npx vitest run`
Expected: tout passe.

Run: `npm run dev` puis ouvrir l'URL vite dans un navigateur, sélectionner un repo mock.
Expected: ~400 ms après l'affichage de la liste, `Level_01.umap` affiche « +252.8 KB », `PlayerCharacter.cpp` « +261 B », `old-notes.md` (delete) « 3.3 KB » sans signe, `T_Icon_Sword.png` rien (delta nul), `SK_Hero.uasset` (add) rien. FilePreview d'un modify : « 2.0 MB → 2.3 MB » ; d'un delete : « 3.3 KB ».

- [ ] **Step 4: Commit**

```bash
git add src/lib/repo.svelte.ts src/lib/Changes.svelte
git commit -m "feat(sizes): fire-and-forget oldSize enrichment + delta column in Changes"
```

---

## Item 2 — Fichiers verrouillés par un coéquipier

### Task 6: Partition committables / verrouillés (`changesPartition.ts`)

**Files:**
- Create: `src/lib/changesPartition.ts`
- Create: `src/lib/changesPartition.test.ts`

- [ ] **Step 1: Écrire les tests qui échouent**

Créer `src/lib/changesPartition.test.ts` :

```ts
import { describe, it, expect } from 'vitest'
import { partitionByLock, filterByQuery } from './changesPartition'
import type { ChangedFile } from './types'

const f = (path: string, lockedBy?: string | null): ChangedFile =>
  ({ path, action: 'modify', isBinary: false, size: 10, lockedBy })

describe('partitionByLock', () => {
  it('keeps unlocked and self-locked files committable', () => {
    const { committable, lockedByOthers } = partitionByLock([f('a'), f('b', null), f('c', 'you')])
    expect(committable.map((x) => x.path)).toEqual(['a', 'b', 'c'])
    expect(lockedByOthers).toEqual([])
  })
  it('moves teammate-locked files to the locked group, order preserved', () => {
    const { committable, lockedByOthers } = partitionByLock([f('a', 'Maya R'), f('b'), f('c', 'Alex L')])
    expect(committable.map((x) => x.path)).toEqual(['b'])
    expect(lockedByOthers.map((x) => x.path)).toEqual(['a', 'c'])
  })
  it('the commit counter counts committables only', () => {
    const { committable } = partitionByLock([f('a', 'Maya R'), f('b'), f('c', 'you')])
    expect(committable.length).toBe(2)
  })
})

describe('filterByQuery', () => {
  const files = [f('Content/Maps/Level_01.umap'), f('Source/Player.cpp', 'Maya R')]
  it('returns everything for a blank query', () => {
    expect(filterByQuery(files, '  ')).toEqual(files)
  })
  it('matches case-insensitively anywhere in the path', () => {
    expect(filterByQuery(files, 'PLAYER').map((x) => x.path)).toEqual(['Source/Player.cpp'])
  })
  it('applies to locked files too (the filter spans both groups)', () => {
    const { lockedByOthers } = partitionByLock(filterByQuery(files, 'player'))
    expect(lockedByOthers.map((x) => x.path)).toEqual(['Source/Player.cpp'])
  })
})
```

- [ ] **Step 2: Vérifier l'échec**

Run: `npx vitest run src/lib/changesPartition.test.ts`
Expected: FAIL — `Cannot find module './changesPartition'`.

- [ ] **Step 3: Implémenter**

Créer `src/lib/changesPartition.ts` :

```ts
import type { ChangedFile } from './types'

export interface ChangesPartition {
  /** Files that can be committed: unlocked, or locked by the current user. */
  committable: ChangedFile[]
  /** Files locked by a teammate — excluded from commit by construction. */
  lockedByOthers: ChangedFile[]
}

export function partitionByLock(files: ChangedFile[]): ChangesPartition {
  const committable: ChangedFile[] = []
  const lockedByOthers: ChangedFile[] = []
  for (const f of files) {
    if (f.lockedBy && f.lockedBy !== 'you') lockedByOthers.push(f)
    else committable.push(f)
  }
  return { committable, lockedByOthers }
}

/** Case-insensitive substring filter on the path; blank query = everything. */
export function filterByQuery(files: ChangedFile[], query: string): ChangedFile[] {
  const q = query.trim().toLowerCase()
  if (!q) return files
  return files.filter((f) => f.path.toLowerCase().includes(q))
}
```

- [ ] **Step 4: Vérifier le PASS**

Run: `npx vitest run src/lib/changesPartition.test.ts`
Expected: `6 passed`.

- [ ] **Step 5: Commit**

```bash
git add src/lib/changesPartition.ts src/lib/changesPartition.test.ts
git commit -m "feat(locks): committable/teammate-locked partition helpers"
```

### Task 7: Section stricte dans Changes.svelte + compteurs

**Files:**
- Modify: `src/lib/Changes.svelte`
- Modify: `src/lib/NavRail.svelte`

- [ ] **Step 1: Réécrire les deriveds et l'effect de staging**

Dans `src/lib/Changes.svelte` :

1. Ajouter l'import : `import { partitionByLock, filterByQuery } from './changesPartition'`
2. Remplacer le bloc de deriveds actuel (`files`/`query`/`shown`/`branch`/`stagedCount`) par :

```ts
  const files = $derived(repo.status?.files ?? [])
  const parts = $derived(partitionByLock(files))
  const shownCommittable = $derived(filterByQuery(parts.committable, filter))
  const shownLocked = $derived(filterByQuery(parts.lockedByOthers, filter))
  const shownCount = $derived(shownCommittable.length + shownLocked.length)
  const branch = $derived(repo.status?.branch ?? 'main')
  const stagedCount = $derived(parts.committable.filter((f) => staged.has(f.path)).length)
```

3. Remplacer l'effect de staging par :

```ts
  // Default: every committable file staged. Teammate-locked files are NEVER
  // staged — exclusion by construction (doCommit excludes everything unstaged),
  // so there is no way to commit them from the app.
  $effect(() => {
    staged = new Set(parts.committable.map((f) => f.path))
  })
```

4. Mettre à jour le commentaire de `doCommit` (le code ne change pas) :

```ts
  async function doCommit() {
    // Everything not staged is excluded — that covers unchecked committables
    // AND every teammate-locked file (never in `staged`, by construction).
    const exclude = files.filter((f) => !staged.has(f.path)).map((f) => f.path)
    await commit(composeCommitMessage(message, description), exclude)
    message = ''
    description = ''
  }
```

5. Dans le colhead, remplacer `shown.length` par `shownCount` :

```svelte
  <div class="colhead">Changes <span class="n">{filter.trim() ? `${shownCount} of ${files.length} files` : `${files.length} ${files.length === 1 ? 'file' : 'files'}`}</span></div>
```

- [ ] **Step 2: Réécrire le markup de la liste (deux groupes)**

Remplacer tout le contenu de `<div class="filelist">…</div>` par :

```svelte
  <div class="filelist">
    {#if repo.busy === 'status' && !repo.status}
      <p class="muted pad">Scanning…</p>
    {:else if files.length === 0}
      <div class="empty muted"><p>No local changes.</p></div>
    {:else if shownCount === 0}
      <p class="muted pad">No files match.</p>
    {:else}
      <ul>
        {#each shownCommittable as f (f.path)}
          <li class="file" class:sel={f.path === selectedPath}
              oncontextmenu={(e) => { e.preventDefault(); ctxMenu = { x: e.clientX, y: e.clientY, path: f.path } }}>
            <input type="checkbox" checked={staged.has(f.path)} onchange={() => toggle(f.path)} title="Stage this file" aria-label="Stage {f.path}" />
            <div class="rowmain" role="button" tabindex="0"
                 onclick={() => onselect(f.path)}
                 onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onselect(f.path) } }}>
              <span class="tag {glyph[f.action]?.c}">{glyph[f.action]?.v ?? '?'}</span>
              {#if listThumbs.get(f.path)}<img class="rowthumb" src={listThumbs.get(f.path)} alt="" />{/if}
              <span class="path"><span class="dir">{dir(f.path)}</span>{base(f.path)}</span>
              {#if formatDelta(f)}<span class="delta">{formatDelta(f)}</span>{/if}
              {#if f.lockedBy === 'you'}
                <span class="lock"><Icon name="lock" size={11} /> you</span>
              {:else if f.isBinary}
                <span class="bin">bin</span>
              {/if}
            </div>
          </li>
        {/each}
      </ul>
      {#if shownLocked.length > 0}
        <div class="lockedhead">
          <Icon name="lock" size={12} />
          <span>Locked by teammates ({shownLocked.length}) — excluded from commit</span>
        </div>
        <ul>
          {#each shownLocked as f (f.path)}
            <li class="file locked" class:sel={f.path === selectedPath}
                oncontextmenu={(e) => { e.preventDefault(); ctxMenu = { x: e.clientX, y: e.clientY, path: f.path } }}>
              <div class="rowmain" role="button" tabindex="0"
                   onclick={() => onselect(f.path)}
                   onkeydown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onselect(f.path) } }}>
                <span class="tag {glyph[f.action]?.c}">{glyph[f.action]?.v ?? '?'}</span>
                {#if listThumbs.get(f.path)}<img class="rowthumb" src={listThumbs.get(f.path)} alt="" />{/if}
                <span class="path"><span class="dir">{dir(f.path)}</span>{base(f.path)}</span>
                {#if formatDelta(f)}<span class="delta">{formatDelta(f)}</span>{/if}
                <span class="lock other"><Icon name="lock" size={11} /> {f.lockedBy}</span>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    {/if}
  </div>
```

Notes :
- **Pas de checkbox** dans le groupe verrouillé ; les lignes restent sélectionnables (preview) et gardent le menu contextuel existant (`ctxItems` n'offre déjà ni Lock ni Unlock quand `lockedBy` est un tiers ; Reveal/Open/Copy/Discard restent — Discard est légitime, c'est la copie locale).
- Le branch `{:else if f.lockedBy}` de l'ancien markup disparaît du groupe committable (ces fichiers sont maintenant dans le groupe du bas).

- [ ] **Step 3: CSS de la section**

Ajouter dans le `<style>` :

```css
  .lockedhead { display: flex; align-items: center; gap: 6px; padding: 10px 12px 4px; font-size: 11px; color: var(--warn-text); border-top: 1px solid var(--border); margin-top: 4px; }
  .file.locked .rowmain { opacity: .62; padding-left: 22px; }
```

(22px = largeur checkbox 14px + gap 8px, pour aligner les chemins.)

- [ ] **Step 4: Badge NavRail = committables seulement**

Dans `src/lib/NavRail.svelte` :

1. Ajouter l'import : `import { partitionByLock } from './changesPartition'`
2. Remplacer la ligne `const changed = ...` par :

```ts
  const changed = $derived(partitionByLock(repo.status?.files ?? []).committable.length)
```

- [ ] **Step 5: Vérifier (suites + dev navigateur)**

Run: `npm run check && npx vitest run`
Expected: tout passe.

Run: `npm run dev` + navigateur :
- `T_Cliff_D.uasset` (Maya R) et `SK_Hero.uasset` (Alex L, joint via getLocks) sont dans la section ambre « Locked by teammates (2) — excluded from commit », sans checkbox, estompés, avec le nom du détenteur.
- Le bouton Commit affiche 8 files (10 − 2) ; le badge Changes du NavRail affiche 8.
- Filtrer « cliff » → seule la section verrouillée montre une ligne, compteur (1).
- Committer → les 2 fichiers verrouillés restent dans la liste (exclus du commit).
- Menu contextuel d'une ligne verrouillée : Reveal / Open / Copy / Discard, pas de Lock/Unlock.

- [ ] **Step 6: Commit**

```bash
git add src/lib/Changes.svelte src/lib/NavRail.svelte
git commit -m "feat(locks): strict teammate-locked section, excluded from commit by construction"
```

### Task 8: Bandeau d'avertissement FilePreview

**Files:**
- Modify: `src/lib/FilePreview.svelte`

- [ ] **Step 1: Markup du bandeau**

Dans `src/lib/FilePreview.svelte`, juste après la fermeture `</header>` (avant le bloc `{#if file.isBinary}`), ajouter :

```svelte
      {#if file.lockedBy && file.lockedBy !== 'you'}
        <div class="lockwarn">
          <Icon name="lock" size={14} />
          <span>Locked by {file.lockedBy} — excluded from commit while locked</span>
        </div>
      {/if}
```

- [ ] **Step 2: CSS**

Ajouter dans le `<style>` :

```css
  .lockwarn { display: flex; align-items: center; gap: 8px; background: var(--warn-bg); color: var(--warn-text); border-radius: 8px; padding: 9px 12px; font-size: 12px; margin: 0 0 14px; }
```

(La ligne « Lock » discrète existante reste en bas du panneau — elle couvre le cas `you` et le bouton Lock/Unlock.)

- [ ] **Step 3: Vérifier**

Run: `npm run check`
Expected: 0 errors.

`npm run dev` + navigateur : sélectionner `T_Cliff_D.uasset` → bandeau ambre « Locked by Maya R — excluded from commit while locked » en tête de panneau ; sélectionner `Level_01.umap` (locké par `you`) → PAS de bandeau, la ligne Lock du bas montre « Locked by you » + bouton Unlock.

- [ ] **Step 4: Commit**

```bash
git add src/lib/FilePreview.svelte
git commit -m "feat(locks): teammate-lock warning banner in FilePreview"
```

---

## Item 3 — Chip StatusBar « merge in progress / staged state »

### Task 9: Capture réelle du status pendant un merge et avec un état stagé

**Files:**
- Create: `src-tauri/tests/fixtures/status_merge.ndjson`
- Create: `src-tauri/tests/fixtures/status_staged.ndjson`
- Modify: `src-tauri/tests/fixtures/README.md`

- [ ] **Step 1: Fabriquer un merge conflictuel dans le repo de test et capturer**

```powershell
$lore = "C:\Users\jimmy\bin\lore.exe"
$repo = "C:\Users\jimmy\lore-test-repo"
# Branche qui va conflicter avec main sur README.md :
& $lore branch create p1-chip-test --repository $repo
Add-Content "$repo\README.md" "line from p1-chip-test"
& $lore stage . --scan --repository $repo
& $lore commit "p1 chip test: branch side" --repository $repo
& $lore branch switch main --repository $repo
Add-Content "$repo\README.md" "line from main"
& $lore stage . --scan --repository $repo
& $lore commit "p1 chip test: main side" --repository $repo
# Merge en cours → capture → abort :
& $lore branch merge start p1-chip-test --repository $repo
& $lore status --repository $repo --json | Out-File -Encoding utf8NoBOM src-tauri\tests\fixtures\status_merge.ndjson
& $lore branch merge abort --repository $repo
& $lore branch archive p1-chip-test --repository $repo
```

Expected: `status_merge.ndjson` non vide, terminé par `complete` status 0.

- [ ] **Step 2: Capturer un état stagé résiduel**

```powershell
Add-Content "$repo\README.md" "staged capture line"
& $lore stage . --scan --repository $repo
& $lore status --repository $repo --json | Out-File -Encoding utf8NoBOM src-tauri\tests\fixtures\status_staged.ndjson
& $lore commit "p1 chip test: staged capture" --repository $repo
```

Expected: `status_staged.ndjson` non vide ; le commit final nettoie l'état stagé du repo de test.

- [ ] **Step 3: Inspecter et pinner les noms wire**

Ouvrir les deux captures et regarder la ligne `repositoryStatusRevision` :
- Quel(s) champ(s) signalent le merge en cours ? (hypothèse du plan : `revisionMerged`, hash non-zéro ; variantes possibles `revisionMergedNumber`, `revisionMergedSource`…)
- Quel champ signale l'état stagé ? (hypothèse : `revisionStaged`, hash non-zéro ; un hash all-zeros = pas d'état.)

**Si les noms/types réels diffèrent, adapter en Task 10** : l'extraction dans `status_from` et les samples inline des tests `merge_and_staged_flags` / `zero_or_absent_merge_fields_are_false`. Documenter dans `src-tauri/tests/fixtures/README.md` en ajoutant :

```markdown

**Merge/staged dans `repositoryStatusRevision`** : `revisionMerged` (hash — non-zéro
= merge en cours) et `revisionStaged` (hash — non-zéro = état stagé résiduel).
Un hash all-zeros (64 × '0') ou un champ absent (CLI plus ancien) = false.
Captures : status_merge.ndjson (pendant `branch merge start` conflictuel),
status_staged.ndjson (après `stage .` sans commit).
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/tests/fixtures/status_merge.ndjson src-tauri/tests/fixtures/status_staged.ndjson src-tauri/tests/fixtures/README.md
git commit -m "test(fixtures): capture status during merge and with staged state"
```

### Task 10: Booléens `merge_in_progress` / `staged_pending` dans le DTO Rust

**Files:**
- Modify: `src-tauri/src/commands.rs` (`StatusResultDto` ~ligne 52, `status_from` ~ligne 160, module `status_tests` ~ligne 1511)

- [ ] **Step 1: Écrire les tests qui échouent**

Ajouter dans le module `status_tests` de `src-tauri/src/commands.rs` (adapter les noms de champs à la capture de Task 9) :

```rust
    #[test]
    fn merge_fixture_sets_merge_in_progress() {
        let events = parse_events(include_str!("../tests/fixtures/status_merge.ndjson")).unwrap();
        let s = status_from(&events, std::path::Path::new(""));
        assert!(s.merge_in_progress);
    }

    #[test]
    fn staged_fixture_sets_staged_pending() {
        let events = parse_events(include_str!("../tests/fixtures/status_staged.ndjson")).unwrap();
        let s = status_from(&events, std::path::Path::new(""));
        assert!(s.staged_pending);
    }

    #[test]
    fn merge_and_staged_flags() {
        let sample = concat!(
            r#"{"tagName":"repositoryStatusRevision","data":{"branchName":"main","revisionLocalNumber":3,"revisionRemoteNumber":3,"isLocalAhead":false,"isRemoteAhead":false,"revisionMerged":"a3e42aeae4e3","revisionStaged":"b4f53bfbf5f4"}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let s = status_from(&parse_events(sample).unwrap(), std::path::Path::new(""));
        assert!(s.merge_in_progress);
        assert!(s.staged_pending);
    }

    #[test]
    fn zero_or_absent_merge_fields_are_false() {
        // All-zero hashes = no merge/staged state.
        let zeros = concat!(
            r#"{"tagName":"repositoryStatusRevision","data":{"branchName":"main","revisionLocalNumber":1,"revisionRemoteNumber":1,"isLocalAhead":false,"isRemoteAhead":false,"revisionMerged":"0000000000000000000000000000000000000000000000000000000000000000","revisionStaged":"0000000000000000000000000000000000000000000000000000000000000000"}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let s = status_from(&parse_events(zeros).unwrap(), std::path::Path::new(""));
        assert!(!s.merge_in_progress);
        assert!(!s.staged_pending);
        // Absent fields (older CLI, no merge) must default to false too.
        let absent = concat!(
            r#"{"tagName":"repositoryStatusRevision","data":{"branchName":"main","revisionLocalNumber":1,"revisionRemoteNumber":1,"isLocalAhead":false,"isRemoteAhead":false}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let s = status_from(&parse_events(absent).unwrap(), std::path::Path::new(""));
        assert!(!s.merge_in_progress);
        assert!(!s.staged_pending);
    }
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test --manifest-path src-tauri/Cargo.toml status_tests`
Expected: FAIL à la compilation — `no field 'merge_in_progress' on type StatusResultDto`.

- [ ] **Step 3: Implémenter**

1. Dans `StatusResultDto`, ajouter après `pub remote_authorized: bool,` :

```rust
    /// A merge is waiting for conflict resolution (revisionMerged* non-zero).
    pub merge_in_progress: bool,
    /// An interrupted commit/merge left a staged state (revisionStaged non-zero).
    pub staged_pending: bool,
```

2. Dans `status_from`, après la ligne `let remote_authorized = ...`, ajouter (adapter les noms de champs à la capture de Task 9) :

```rust
    // Merge/staged residual state (StatusBar chip). Field names pinned against
    // tests/fixtures/status_merge.ndjson + status_staged.ndjson; absent fields
    // (older CLI, no merge) default to false.
    let merge_in_progress = rev
        .and_then(|d| d.get("revisionMerged"))
        .and_then(|v| v.as_str())
        .map(|h| !zero_hash(h))
        .unwrap_or(false);
    let staged_pending = rev
        .and_then(|d| d.get("revisionStaged"))
        .and_then(|v| v.as_str())
        .map(|h| !zero_hash(h))
        .unwrap_or(false);
```

3. Mettre à jour le littéral de retour :

```rust
    StatusResultDto { branch, local_ahead, remote_ahead, revision_number, remote_available, remote_authorized, merge_in_progress, staged_pending, files }
```

(`zero_hash` existe déjà dans commands.rs, ~ligne 263.)

- [ ] **Step 4: Vérifier le PASS**

Run: `cargo test --manifest-path src-tauri/Cargo.toml status_tests`
Expected: `test result: ok` (les 4 nouveaux tests + les 4 existants passent).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(status): mergeInProgress + stagedPending flags in the status DTO"
```

### Task 11: `StatusResult` frontend + mock + précédence du chip (`statusChip.ts`)

**Files:**
- Create: `src/lib/statusChip.ts`
- Create: `src/lib/statusChip.test.ts`
- Modify: `src/lib/types.ts` (`StatusResult`)
- Modify: `src/lib/mock.ts` (`getStatus`)
- Modify: `src/lib/mockOps.test.ts`

- [ ] **Step 1: Écrire les tests qui échouent**

Créer `src/lib/statusChip.test.ts` :

```ts
import { describe, it, expect } from 'vitest'
import { chipFor } from './statusChip'
import type { StatusResult } from './types'

const base: StatusResult = {
  branch: 'main', localAhead: 0, remoteAhead: 0, revisionNumber: 1,
  remoteAvailable: true, remoteAuthorized: true,
  mergeInProgress: false, stagedPending: false, files: [],
}

describe('chipFor', () => {
  it('returns null with no status', () => {
    expect(chipFor(null)).toBeNull()
  })
  it('returns null when neither flag is set', () => {
    expect(chipFor(base)).toBeNull()
  })
  it('shows the staged chip when only stagedPending is set', () => {
    expect(chipFor({ ...base, stagedPending: true })).toEqual({ kind: 'staged' })
  })
  it('merge takes precedence over staged (a merge implies a staged state)', () => {
    expect(chipFor({ ...base, mergeInProgress: true, stagedPending: true })).toEqual({ kind: 'merge' })
  })
})
```

Ajouter à la fin de `src/lib/mockOps.test.ts` :

```ts
describe('mock status flags', () => {
  it('reports no merge and no staged state by default', async () => {
    const s = await mock.getStatus('C:/repos/flags')
    expect(s.mergeInProgress).toBe(false)
    expect(s.stagedPending).toBe(false)
  })
  it('reports mergeInProgress while a conflicting merge is open', async () => {
    await mock.mergeStart('C:/repos/flags', 'feature/loot')
    expect((await mock.getStatus('C:/repos/flags')).mergeInProgress).toBe(true)
    await mock.mergeAbort('C:/repos/flags')
    expect((await mock.getStatus('C:/repos/flags')).mergeInProgress).toBe(false)
  })
})
```

- [ ] **Step 2: Vérifier l'échec**

Run: `npx vitest run src/lib/statusChip.test.ts src/lib/mockOps.test.ts`
Expected: FAIL — `Cannot find module './statusChip'` + les flags absents du mock.

- [ ] **Step 3: Implémenter**

Dans `src/lib/types.ts`, ajouter à `StatusResult` après `remoteAuthorized: boolean` :

```ts
  /** A merge is waiting for conflict resolution — the Merge view can resume it. */
  mergeInProgress: boolean
  /** An interrupted commit/merge left a staged state; picked up by the next commit/merge. */
  stagedPending: boolean
```

Créer `src/lib/statusChip.ts` :

```ts
import type { StatusResult } from './types'

export type StatusChip = { kind: 'merge' } | { kind: 'staged' } | null

/**
 * Which StatusBar chip to show. Merge takes precedence: a merge implies a
 * staged state, so the staged chip is hidden while a merge is in progress.
 */
export function chipFor(status: StatusResult | null): StatusChip {
  if (!status) return null
  if (status.mergeInProgress) return { kind: 'merge' }
  if (status.stagedPending) return { kind: 'staged' }
  return null
}
```

Dans `src/lib/mock.ts`, remplacer le retour de `getStatus` par :

```ts
  async getStatus(repoPath: string) {
    await delay(250)
    const s = stateFor(repoPath)
    return {
      branch: s.branch, localAhead: s.localAhead, remoteAhead: s.remoteAhead,
      revisionNumber: 5, remoteAvailable: true, remoteAuthorized: true,
      // mergeStart opens conflicts; commit/abort clears them (see mergeConflictState).
      mergeInProgress: mergeConflictState.length > 0,
      // Dev lever for the informational chip:
      //   localStorage.setItem('loredesktop.mock.staged', '1')  (removeItem to clear)
      stagedPending: localStorage.getItem('loredesktop.mock.staged') === '1',
      files: [...s.files],
    } as StatusResult
  },
```

- [ ] **Step 4: Vérifier le PASS**

Run: `npx vitest run src/lib/statusChip.test.ts src/lib/mockOps.test.ts`
Expected: tout passe.

Run: `npm run check`
Expected: 0 errors (tous les producteurs de `StatusResult` fournissent les deux booléens).

- [ ] **Step 5: Commit**

```bash
git add src/lib/statusChip.ts src/lib/statusChip.test.ts src/lib/types.ts src/lib/mock.ts src/lib/mockOps.test.ts
git commit -m "feat(status): StatusResult flags, chip precedence helper, mock levers"
```

### Task 12: Chip dans la StatusBar

**Files:**
- Modify: `src/lib/StatusBar.svelte`

- [ ] **Step 1: Markup + wiring**

Dans `src/lib/StatusBar.svelte` :

1. Ajouter aux imports :

```ts
  import { setView } from './ui.svelte'
  import { chipFor } from './statusChip'
```

2. Ajouter le derived après `const expired = ...` :

```ts
  const chip = $derived(chipFor(repo.status))
```

3. Dans le markup, juste après le premier `</span>` (celui du `.item` principal) et avant `<span class="spacer"></span>`, insérer :

```svelte
  {#if chip?.kind === 'merge'}
    <button class="chip merge" onclick={() => setView('merge')} title="A merge is waiting for conflict resolution — click to resume it">
      <Icon name="branch" size={12} /> Merge in progress — resume
    </button>
  {:else if chip?.kind === 'staged'}
    <span class="chip" title="An interrupted commit or merge left a staged state; it will be picked up by the next commit or merge.">
      <Icon name="info" size={12} /> Staged state pending
    </span>
  {/if}
```

(Chip staged **informatif seulement** en v1 : pas de bouton, juste le tooltip. La sémantique d'un abandon sera validée contre le CLI réel avant d'exposer une action destructive.)

4. CSS, ajouter dans le `<style>` :

```css
  .chip { display: inline-flex; align-items: center; gap: 5px; font-size: 11px; padding: 1px 8px; border-radius: 999px; background: var(--panel); color: var(--text-muted); border: 1px solid var(--border); }
  .chip.merge { background: var(--warn-bg); color: var(--warn-text); border-color: transparent; cursor: pointer; }
```

- [ ] **Step 2: Vérifier**

Run: `npm run check && npx vitest run`
Expected: tout passe.

`npm run dev` + navigateur :
- Ouvrir la vue Merge (menu branche), prévisualiser `feature/loot` (2 conflits), démarrer le merge, revenir à Changes → chip ambre « Merge in progress — resume » dans la StatusBar ; clic → vue Merge.
- Console : `localStorage.setItem('loredesktop.mock.staged', '1')` + recharger → chip gris « Staged state pending » avec tooltip ; pendant un merge en cours, seul le chip merge apparaît (précédence). `localStorage.removeItem('loredesktop.mock.staged')` pour nettoyer.

- [ ] **Step 3: Commit**

```bash
git add src/lib/StatusBar.svelte
git commit -m "feat(status): merge/staged chip in the StatusBar (merge takes precedence)"
```

---

## Item 4 — Progression clone / sync / push + détection de blocage

### Task 13: Capture réelle du NDJSON de clone, sync et push

**Files:**
- Create: `src-tauri/tests/fixtures/clone_progress.ndjson`
- Modify: `src-tauri/tests/fixtures/README.md`

- [ ] **Step 1: Capturer un clone réel**

```powershell
$lore = "C:\Users\jimmy\bin\lore.exe"
$scratch = "C:\Users\jimmy\AppData\Local\Temp\p1-stream-capture"
New-Item -ItemType Directory -Force $scratch | Out-Null
& $lore clone lore://lore.example.com:41337/019f333af5e073d28bb117ad1596784a "$scratch\desktoptest1" --json |
  Out-File -Encoding utf8NoBOM "$scratch\clone.ndjson"
Get-Content "$scratch\clone.ndjson" | Select-Object -First 20
```

Expected: un flux NDJSON contenant (hypothèse slice B) des événements `repositoryCloneProgress`, terminé par `complete` status 0.

- [ ] **Step 2: Capturer un push et un sync réels**

```powershell
$repo = "C:\Users\jimmy\lore-test-repo"
Add-Content "$repo\README.md" "p1 stream capture"
& $lore stage . --scan --repository $repo
& $lore commit "p1 stream capture" --repository $repo
& $lore push --repository $repo --json | Out-File -Encoding utf8NoBOM "$scratch\push.ndjson"
# La copie clonée au Step 1 est maintenant en retard d'un commit — sync :
& $lore sync --repository "$scratch\desktoptest1" --json | Out-File -Encoding utf8NoBOM "$scratch\sync.ndjson"
Get-Content "$scratch\push.ndjson"
Get-Content "$scratch\sync.ndjson"
```

- [ ] **Step 3: Pinner l'encodage de progression + fixture committée**

Inspecter les trois captures :
- `tagName` exact des événements de progression du clone (hypothèse : `repositoryCloneProgress`) et ses champs (hypothèses : `done`/`total`, ou `current`/`total` ; unité octets ou fichiers ?).
- Existe-t-il des équivalents pour sync/push ? **S'il n'y en a pas, c'est prévu par le design : progression indéterminée pour ces opérations** — noter le fait, ne rien inventer.

Créer `src-tauri/tests/fixtures/clone_progress.ndjson` avec un extrait représentatif du clone : 2–3 lignes de progression (début, milieu, fin) + la ligne `complete` finale, copiées telles quelles depuis `$scratch\clone.ndjson`. **Si les tags/champs réels diffèrent des hypothèses, adapter en Tasks 14–15** les samples inline et `op_progress_from`.

Ajouter au README des fixtures :

```markdown

**`repositoryCloneProgress`** (clone ; fixture clone_progress.ndjson) : `done` /
`total` (u64). Unité constatée : <octets|fichiers — compléter à la capture>.
Sync/push : <événements constatés ou « aucun événement de progression → barre
indéterminée » — compléter à la capture>.
```

- [ ] **Step 4: Nettoyage + commit**

```powershell
Remove-Item -Recurse -Force "$scratch\desktoptest1"
```

```bash
git add src-tauri/tests/fixtures/clone_progress.ndjson src-tauri/tests/fixtures/README.md
git commit -m "test(fixtures): capture clone/sync/push streams with progress events"
```

### Task 14: `run_lore_streaming` + détection de blocage dans lore.rs

**Files:**
- Modify: `src-tauri/src/lore.rs`

- [ ] **Step 1: Écrire les tests qui échouent**

Ajouter dans le module `tests` de `src-tauri/src/lore.rs` :

```rust
    use std::time::Duration;

    #[test]
    fn streaming_relays_events_incrementally_in_order() {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        tx.send(r#"{"tagName":"repositoryCloneProgress","data":{"done":10,"total":100}}"#.into()).unwrap();
        tx.send(r#"{"tagName":"repositoryCloneProgress","data":{"done":100,"total":100}}"#.into()).unwrap();
        tx.send(r#"{"tagName":"complete","data":{"status":0}}"#.into()).unwrap();
        drop(tx);
        let mut seen: Vec<String> = Vec::new();
        let events = collect_streaming(&rx, Duration::from_millis(500), &mut |ev| seen.push(ev.tag_name.clone())).unwrap();
        assert_eq!(seen, ["repositoryCloneProgress", "repositoryCloneProgress", "complete"]);
        assert_eq!(events.len(), 3);
    }

    #[test]
    fn streaming_requires_a_complete_event() {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        tx.send(r#"{"tagName":"repositoryCloneProgress","data":{"done":10,"total":100}}"#.into()).unwrap();
        drop(tx); // stream ends without complete
        let err = collect_streaming(&rx, Duration::from_millis(500), &mut |_| {}).unwrap_err();
        assert!(err.contains("completion"), "err was {err}");
    }

    #[test]
    fn streaming_silence_is_a_stall_error() {
        let (_tx, rx) = std::sync::mpsc::channel::<String>();
        // Sender alive but silent (fake hung child) → stall, not disconnect.
        let err = collect_streaming(&rx, Duration::from_millis(50), &mut |_| {}).unwrap_err();
        assert!(err.contains("no progress"), "err was {err}");
    }

    #[test]
    fn streaming_error_event_fails_check() {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        tx.send(r#"{"tagName":"error","data":{"errorInner":"nope"}}"#.into()).unwrap();
        tx.send(r#"{"tagName":"complete","data":{"status":1}}"#.into()).unwrap();
        drop(tx);
        assert!(collect_streaming(&rx, Duration::from_millis(500), &mut |_| {}).is_err());
    }

    #[test]
    #[cfg(windows)]
    fn stalled_child_is_killed_promptly() {
        // A real child that prints nothing: the stall detector must kill it and
        // return well before its natural 30 s lifetime.
        let start = std::time::Instant::now();
        let err = run_streaming_cmd(
            "powershell",
            &["-NoProfile", "-Command", "Start-Sleep -Seconds 30"],
            Duration::from_millis(300),
            &mut |_| {},
        )
        .unwrap_err();
        assert!(err.contains("no progress"), "err was {err}");
        assert!(start.elapsed() < Duration::from_secs(10), "child was not killed promptly");
    }
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test --manifest-path src-tauri/Cargo.toml streaming`
Expected: FAIL à la compilation — `cannot find function 'collect_streaming'`.

- [ ] **Step 3: Implémenter le runner streaming**

Ajouter dans `src-tauri/src/lore.rs`, après `run_lore` (avant le module `tests`) :

```rust
/// Stall detector for the streaming runner: if the child emits NO line for this
/// long, it is considered hung, killed, and the operation errors. An operation
/// that keeps making progress is never killed — this replaces the flat 45 s cap
/// for clone/sync/push, which legitimately run for minutes on studio binaries.
pub const LORE_STALL_TIMEOUT: Duration = Duration::from_secs(60);

/// Run `lore <args> --json` streaming stdout line by line. Each NDJSON event is
/// (a) forwarded to `on_event` as it arrives and (b) collected for the final
/// result, validated by the same complete-event + `check_ok` rules as
/// `run_lore`. Modeled on the notifications sidecar (notifications.rs).
pub fn run_lore_streaming(
    args: &[&str],
    on_event: &mut dyn FnMut(&LoreEvent),
) -> Result<Vec<LoreEvent>, String> {
    let mut owned: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
    owned.push("--json".to_string());
    let owned_refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
    run_streaming_cmd("lore", &owned_refs, LORE_STALL_TIMEOUT, on_event)
}

/// Program-agnostic core of the streaming runner (testable with a fake child).
fn run_streaming_cmd(
    program: &str,
    args: &[&str],
    stall: Duration,
    on_event: &mut dyn FnMut(&LoreEvent),
) -> Result<Vec<LoreEvent>, String> {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let mut child = cmd.spawn().map_err(|e| format!("failed to launch {program}: {e}"))?;
    let stdout = child.stdout.take().ok_or_else(|| "no stdout pipe".to_string())?;
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        use std::io::BufRead;
        for line in std::io::BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if tx.send(line).is_err() {
                break; // receiver gone (stall kill) — stop reading
            }
        }
    });
    let result = collect_streaming(&rx, stall, on_event);
    if result.is_err() {
        let _ = child.kill(); // stall or bad stream — don't leave a zombie
    }
    let _ = child.wait();
    result
}

/// Drain the line channel with a stall timeout, parsing + relaying each event.
/// Channel disconnect = clean EOF; a silent-but-alive sender = a stalled child.
fn collect_streaming(
    rx: &std::sync::mpsc::Receiver<String>,
    stall: Duration,
    on_event: &mut dyn FnMut(&LoreEvent),
) -> Result<Vec<LoreEvent>, String> {
    let mut events: Vec<LoreEvent> = Vec::new();
    loop {
        match rx.recv_timeout(stall) {
            Ok(line) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if let Ok(ev) = serde_json::from_str::<LoreEvent>(line) {
                    on_event(&ev);
                    events.push(ev);
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                return Err(format!(
                    "lore made no progress for {} s — operation aborted",
                    stall.as_secs().max(1)
                ));
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    if !events.iter().any(|e| e.tag_name == "complete") {
        return Err("lore did not emit a completion event".to_string());
    }
    check_ok(&events)?;
    Ok(events)
}
```

Note : le message de stall utilise `stall.as_secs().max(1)` pour rester lisible avec les stalls sub-seconde des tests ; il contient toujours « no progress ».

- [ ] **Step 4: Vérifier le PASS**

Run: `cargo test --manifest-path src-tauri/Cargo.toml streaming && cargo test --manifest-path src-tauri/Cargo.toml stalled`
Expected: `test result: ok` — les 5 tests passent (le test kill prend ~0.5 s, pas 30 s).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lore.rs
git commit -m "feat(progress): streaming lore runner with 60s stall detection"
```

### Task 15: Relais `lore://op-progress` + bascule de clone/sync/push

**Files:**
- Modify: `src-tauri/src/commands.rs` (imports ligne 1, `lore_clone` ~ligne 512, `lore_push` ~ligne 654, `lore_sync` ~ligne 755)

- [ ] **Step 1: Écrire le test qui échoue**

Ajouter dans `src-tauri/src/commands.rs`, après le module `clone_tests` :

```rust
#[cfg(test)]
mod op_progress_tests {
    use super::*;
    use crate::lore::parse_events;

    #[test]
    fn maps_clone_progress_fixture() {
        let events = parse_events(include_str!("../tests/fixtures/clone_progress.ndjson")).unwrap();
        let ticks: Vec<_> = events.iter().filter_map(op_progress_from).collect();
        assert!(!ticks.is_empty(), "the captured fixture must yield progress ticks");
    }

    #[test]
    fn maps_done_total_and_ignores_other_events() {
        let sample = concat!(
            r#"{"tagName":"repositoryCloneProgress","data":{"done":512,"total":2048}}"#, "\n",
            r#"{"tagName":"repositoryStatusRevision","data":{"branchName":"main"}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let events = parse_events(sample).unwrap();
        let ticks: Vec<_> = events.iter().filter_map(op_progress_from).collect();
        assert_eq!(ticks, vec![(512, Some(2048))]);
    }

    #[test]
    fn progress_without_total_is_indeterminate() {
        let sample = concat!(
            r#"{"tagName":"repositorySyncProgress","data":{"done":3}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let events = parse_events(sample).unwrap();
        let ticks: Vec<_> = events.iter().filter_map(op_progress_from).collect();
        assert_eq!(ticks, vec![(3, None)]);
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test --manifest-path src-tauri/Cargo.toml op_progress`
Expected: FAIL à la compilation — `cannot find function 'op_progress_from'`.

- [ ] **Step 3: Implémenter le relais et basculer les trois commandes**

1. Ligne 1 de `src-tauri/src/commands.rs`, remplacer l'import par :

```rust
use crate::lore::{events_with_tag, run_lore, run_lore_streaming, LoreEvent};
```

2. Ajouter au-dessus de `lore_clone` :

```rust
/// Wire payload of `lore://op-progress`. The `op_id` is generated by the
/// FRONTEND and passed through the command, so the webview listener can filter
/// on it — this distinguishes simultaneous operations (e.g. a sync while a
/// clone runs) without an extra round-trip.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpProgressPayload {
    pub op_id: String,
    pub kind: &'static str, // "clone" | "sync" | "push"
    pub done: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<&'static str>, // "bytes" | "files"
}

/// Map a progress event to `(done, total)`. Tag + field names pinned against
/// tests/fixtures/clone_progress.ndjson (e.g. `repositoryCloneProgress`); any
/// `…Progress` event with a `done`/`current` count qualifies, so the sync/push
/// equivalents discovered at capture time are relayed too.
fn op_progress_from(ev: &LoreEvent) -> Option<(u64, Option<u64>)> {
    if !ev.tag_name.ends_with("Progress") {
        return None;
    }
    let d = &ev.data;
    let done = d.get("done").or_else(|| d.get("current")).and_then(|v| v.as_u64())?;
    let total = d.get("total").and_then(|v| v.as_u64());
    Some((done, total))
}

/// Unit of the progress counts, per the captured fixtures: the clone stream
/// counts bytes. Adjust here if the capture shows file counts instead.
const OP_PROGRESS_UNIT: Option<&'static str> = Some("bytes");

/// Run a long lore operation on the streaming runner, relaying every progress
/// event to the webview as `lore://op-progress`. Stall (60 s of silence) kills
/// the child and surfaces the same error toast as any failed operation.
fn run_lore_op(
    app: &tauri::AppHandle,
    kind: &'static str,
    op_id: &str,
    args: &[&str],
) -> Result<Vec<LoreEvent>, String> {
    use tauri::Emitter;
    let mut on_event = |ev: &LoreEvent| {
        if let Some((done, total)) = op_progress_from(ev) {
            let _ = app.emit(
                "lore://op-progress",
                OpProgressPayload { op_id: op_id.to_string(), kind, done, total, unit: OP_PROGRESS_UNIT },
            );
        }
    };
    run_lore_streaming(args, &mut on_event)
}
```

3. Remplacer `lore_clone` par :

```rust
/// Clone `<server_url>/<repo_id>` into `<dest_parent>/<repo_name>` and return
/// the created path. Streaming runner: progress is relayed as
/// `lore://op-progress` (filtered by the frontend-generated `op_id`) and a
/// stalled transfer errors after 60 s of silence — a long but advancing clone
/// is never killed (the old flat 45 s cap made big clones fail).
#[tauri::command]
pub async fn lore_clone(
    app: tauri::AppHandle,
    server_url: String,
    repo_id: String,
    repo_name: String,
    dest_parent: String,
    op_id: String,
) -> Result<String, String> {
    blocking(move || {
        let (url, path) = build_clone_args(&server_url, &repo_id, &repo_name, &dest_parent);
        run_lore_op(&app, "clone", &op_id, &["clone", &url, &path])?;
        Ok(path)
    })
    .await
}
```

4. Remplacer `lore_push` par :

```rust
#[tauri::command]
pub async fn lore_push(app: tauri::AppHandle, repo_path: String, op_id: String) -> Result<(), String> {
    blocking(move || {
        run_lore_op(&app, "push", &op_id, &["push", "--repository", &repo_path])?;
        Ok(())
    })
    .await
}
```

5. Remplacer `lore_sync` par (garder le commentaire non-destructif existant) :

```rust
/// Plain `lore sync` — pulls/merges the remote into the local branch
/// non-destructively (NO `--reset`, which would discard local modifications).
#[tauri::command]
pub async fn lore_sync(app: tauri::AppHandle, repo_path: String, op_id: String) -> Result<(), String> {
    blocking(move || {
        run_lore_op(&app, "sync", &op_id, &["sync", "--repository", &repo_path])?;
        Ok(())
    })
    .await
}
```

(Toutes les autres commandes — status, diff, lock… — gardent `run_lore` et son plafond de 45 s, adapté aux opérations courtes. Les noms de commande ne changent pas → rien à toucher dans lib.rs.)

- [ ] **Step 4: Vérifier le PASS**

Run: `cargo test --manifest-path src-tauri/Cargo.toml op_progress && cargo test --manifest-path src-tauri/Cargo.toml`
Expected: `test result: ok` sur la suite complète.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(progress): clone/sync/push on the streaming runner with op-progress relay"
```

### Task 16: Contrat frontend `OpProgress` + écoute filtrée + ticks du mock

**Files:**
- Modify: `src/lib/types.ts`
- Modify: `src/lib/tauri.ts`
- Modify: `src/lib/mock.ts`
- Modify: `src/lib/mockOps.test.ts`

- [ ] **Step 1: Écrire les tests qui échouent**

Ajouter à la fin de `src/lib/mockOps.test.ts` :

```ts
import type { OpProgress } from './types'

describe('mock op progress', () => {
  it('clone reports increasing ticks and finishes at total', async () => {
    const ticks: OpProgress[] = []
    await mock.cloneRepo('lore://x', 'id1', 'game', 'C:/repos', (p) => ticks.push(p))
    expect(ticks.length).toBeGreaterThan(3)
    for (let i = 1; i < ticks.length; i++) expect(ticks[i].done).toBeGreaterThanOrEqual(ticks[i - 1].done)
    const last = ticks[ticks.length - 1]
    expect(last.total).toBeGreaterThan(0)
    expect(last.done).toBe(last.total)
  })
  it('sync and push tick too, and still mutate the repo state', async () => {
    const syncTicks: OpProgress[] = []
    await mock.sync('C:/repos/prog', (p) => syncTicks.push(p))
    expect(syncTicks.length).toBeGreaterThan(2)
    expect((await mock.getStatus('C:/repos/prog')).remoteAhead).toBe(0)
    const pushTicks: OpProgress[] = []
    await mock.push('C:/repos/prog', (p) => pushTicks.push(p))
    expect(pushTicks.length).toBeGreaterThan(2)
    expect((await mock.getStatus('C:/repos/prog')).localAhead).toBe(0)
  })
  it('progress callbacks stay optional', async () => {
    await expect(mock.sync('C:/repos/noprog')).resolves.toBeUndefined()
  })
})
```

(Déplacer la ligne `import type { OpProgress } from './types'` en tête de fichier, avec les autres imports.)

- [ ] **Step 2: Vérifier l'échec**

Run: `npx vitest run src/lib/mockOps.test.ts`
Expected: FAIL — `OpProgress` n'existe pas / les mocks n'appellent pas `onProgress`.

- [ ] **Step 3: Implémenter le contrat**

Dans `src/lib/types.ts` :

1. Ajouter après l'interface `StatusResult` :

```ts
/** One progress tick of a long operation (clone/sync/push). No `total` = indeterminate. */
export interface OpProgress {
  done: number
  total?: number
  unit?: 'bytes' | 'files'
}
```

2. Dans `LoreApi`, remplacer les trois signatures :

```ts
  /** Clone <serverUrl>/<repoId> into <destParent>/<repoName>; returns the created path. Progress ticks stream via onProgress. */
  cloneRepo(serverUrl: string, repoId: string, repoName: string, destParent: string, onProgress?: (p: OpProgress) => void): Promise<string>
```

```ts
  push(repoPath: string, onProgress?: (p: OpProgress) => void): Promise<void>
  sync(repoPath: string, onProgress?: (p: OpProgress) => void): Promise<void>
```

Dans `src/lib/tauri.ts` :

1. Ajouter `OpProgress` à l'import de types.
2. Ajouter au-dessus de `export const tauriApi` :

```ts
type WireProgress = { opId: string; kind: string; done: number; total?: number; unit?: 'bytes' | 'files' }

/**
 * Invoke a long command with a frontend-generated opId, listening to
 * `lore://op-progress` filtered on that id for the call's duration. The id is
 * what distinguishes simultaneous operations (e.g. a sync during a clone).
 */
async function invokeWithProgress<T>(
  cmd: string,
  args: Record<string, unknown>,
  onProgress?: (p: OpProgress) => void,
): Promise<T> {
  const opId = crypto.randomUUID()
  let unlisten: (() => void) | null = null
  if (onProgress) {
    unlisten = await listen<WireProgress>('lore://op-progress', (e) => {
      if (e.payload.opId === opId)
        onProgress({ done: e.payload.done, total: e.payload.total, unit: e.payload.unit })
    })
  }
  try {
    return await invoke<T>(cmd, { ...args, opId })
  } finally {
    unlisten?.()
  }
}
```

3. Remplacer les trois entrées :

```ts
  cloneRepo: (serverUrl, repoId, repoName, destParent, onProgress) =>
    invokeWithProgress<string>('lore_clone', { serverUrl, repoId, repoName, destParent }, onProgress),
  push: (repoPath, onProgress) => invokeWithProgress<void>('lore_push', { repoPath }, onProgress),
  sync: (repoPath, onProgress) => invokeWithProgress<void>('lore_sync', { repoPath }, onProgress),
```

Dans `src/lib/mock.ts` : ajouter `OpProgress` à la liste de l'import de types existant (ligne 2), puis remplacer `cloneRepo`, `push` et `sync` par des versions à ticks (browser dev) :

```ts
  async cloneRepo(_serverUrl: string, _repoId: string, repoName: string, destParent: string, onProgress?: (p: OpProgress) => void) {
    // Simulated determinate transfer so the clone progress bar lives in dev.
    const total = 48 * 1024 * 1024
    for (let i = 1; i <= 12; i++) {
      await delay(90)
      onProgress?.({ done: Math.round((total * i) / 12), total, unit: 'bytes' })
    }
    return `${destParent}/${repoName}`
  },
```

```ts
  async push(repoPath: string, onProgress?: (p: OpProgress) => void) {
    for (let i = 1; i <= 6; i++) {
      await delay(100)
      onProgress?.({ done: i, total: 6, unit: 'files' })
    }
    stateFor(repoPath).localAhead = 0
  },
  async sync(repoPath: string, onProgress?: (p: OpProgress) => void) {
    for (let i = 1; i <= 6; i++) {
      await delay(80)
      onProgress?.({ done: i, total: 6, unit: 'files' })
    }
    stateFor(repoPath).remoteAhead = 0
  },
```

- [ ] **Step 4: Vérifier le PASS**

Run: `npx vitest run src/lib/mockOps.test.ts && npm run check`
Expected: tout passe, 0 erreurs de types.

- [ ] **Step 5: Commit**

```bash
git add src/lib/types.ts src/lib/tauri.ts src/lib/mock.ts src/lib/mockOps.test.ts
git commit -m "feat(progress): OpProgress contract, filtered op-progress listener, mock ticks"
```

### Task 17: Store de progression + barres UI (TitleBar, RepoPicker, RepoSwitcher)

**Files:**
- Create: `src/lib/progress.ts`
- Create: `src/lib/progress.test.ts`
- Create: `src/lib/opProgress.svelte.ts`
- Modify: `src/lib/repo.svelte.ts` (sync/push)
- Modify: `src/lib/repoActions.ts` (clone)
- Modify: `src/lib/TitleBar.svelte`
- Modify: `src/lib/RepoPicker.svelte`
- Modify: `src/lib/RepoSwitcher.svelte`

- [ ] **Step 1: Écrire le test qui échoue (helper pur)**

Créer `src/lib/progress.test.ts` :

```ts
import { describe, it, expect } from 'vitest'
import { pct, cloneLabel } from './progress'

describe('pct', () => {
  it('returns a clamped 0-100 percentage when a total exists', () => {
    expect(pct({ done: 512, total: 2048 })).toBe(25)
    expect(pct({ done: 3000, total: 2048 })).toBe(100)
  })
  it('returns null for indeterminate progress', () => {
    expect(pct(null)).toBeNull()
    expect(pct({ done: 10 })).toBeNull()
    expect(pct({ done: 10, total: 0 })).toBeNull()
  })
})

describe('cloneLabel', () => {
  it('appends the percentage when known', () => {
    expect(cloneLabel(42)).toBe('Cloning… 42%')
  })
  it('stays plain when indeterminate', () => {
    expect(cloneLabel(null)).toBe('Cloning…')
  })
})
```

- [ ] **Step 2: Vérifier l'échec**

Run: `npx vitest run src/lib/progress.test.ts`
Expected: FAIL — `Cannot find module './progress'`.

- [ ] **Step 3: Implémenter le helper et le store**

Créer `src/lib/progress.ts` :

```ts
import type { OpProgress } from './types'

/** 0-100 (clamped) when a total is known, null for indeterminate progress. */
export function pct(p: OpProgress | null): number | null {
  if (!p || !p.total || p.total <= 0) return null
  return Math.min(100, Math.round((p.done / p.total) * 100))
}

/** Button label for an in-flight clone. */
export function cloneLabel(percent: number | null): string {
  return percent === null ? 'Cloning…' : `Cloning… ${percent}%`
}
```

Créer `src/lib/opProgress.svelte.ts` :

```ts
import type { OpProgress } from './types'

// Live progress of the long-running operations, one slot per kind (an opId
// already isolates concurrent ops backend-side; the UI shows one bar per
// button/flow, so per-kind slots are enough). Null = idle.
export const opProgress = $state({
  clone: null as OpProgress | null,
  sync: null as OpProgress | null,
  push: null as OpProgress | null,
})
```

- [ ] **Step 4: Vérifier le PASS du helper**

Run: `npx vitest run src/lib/progress.test.ts`
Expected: `4 passed`.

- [ ] **Step 5: Wiring sync/push (repo.svelte.ts) et clone (repoActions.ts)**

Dans `src/lib/repo.svelte.ts` :

1. Ajouter l'import : `import { opProgress } from './opProgress.svelte'`
2. Remplacer `export const sync = ...` par :

```ts
export const sync = () => act('sync', async (p) => {
  try { await api.sync(p, (prog) => { opProgress.sync = prog }) }
  finally { opProgress.sync = null }
})
```

3. Remplacer le corps de `export const push = ...` par :

```ts
export const push = () => act('push', async (p) => {
  let candidates: string[] = []
  try { candidates = await api.pushedLockFiles(p) } catch { /* best-effort; never block the push */ }
  try { await api.push(p, (prog) => { opProgress.push = prog }) }
  finally { opProgress.push = null }
  if (candidates.length) {
    const n = candidates.length
    toastAction(`${n} locked file${n > 1 ? 's' : ''} pushed`, {
      label: 'Release locks',
      run: () => releaseLocks(candidates),
    })
  }
})
```

Dans `src/lib/repoActions.ts` :

1. Ajouter l'import : `import { opProgress } from './opProgress.svelte'`
2. Remplacer le corps de `cloneServerRepo` par :

```ts
export async function cloneServerRepo(entry: RepoEntry): Promise<boolean> {
  const parent = await api.pickFolder()
  if (!parent) return false // cancelled
  try {
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

- [ ] **Step 6: Barres fines dans la TitleBar (sync/push)**

Dans `src/lib/TitleBar.svelte` :

1. Ajouter aux imports :

```ts
  import { opProgress } from './opProgress.svelte'
  import { pct } from './progress'
```

2. Remplacer les deux boutons d'action par :

```svelte
    <button class="action" onclick={sync} disabled={!!repo.busy || noRemote} title={noRemote ? 'Server unreachable — sync is unavailable' : 'Sync'}>
      <Icon name="sync" size={16} />
      <span>{repo.busy === 'sync' ? 'Syncing…' : 'Sync'}</span>
      {#if repo.status?.remoteAhead}<span class="count">{repo.status.remoteAhead}</span>{/if}
      {#if repo.busy === 'sync'}
        <span class="opbar" class:indet={pct(opProgress.sync) === null} style="width: {pct(opProgress.sync) ?? 40}%"></span>
      {/if}
    </button>
    <button class="action accent" onclick={push} disabled={!!repo.busy || noRemote || (repo.status?.localAhead ?? 0) === 0} title={noRemote ? 'Server unreachable — push is unavailable' : 'Push'}>
      <Icon name="push" size={16} />
      <span>{repo.busy === 'push' ? 'Pushing…' : 'Push'}</span>
      {#if repo.status?.localAhead}<span class="count on">{repo.status.localAhead}</span>{/if}
      {#if repo.busy === 'push'}
        <span class="opbar" class:indet={pct(opProgress.push) === null} style="width: {pct(opProgress.push) ?? 40}%"></span>
      {/if}
    </button>
```

3. CSS, ajouter dans le `<style>` :

```css
  .action { position: relative; overflow: hidden; }
  .opbar { position: absolute; left: 0; bottom: 0; height: 2px; background: var(--accent); transition: width .25s ease; }
  .action.accent .opbar { background: var(--on-accent); opacity: .85; }
  .opbar.indet { animation: opslide 1.1s linear infinite; }
  @keyframes opslide { from { transform: translateX(-100%); } to { transform: translateX(350%); } }
```

(La règle `.action` existante `display: flex; align-items: center; gap: 6px; height: 32px;` reste — fusionner `position: relative; overflow: hidden;` dedans plutôt que de dupliquer le sélecteur.)

- [ ] **Step 7: Progression du clone dans RepoPicker**

Dans `src/lib/RepoPicker.svelte` :

1. Ajouter aux imports :

```ts
  import { opProgress } from './opProgress.svelte'
  import { pct, cloneLabel } from './progress'
```

2. Remplacer le `<li>` de la liste des repos par :

```svelte
      <li>
        <span class="ico"><Icon name="folder" size={16} /></span>
        <div class="meta"><strong>{r.name}</strong><p class="muted small mono">{r.id.slice(0, 12)}…</p></div>
        <span class="spacer"></span>
        <button onclick={() => cloneRepo(r)} disabled={busy === `clone:${r.id}`}>
          {busy === `clone:${r.id}` ? cloneLabel(pct(opProgress.clone)) : 'Clone…'}
        </button>
        {#if busy === `clone:${r.id}`}
          <span class="clonebar" class:indet={pct(opProgress.clone) === null} style="width: {pct(opProgress.clone) ?? 40}%"></span>
        {/if}
      </li>
```

3. CSS : modifier la règle `.repos li` (ajouter `position: relative;`) et ajouter :

```css
  .repos li { display: flex; align-items: center; gap: 10px; padding: 10px 4px; border-bottom: 1px solid var(--border); position: relative; overflow: hidden; }
  .clonebar { position: absolute; left: 0; bottom: 0; height: 2px; background: var(--accent); transition: width .25s ease; }
  .clonebar.indet { animation: pickerslide 1.1s linear infinite; }
  @keyframes pickerslide { from { transform: translateX(-100%); } to { transform: translateX(350%); } }
```

- [ ] **Step 8: Progression du clone dans RepoSwitcher**

Dans `src/lib/RepoSwitcher.svelte` :

1. Ajouter aux imports :

```ts
  import { opProgress } from './opProgress.svelte'
  import { pct, cloneLabel } from './progress'
```

2. Dans le mode `clone`, remplacer la ligne `.rp` du bouton par :

```svelte
              <span class="rp">{busy === `clone:${r.id}` ? cloneLabel(pct(opProgress.clone)) : r.id.slice(0, 12) + '…'}</span>
```

3. Toujours dans le mode `clone`, ajouter la barre juste avant la fermeture `</div>` du `.rowwrap` :

```svelte
          {#if busy === `clone:${r.id}`}
            <span class="clonebar" class:indet={pct(opProgress.clone) === null} style="width: {pct(opProgress.clone) ?? 40}%"></span>
          {/if}
```

4. CSS : modifier `.rowwrap` (elle a déjà `position: relative;`, ajouter `overflow: hidden;`) et ajouter :

```css
  .rowwrap { position: relative; overflow: hidden; }
  .clonebar { position: absolute; left: 0; bottom: 0; height: 2px; background: var(--accent); transition: width .25s ease; }
  .clonebar.indet { animation: switcherslide 1.1s linear infinite; }
  @keyframes switcherslide { from { transform: translateX(-100%); } to { transform: translateX(350%); } }
```

- [ ] **Step 9: Vérifier (suites + dev navigateur)**

Run: `npm run check && npx vitest run`
Expected: tout passe.

`npm run dev` + navigateur :
- Sync : la barre fine avance sous le bouton pendant ~0.5 s (6 ticks).
- Push (après un commit) : idem sous le bouton Push.
- RepoSwitcher → Add → Clone repository… → cloner un repo : le libellé passe par « Cloning… 8% … 100% » avec la barre qui avance (~1.1 s).
- RepoPicker (déconnecter le repo courant via le switcher ×) : même comportement sur le bouton Clone….

- [ ] **Step 10: Commit**

```bash
git add src/lib/progress.ts src/lib/progress.test.ts src/lib/opProgress.svelte.ts src/lib/repo.svelte.ts src/lib/repoActions.ts src/lib/TitleBar.svelte src/lib/RepoPicker.svelte src/lib/RepoSwitcher.svelte
git commit -m "feat(progress): progress bars for clone/sync/push in the UI"
```

---

### Task 18: Vérification finale

**Files:** aucun (vérification seulement ; fixes éventuels committés au fil de l'eau).

- [ ] **Step 1: Suites complètes**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
npx vitest run
npm run check
```

Expected: 100 % PASS, 0 erreur svelte-check/tsc.

- [ ] **Step 2: Vérification réelle (skill verify — app Tauri + repo de test)**

Lancer l'app réelle (`npx tauri dev`), ouvrir `C:\Users\jimmy\lore-test-repo`, et vérifier point par point (liste de la spec) :

1. **Delta réel** : modifier un asset du repo de test (ex. agrandir `README.md`), rafraîchir → le delta signé apparaît en fin de ligne quelques instants après le status ; FilePreview affiche « old → new ». Supprimer un fichier → l'ancienne taille seule.
2. **Section verrouillée** : poser un lock au CLI avec un second compte (ou vérifier avec un lock existant d'un tiers) : `lore lock acquire <abs path> --repository <repo>` → le fichier modifié apparaît dans la section ambre, sans checkbox, exclu du commit ; le bandeau FilePreview s'affiche ; la section apparaît/disparaît en live via les notifications.
3. **Chip merge** : rejouer le scénario de la Task 9 (branche conflictuelle + `merge start`) → chip « Merge in progress — resume » dans la StatusBar, clic → vue Merge ; aborter → chip disparaît. État stagé (`lore stage .` sans commit) → chip « Staged state pending » informatif.
4. **Clone streaming** : cloner `desktoptest1` depuis l'app → barre de progression qui avance, et **aucun échec à 45 s** (chronométrer si le clone est long ; au besoin, brider la bande passante pour dépasser 45 s). Sync/push : barre fine (déterminée ou indéterminée selon les événements découverts en Task 13). Débrancher le réseau en plein clone → erreur « no progress » après ~60 s, pas de process `lore` zombie dans le Gestionnaire des tâches.

- [ ] **Step 3: Nettoyage éventuel du repo de test**

Retirer les locks/branches de test posés au Step 2 (`lore lock release …`, `lore branch archive …`).

---

## Self-review (fait à l'écriture du plan)

- **Couverture spec** : Item 1 → Tasks 1–5 (batch unique, chemins absolus, fire-and-forget, silence total sur échec, delta neutre/nul/inconnu, delete = ancienne taille seule, FilePreview old → new). Item 2 → Tasks 6–8 (partition stricte, pas de checkbox, exclude par construction, compteurs commit + NavRail = committables, en-tête avec compte, filtre sur les deux groupes, menu contextuel conservé, Discard conservé, bandeau FilePreview, ligne Lock du bas conservée, vue Merge intacte). Item 3 → Tasks 9–12 (capture wire, défauts false, précédence merge > staged, chip staged informatif, merge cliquable → vue merge, staged masqué pendant merge, mock à jour). Item 4 → Tasks 13–17 (capture progression, runner streaming sur le modèle notifications, stall 60 s + kill, complete requis + check_ok, `lore://op-progress` `{opId, kind, done, total?, unit?}`, opId généré frontend, seuls clone/sync/push basculent, barre clone dans RepoPicker ET RepoSwitcher, barres fines TitleBar, indéterminé si pas d'événement, pas d'annulation v1, mock à ticks, même surface d'erreur toast). Tests spec : fixtures Rust (a)(b)(c) dont kill effectif (test `stalled_child_is_killed_promptly`) ; vitest partition/compteur/filtre, delta signé, précédence chip, fusion oldSize (fichiers disparus ignorés), progression mock. Hors périmètre respecté (pas de force release, pas d'« include anyway », pas d'action staged, pas d'annulation).
- **Placeholders** : les seules « adaptations » sont les étapes de capture (Tasks 1, 9, 13) exigées par la spec elle-même ; chaque hypothèse wire est nommée, avec le point d'adaptation exact (fonction + test à modifier). Aucun TBD/TODO, aucun « similar to Task N » : le code est complet dans chaque step.
- **Cohérence des types** : `fileSizes` (Task 3) ↔ `lore_file_sizes` (Task 2, retour `HashMap<String, u64>` ↔ `Record<string, number>`) ; `mergeInProgress`/`stagedPending` camelCase via `#[serde(rename_all = "camelCase")]` (Task 10 ↔ Task 11) ; `OpProgress { done, total?, unit? }` (Task 16) ↔ `OpProgressPayload` sérialisé camelCase avec `skip_serializing_if` sur les options (Task 15) ; `partitionByLock`/`filterByQuery` utilisés à l'identique dans Changes et NavRail (Tasks 6–7) ; `pct`/`cloneLabel` partagés entre TitleBar/RepoPicker/RepoSwitcher (Task 17) ; `fmtSize`/`formatDelta` partagés entre Changes et FilePreview (Tasks 4–5).

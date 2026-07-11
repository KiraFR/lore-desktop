# Lore Desktop — Lot P3 « récupération » Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Livrer les 3 items du lot P3 (spec `docs/superpowers/specs/2026-07-11-lore-desktop-p3-recovery-design.md`) : garde anti-race de `refreshHistory`, « Locate moved project » (détection des repos manquants + relocalisation via `repository update-path`), et sync-to-revision (« time travel ») depuis History avec chip `behind` et retour au latest.

**Architecture:** Backend Rust : deux commandes triviales (`os_path_exists`, `lore_update_path`) + `lore_sync_to` sur le runner streaming existant (`run_lore_op`, kind "sync" → progression déjà câblée). Frontend : jetons anti-race dans `repo.svelte.ts` (pattern `sizesToken` existant), helper pur `missingRepos`, état `missing` + flux Locate dans RepoSwitcher/RepoPicker/App, extension de `statusChip` (kind `behind`, précédence merge > staged > behind), bouton « Sync to this revision… » dans le détail de commit History, garde Commit désactivé quand behind. Mock : leviers localStorage (repo manquant, état behind).

**Tech Stack:** Rust (std::path, serde_json), Tauri v2, Svelte 5 runes, TypeScript, vitest, cargo test.

---

## Contexte & conventions (à lire avant toute tâche)

- **Repo de test réel** : `C:\Users\jimmy\lore-test-repo` (branche `feature/test`), CLI `C:\Users\jimmy\bin\lore.exe`, serveur `lore://lore.example.com:41337`, repo id `019f333af5e073d28bb117ad1596784a` (`desktoptest1`). Ne JAMAIS déplacer le repo de test lui-même — les scénarios destructifs passent par des clones temporaires dans `$env:TEMP`.
- **Captures** : NDJSON réels dans `src-tauri/tests/fixtures/` + documentation dans son `README.md`. Chaque tâche de capture : (1) capturer, (2) inspecter, (3) adapter le code/tests des tâches suivantes si les noms wire diffèrent, (4) documenter. Constat négatif (commande absente/sémantique différente) = résultat valide → appliquer le repli prévu et le documenter.
- **Suites** : `cargo test --manifest-path src-tauri/Cargo.toml` (baseline 98), `npx vitest run` (baseline 119), `npm run check` (0 erreur 0 warning). Chaque tâche rapporte les comptes réels.
- **`.svelte.ts` non testable en vitest** (pas de plugin svelte — `$state is not defined`) : la logique testable vit dans des modules purs `.ts` ; le wiring des stores se vérifie en navigateur (pattern A/B des lots P1/P2).
- **Chemins absolus** pour toute commande lore file-scoped (gotcha cwd).
- Ordre imposé : Item A (anti-race) → Item B (Locate) → Item C (time travel) → vérification finale.

## Carte des fichiers

- Modify: `src/lib/repo.svelte.ts` (jetons history), `src/lib/types.ts`, `src/lib/tauri.ts`, `src/lib/mock.ts`, `src/lib/repoList.ts` (ou nouveau `repoHealth.ts`), `src/lib/RepoSwitcher.svelte`, `src/lib/App.svelte`, `src/lib/statusChip.ts` (+ test), `src/lib/StatusBar.svelte`, `src/lib/History.svelte`, `src/lib/Changes.svelte` (garde Commit)
- Create: `src/lib/repoHealth.ts` (+ test), `src/lib/mock.test.ts` (extensions)
- Modify: `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`, `src-tauri/tests/fixtures/README.md`
- Create: `src-tauri/tests/fixtures/update_path.ndjson` (si NDJSON émis), `src-tauri/tests/fixtures/status_behind.ndjson`

---

# Item A — Garde anti-race `refreshHistory` (Task 1)

### Task 1: Jetons anti-race sur `refreshHistory` / `loadMoreHistory`

**Files:**
- Modify: `src/lib/repo.svelte.ts:35-61`

- [ ] **Step 1: Appliquer le pattern jeton (même idiome que `sizesToken`, présent plus bas dans le fichier)**

Remplacer `refreshHistory` et `loadMoreHistory` (lignes 35-61 — si les lignes ont bougé, chercher les fonctions par nom) par :

```ts
let historyToken = 0

export async function refreshHistory(silent = false) {
  const path = session.config.currentRepo
  const token = ++historyToken
  if (!path) {
    Object.assign(history, { commits: [], cursor: undefined, selectedId: null, loaded: false, repoPath: null })
    return
  }
  if (history.repoPath !== path) {
    // New repo → drop the stale history so the loading state shows for the first load.
    Object.assign(history, { commits: [], cursor: undefined, selectedId: null, loaded: false, repoPath: path })
  }
  try {
    const page = await api.getHistory(path, HISTORY_PAGE)
    // A newer call (or a repo switch) superseded this one — drop the stale page.
    if (token !== historyToken || session.config.currentRepo !== path) return
    history.commits = page.commits
    history.cursor = page.nextCursor
    if (page.commits.length && (history.selectedId === null || !page.commits.some((c) => c.id === history.selectedId)))
      history.selectedId = page.commits[0].id
    history.loaded = true
  } catch (e) { if (!silent) toastError("Couldn't load history", e) }
}

export async function loadMoreHistory() {
  const path = session.config.currentRepo
  if (!path || !history.cursor) return
  const token = historyToken
  const page = await api.getHistory(path, HISTORY_PAGE, history.cursor)
  // A refresh/switch happened while paging — the appended page would be stale.
  if (token !== historyToken || session.config.currentRepo !== path) return
  history.commits = [...history.commits, ...page.commits]
  history.cursor = page.nextCursor
}
```

(Note : `loadMoreHistory` lit le jeton SANS l'incrémenter — un load-more ne doit pas invalider un refresh concurrent, c'est l'inverse qui prime.)

- [ ] **Step 2: Typecheck + suite**

Run: `npm run check && npx vitest run`
Expected: 0 erreur, 119 tests verts (aucun test ne bouge — wiring non testable en vitest, cf. conventions).

- [ ] **Step 3: Vérification navigateur A/B (mock)**

`npm run dev` + navigateur. Le mock `getHistory` a un délai (~300 ms) : dans la console, enchaîner un switch rapide de repo pendant le vol :

```js
// A: reproduire la course SANS le fix est déjà impossible (fix appliqué) — on
// vérifie le comportement final : switcher deux fois vite, l'history affichée
// appartient TOUJOURS au repo final.
```

Via le RepoSwitcher : ouvrir le repo A, aller dans History (commits chargés), switcher vers le repo B et IMMÉDIATEMENT revenir sur History → après stabilisation, les commits affichés sont ceux de B (`history.repoPath === currentRepo` ; en mock les listes diffèrent par le seed). Répéter 3-4 switchs rapides A↔B : jamais de mélange, jamais de commits de A sous l'en-tête de B.

- [ ] **Step 4: Commit**

```bash
git add src/lib/repo.svelte.ts
git commit -m "fix(history): token guard against stale pages on rapid repo switches"
```

---

# Item B — « Locate moved project » (Tasks 2-5)

### Task 2: CAPTURE — `repository update-path` réel (clone temporaire déplacé)

**Files:**
- Create: `src-tauri/tests/fixtures/update_path.ndjson` (si la commande émet du NDJSON exploitable)
- Modify: `src-tauri/tests/fixtures/README.md`

- [ ] **Step 1: Lire le help**

```powershell
$lore = "C:\Users\jimmy\bin\lore.exe"
& $lore repository update-path --help
```

Noter : la syntaxe exacte (argument = nouveau chemin ? `--repository` = ancien ou nouveau ?), et si un identifiant d'instance est requis.

- [ ] **Step 2: Fabriquer un repo déplacé et tester la commande**

```powershell
$scratch = "$env:TEMP\p3-locate"
New-Item -ItemType Directory -Force $scratch | Out-Null
& $lore clone lore://lore.example.com:41337/019f333af5e073d28bb117ad1596784a "$scratch\before" --json | Select-Object -Last 2
Move-Item "$scratch\before" "$scratch\after"
# Sans update-path, le status au nouveau chemin marche-t-il déjà ? (peut-être que
# le chemin stocké ne sert qu'au service/instance) :
& $lore status --repository "$scratch\after" --json | Select-Object -First 2
# Puis la commande, selon la syntaxe du help (hypothèse : nouveau chemin en argument) :
& $lore repository update-path "$scratch\after" --repository "$scratch\after" --json |
  Out-File -Encoding utf8NoBOM src-tauri\tests\fixtures\update_path.ndjson
Get-Content src-tauri\tests\fixtures\update_path.ndjson
& $lore status --repository "$scratch\after" --json | Select-Object -First 2
```

Trois constats possibles (tous valides) :
- **(a)** le status marche déjà après le Move et `update-path` ne fait que réenregistrer l'instance → le flux Locate app = valider par `getStatus` + appeler `update-path` en best-effort (échec non bloquant) ;
- **(b)** le status échoue avant `update-path` et marche après → `update-path` est OBLIGATOIRE dans le flux ;
- **(c)** la commande n'existe pas/échoue toujours → repli spec : valider le nouveau dossier par `getStatus` seul, pas d'appel update-path (documenter).
**Adapter la Task 3 au constat** (le code ci-dessous couvre (a)/(b) ; pour (c), `lore_update_path` devient un simple `lore_status`-check et le README le dit).

- [ ] **Step 3: Cleanup + documentation + commit**

```powershell
Remove-Item -Recurse -Force $scratch
```

Ajouter au README des fixtures la section « repository update-path » (syntaxe réelle, constat (a)/(b)/(c), événements NDJSON observés). Ne committer la fixture QUE si du NDJSON exploitable a été émis.

```bash
git add src-tauri/tests/fixtures/README.md src-tauri/tests/fixtures/update_path.ndjson
git commit -m "test(fixtures): pin repository update-path semantics on a real moved clone"
```

### Task 3: Rust — `os_path_exists` + `lore_update_path` (TDD)

**Files:**
- Modify: `src-tauri/src/commands.rs` (après le bloc `os_open_path`), `src-tauri/src/lib.rs` (invoke_handler)

- [ ] **Step 1: Tests qui échouent**

Dans un nouveau module `repo_health_tests` de commands.rs :

```rust
#[cfg(test)]
mod repo_health_tests {
    use super::*;

    #[test]
    fn path_exists_reports_real_directories() {
        let dir = std::env::temp_dir();
        assert!(path_exists_impl(dir.to_str().unwrap()));
        assert!(!path_exists_impl(dir.join("p3-definitely-missing-xyz").to_str().unwrap()));
    }
}
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml repo_health`
Expected: FAIL compilation — `cannot find function path_exists_impl`.

- [ ] **Step 2: Implémenter**

```rust
/// Pure check behind `os_path_exists` (testable without Tauri).
fn path_exists_impl(path: &str) -> bool {
    std::path::Path::new(path).is_dir()
}

/// Does this directory still exist? Drives the "Missing" state of the repo list.
#[tauri::command]
pub fn os_path_exists(path: String) -> bool {
    path_exists_impl(&path)
}

/// Re-register a repository after the user moved its folder on disk. Runs
/// `lore repository update-path` (semantics pinned by the Task 2 capture),
/// then proves the new path works with a plain status. Constat (a): the
/// update-path call is best-effort; constat (b): its failure is fatal here.
/// (Adapter selon la capture — voir README fixtures.)
#[tauri::command]
pub async fn lore_update_path(new_path: String) -> Result<(), String> {
    blocking(move || {
        // Adapter les args exacts au constat de la Task 2 :
        let _ = run_lore(&["repository", "update-path", &new_path, "--repository", &new_path]);
        // La preuve de vie est le status au nouveau chemin :
        run_lore(&["status", "--repository", &new_path])?;
        Ok(())
    })
    .await
}
```

(Constat (b) : remplacer `let _ =` par `?`. Constat (c) : supprimer l'appel update-path. L'exécuteur applique le constat et le commente.)

Enregistrer `commands::os_path_exists,` et `commands::lore_update_path,` dans lib.rs.

- [ ] **Step 3: PASS + build**

Run: `cargo test --manifest-path src-tauri/Cargo.toml repo_health && cargo build --manifest-path src-tauri/Cargo.toml`
Expected: tests ok, 0 warning nouveau. Suite complète : compte rapporté (99 attendu : 98 + 1).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(repos): os_path_exists + lore_update_path commands"
```

### Task 4: Front — contrat + helper pur `repoHealth` + mock (TDD)

**Files:**
- Create: `src/lib/repoHealth.ts`, `src/lib/repoHealth.test.ts`
- Modify: `src/lib/types.ts`, `src/lib/tauri.ts`, `src/lib/mock.ts`, `src/lib/mock.test.ts`

- [ ] **Step 1: Tests qui échouent** — `src/lib/repoHealth.test.ts` :

```ts
import { describe, it, expect } from 'vitest'
import { missingRepos } from './repoHealth'

describe('missingRepos', () => {
  it('flags exactly the paths whose directory is gone', () => {
    const exists = new Map([['C:/a', true], ['C:/b', false], ['C:/c', true]])
    expect(missingRepos(['C:/a', 'C:/b', 'C:/c'], exists)).toEqual(new Set(['C:/b']))
  })
  it('treats unknown paths as present (no false alarm before the check ran)', () => {
    expect(missingRepos(['C:/a'], new Map())).toEqual(new Set())
  })
})
```

Et dans `mock.test.ts` (describe `mock api`) :

```ts
  it('pathExists honours the missing-repos dev lever', async () => {
    localStorage.setItem('loredesktop.mock.missing', JSON.stringify(['C:/repos/gone']))
    expect(await mock.pathExists('C:/repos/gone')).toBe(false)
    expect(await mock.pathExists('C:/repos/game')).toBe(true)
    localStorage.removeItem('loredesktop.mock.missing')
  })
```

Run: `npx vitest run src/lib/repoHealth.test.ts src/lib/mock.test.ts`
Expected: FAIL (module absent, mock.pathExists absent).

- [ ] **Step 2: Implémenter**

`src/lib/repoHealth.ts` :

```ts
/** Known repo paths whose folder has vanished from disk. Unknown paths (the
 *  existence check hasn't run yet) count as present — never alarm early. */
export function missingRepos(paths: string[], exists: Map<string, boolean>): Set<string> {
  return new Set(paths.filter((p) => exists.get(p) === false))
}
```

`types.ts` (LoreApi, section OS, après `openPath`) :

```ts
  /** Does this directory still exist on disk? Drives the "Missing" repo state. */
  pathExists(path: string): Promise<boolean>
  /** Re-register a repository after its folder moved; resolves when the new path answers a status. */
  updateRepoPath(newPath: string): Promise<void>
```

`tauri.ts` :

```ts
  pathExists: (path) => invoke<boolean>('os_path_exists', { path }),
  updateRepoPath: (newPath) => invoke<void>('lore_update_path', { newPath }),
```

`mock.ts` (après `openPath`) :

```ts
  async pathExists(path: string) {
    // Dev lever: localStorage.setItem('loredesktop.mock.missing', JSON.stringify(['C:/repos/x']))
    const missing: string[] = JSON.parse(localStorage.getItem('loredesktop.mock.missing') ?? '[]')
    return !missing.includes(path)
  },
  async updateRepoPath(_newPath: string) {
    await delay(300)
  },
```

- [ ] **Step 3: PASS complet**

Run: `npx vitest run && npm run check`
Expected: 122 verts (119 + 3 — rapporter le réel), 0 erreur.

- [ ] **Step 4: Commit**

```bash
git add src/lib/repoHealth.ts src/lib/repoHealth.test.ts src/lib/types.ts src/lib/tauri.ts src/lib/mock.ts src/lib/mock.test.ts
git commit -m "feat(repos): pathExists/updateRepoPath contract, missing-repos helper, mock lever"
```

### Task 5: UI — état « Missing » + flux Locate + garde au démarrage

**Files:**
- Modify: `src/lib/RepoSwitcher.svelte`, `src/lib/App.svelte`, `src/lib/repoActions.ts`

- [ ] **Step 1: Vérification d'existence au chargement**

Dans `App.svelte`, dans l'effect de changement de repo (celui qui appelle `refreshStatus`), ajouter en tête un check du repo courant :

```ts
  // A moved/renamed repo folder must not strand the app on a dead path: fall
  // back to the picker with a hint, the Locate flow lives in the switcher.
  let missing = $state<Set<string>>(new Set())
  async function checkRepoHealth() {
    const paths = session.config.repos ?? []
    const entries = await Promise.all(paths.map(async (p) => [p, await api.pathExists(p)] as const))
    missing = missingRepos(paths, new Map(entries))
    const current = session.config.currentRepo
    if (current && missing.has(current)) {
      session.config.currentRepo = null
      toastInfo(`${current.split(/[\\/]/).pop()} is missing — locate it from the repository list`)
    }
  }
```

appelée dans l'effect (fire-and-forget) et passée au RepoSwitcher (`missing` en prop). (Adapter les noms réels de config — `session.config.repos` / la mécanique de sauvegarde ; suivre `repoList.ts`. Signaler tout écart.)

- [ ] **Step 2: RepoSwitcher — ligne Missing + Locate**

Sur chaque entrée dont le chemin est dans `missing` : ligne estompée (`opacity: .6`), badge « Missing » (`--warn-text`), le clic normal désactivé, et un bouton « Locate… » à la place du bouton Remove au survol :

```ts
  async function locate(oldPath: string) {
    const parent = await api.pickFolder()
    if (!parent) return
    try {
      await api.updateRepoPath(parent)
      await api.getStatus(parent) // proof of life + les données du switch
      replaceRepoPath(oldPath, parent) // repoList.ts — à créer si absent : remplace dans config.repos (+ currentRepo si c'était lui) et sauvegarde
      toastInfo('Repository relocated')
    } catch (e) {
      toastError("That folder doesn't answer as this repository", e)
    }
  }
```

(`replaceRepoPath` : petite fonction dans `repoList.ts`, testée dans `repoList.test.ts` — remplacement + dédup + persistance, sur le modèle des fonctions existantes du fichier.)

- [ ] **Step 3: Suites + navigateur (mock, levier missing)**

`npm run check && npx vitest run` (tout vert, compte rapporté). Navigateur : poser le levier `loredesktop.mock.missing` avec un des repos mock + reload → la ligne est estompée avec badge Missing ; « Locate… » → pickFolder mock → toast « Repository relocated » et la ligne redevient normale (retirer le chemin du levier dans le mock updateRepoPath OU documenter que le levier se nettoie à la main). Si le repo COURANT est dans le levier au reload → retour au RepoPicker + toast « … is missing ».

- [ ] **Step 4: Commit**

```bash
git add src/lib/App.svelte src/lib/RepoSwitcher.svelte src/lib/repoActions.ts src/lib/repoList.ts src/lib/repoList.test.ts
git commit -m "feat(repos): missing-repo detection and Locate flow"
```

---

# Item C — Sync-to-revision (Tasks 6-9)

### Task 6: CAPTURE — `sync <revision>` réel + status « behind »

**Files:**
- Create: `src-tauri/tests/fixtures/status_behind.ndjson`
- Modify: `src-tauri/tests/fixtures/README.md`

- [ ] **Step 1: Forme de la commande + comportement**

```powershell
$lore = "C:\Users\jimmy\bin\lore.exe"
$repo = "C:\Users\jimmy\lore-test-repo"
& $lore sync --help          # accepte-t-il une révision en argument ?
& $lore revision sync --help # ou la variante namespaced ?
# Identifier une révision antérieure :
& $lore history 3 --repository $repo --json | Select-String revisionNumber
# Time travel (utiliser le HASH ou le numéro selon le help) :
& $lore sync <rev N-1> --repository $repo --json | Select-Object -Last 3
& $lore status --scan --repository $repo --json | Out-File -Encoding utf8NoBOM src-tauri\tests\fixtures\status_behind.ndjson
Get-Content src-tauri\tests\fixtures\status_behind.ndjson | Select-Object -First 2
# Retour au latest :
& $lore sync --repository $repo --json | Select-Object -Last 3
& $lore status --scan --repository $repo --json | Select-Object -First 2   # attendu : local == remote de nouveau
```

Noter : hash vs numéro accepté ; le status behind — `isRemoteAhead` true ? `revisionLocalNumber < revisionRemoteNumber` ? des fichiers apparaissent-ils « modifiés » (ils ne doivent PAS finir dans Changes comme committables — si le status --scan les liste, le noter : la garde « arbre propre » de l'UI devra se baser sur le status AVANT le voyage). Tester aussi le refus sur arbre sale : `Add-Content README.md "dirty"` → `sync <rev>` → noter l'erreur exacte → `lore reset` du fichier.

- [ ] **Step 2: Cleanup + doc + commit**

Repo de test revenu au latest, propre (`status --scan` vierge). README fixtures : section « sync <revision> / status behind » avec le constat complet.

```bash
git add src-tauri/tests/fixtures/status_behind.ndjson src-tauri/tests/fixtures/README.md
git commit -m "test(fixtures): pin sync-to-revision semantics and the behind status shape"
```

### Task 7: Rust — `lore_sync_to` (streaming) + test fixture behind (TDD)

**Files:**
- Modify: `src-tauri/src/commands.rs`, `src-tauri/src/lib.rs`

- [ ] **Step 1: Test qui échoue** (module `status_tests` existant) :

```rust
    #[test]
    fn behind_fixture_reports_remote_ahead() {
        let events = parse_events(include_str!("../tests/fixtures/status_behind.ndjson")).unwrap();
        let s = status_from(&events, std::path::Path::new(""));
        assert!(s.remote_ahead > 0, "time-traveled status must show the repo behind the tip");
    }
```

(Adapter l'assertion au constat de la Task 6 si le « behind » se lit autrement — c'est le POINT de ce test : pinner la forme réelle. Si un champ nouveau est nécessaire dans le DTO, l'ajouter sur le modèle merge/staged du P1 avec défauts sûrs.)

Run: `cargo test --manifest-path src-tauri/Cargo.toml behind_fixture`
Expected: FAIL (fixture pas encore consommée / assertion rouge si la forme diffère).

- [ ] **Step 2: Implémenter la commande** (après `lore_sync`) :

```rust
/// Time travel: sync the working copy to a specific revision (streaming, same
/// progress relay and stall detection as a plain sync). The UI only offers it
/// on a clean tree; the CLI's own dirty-tree refusal (pinned by the capture)
/// is surfaced as the error toast otherwise.
#[tauri::command]
pub async fn lore_sync_to(app: tauri::AppHandle, repo_path: String, revision: String, op_id: Option<String>) -> Result<(), String> {
    blocking(move || {
        let op_id = op_id_or_default(op_id);
        run_lore_op(&app, "sync", &op_id, &["sync", &revision, "--repository", &repo_path])?;
        Ok(())
    })
    .await
}
```

(Adapter `["sync", &revision, ...]` à la forme pinnée en Task 6 — `revision sync` namespaced le cas échéant.) Enregistrer `commands::lore_sync_to,` dans lib.rs.

- [ ] **Step 3: PASS + build** — suite Rust complète (compte rapporté, ~100), build 0 warning.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(history): lore_sync_to streaming command + behind status pin"
```

### Task 8: `statusChip` kind `behind` + StatusBar + garde Commit (TDD)

**Files:**
- Modify: `src/lib/statusChip.ts`, `src/lib/statusChip.test.ts`, `src/lib/StatusBar.svelte`, `src/lib/Changes.svelte`, `src/lib/types.ts`, `src/lib/tauri.ts`, `src/lib/mock.ts`, `src/lib/mock.test.ts`

- [ ] **Step 1: Tests qui échouent** (statusChip.test.ts) :

```ts
  it('shows the behind chip when the working copy trails the tip', () => {
    expect(chipFor({ ...base, remoteAhead: 3, revisionNumber: 4 })).toEqual({ kind: 'behind', revision: 4 })
  })
  it('merge and staged both take precedence over behind', () => {
    expect(chipFor({ ...base, remoteAhead: 3, mergeInProgress: true })).toEqual({ kind: 'merge' })
    expect(chipFor({ ...base, remoteAhead: 3, stagedPending: true })).toEqual({ kind: 'staged' })
  })
```

- [ ] **Step 2: Implémenter** — `statusChip.ts` :

```ts
export type StatusChip =
  | { kind: 'merge' }
  | { kind: 'staged' }
  | { kind: 'behind'; revision: number }
  | null

/**
 * Which StatusBar chip to show. Merge takes precedence (a merge implies a
 * staged state); behind comes last — it's a normal state after a
 * sync-to-revision, or simply a teammate having pushed.
 */
export function chipFor(status: StatusResult | null): StatusChip {
  if (!status) return null
  if (status.mergeInProgress) return { kind: 'merge' }
  if (status.stagedPending) return { kind: 'staged' }
  if (status.remoteAhead > 0) return { kind: 'behind', revision: status.revisionNumber }
  return null
}
```

`StatusBar.svelte` — troisième branche du chip (bouton, comme merge) :

```svelte
  {:else if chip?.kind === 'behind'}
    <button class="chip" onclick={sync} title="Your working copy is behind the latest revision — click to sync back">
      <Icon name="sync" size={12} /> At rev {chip.revision} — back to latest
    </button>
```

(`sync` vient de `repo.svelte` — importer comme TitleBar le fait. Le chip behind reste GRIS (pas ambre) : état normal, pas un avertissement.)

`Changes.svelte` — garde conservatrice de la spec : quand `chipFor(repo.status)?.kind === 'behind'`, le bouton Commit est `disabled` avec `title="Commit is disabled while behind the latest — sync back first"` (réutiliser le derived, ne pas dupliquer la logique).

`LoreApi` : `syncToRevision(repoPath: string, revision: string, onProgress?: (p: OpProgress) => void): Promise<void>` ; `tauri.ts` : `invokeWithProgress<void>('lore_sync_to', { repoPath, revision }, onProgress)` ; mock :

```ts
  async syncToRevision(repoPath: string, _revision: string, onProgress?: (p: OpProgress) => void) {
    for (let i = 1; i <= 6; i++) { await delay(80); onProgress?.({ done: i, total: 6, unit: 'files' }) }
    stateFor(repoPath).remoteAhead = 3 // time-traveled → behind by construction
  },
```

+ test mock : `syncToRevision` laisse `remoteAhead > 0`, et le `sync` mock existant le remet à 0 (round-trip).

- [ ] **Step 3: PASS complet** — vitest (compte rapporté, ~125) + check 0.

- [ ] **Step 4: Commit**

```bash
git add src/lib/statusChip.ts src/lib/statusChip.test.ts src/lib/StatusBar.svelte src/lib/Changes.svelte src/lib/types.ts src/lib/tauri.ts src/lib/mock.ts src/lib/mock.test.ts
git commit -m "feat(history): behind chip with back-to-latest, commit guard while behind"
```

### Task 9: History — bouton « Sync to this revision… »

**Files:**
- Modify: `src/lib/History.svelte`, `src/lib/repo.svelte.ts`

- [ ] **Step 1: Action store** (repo.svelte.ts, à côté de `sync`) :

```ts
export const syncToRevision = (revision: string) => act('sync', async (p) => {
  try { await api.syncToRevision(p, revision, (prog) => { opProgress.sync = prog }) }
  finally { opProgress.sync = null }
})
```

(Adapter au vrai helper `act` — même wrapper que `sync`/`push`. Signaler l'écart s'il n'y a pas d'`act`.)

- [ ] **Step 2: Bouton dans le détail de commit** (History.svelte, zone des actions du commit sélectionné, à côté d'Undo/Edit) :

```svelte
  {#if !isSelectedTip}
    <button class="mini" disabled={!!repo.busy || dirtyTree}
            title={dirtyTree ? 'Commit or discard your local changes first' : 'Sync the working copy to this revision'}
            onclick={confirmSyncTo}>
      Sync to this revision…
    </button>
  {/if}
```

```ts
  const dirtyTree = $derived((repo.status?.files.length ?? 0) > 0)
  const isSelectedTip = $derived(isLocalTip(selected?.id ?? '', commits))
  async function confirmSyncTo() {
    if (!selected) return
    const ok = await confirmAction(
      `Your working copy will match revision #${selected.revisionNumber}. You'll be behind the latest — sync back when you're done.`,
      'Sync to revision',
    )
    if (ok) syncToRevision(selected.id)
  }
```

(Adapter : le champ passé au backend — hash `selected.id` complet ou `revisionNumber` — suit le constat de la Task 6 ; `confirmAction` et les gardes Undo existantes donnent le pattern exact du fichier. `isLocalTip` est déjà importé depuis P2.)

- [ ] **Step 3: Suites + navigateur (mock)**

check 0, vitest vert. Navigateur : History → commit ancien → « Sync to this revision… » → confirmation → barre de progression sync (TitleBar) → chip « At rev N — back to latest » dans la StatusBar, bouton Commit désactivé avec tooltip ; clic sur le chip → sync → chip disparaît, Commit réactivé. Le bouton est absent sur le commit de tête et désactivé si des fichiers sont modifiés (levier : cocher un fichier mock — l'arbre mock est toujours « sale » par défaut : utiliser un repo mock vidé ou documenter la vérification de la garde par inspection du disabled).

- [ ] **Step 4: Commit**

```bash
git add src/lib/History.svelte src/lib/repo.svelte.ts
git commit -m "feat(history): sync-to-revision from the commit detail"
```

---

## Vérification finale

### Task 10: Suites + navigateur mock + vérifications réelles

- [ ] **Step 1: Les trois suites** — comptes exacts rapportés (attendus ~100 cargo / ~125 vitest / 0-0 check).

- [ ] **Step 2: Parcours navigateur mock** — anti-race (switchs rapides), Missing/Locate (levier), time-travel aller-retour (chip, garde Commit, progression).

- [ ] **Step 3: Vérifications réelles**
1. **Locate réel** : rejouer le scénario de la Task 2 avec l'APP réelle (`npx tauri dev`) : cloner en temp, fermer/rouvrir l'app avec le chemin déplacé → ligne Missing → Locate → l'app bascule sur le repo relocalisé. Cleanup temp.
2. **Time travel réel** : sur `C:\Users\jimmy\lore-test-repo` (propre), History → revision N-1 → Sync to this revision → chip behind → back to latest → status propre, local == remote. Aucun process lore résiduel.

- [ ] **Step 4: Header du plan** — marquer « STATUT : EXÉCUTÉ ET VÉRIFIÉ » (modèle P1/P2) avec comptes, constats des captures (a/b/c d'update-path, forme de sync <rev>), déviations. Commit docs.

---

## Self-review (fait à l'écriture du plan)

- **Couverture spec** : item 3 → Task 1 (jetons refresh + loadMore, précédence refresh > load-more, vérif A/B navigateur) ; item 1 → Tasks 2-5 (capture aux trois constats prévus (a)/(b)/(c) avec repli spec, os_path_exists pur testé, contrat front + levier mock, détection au chargement + bascule RepoPicker + toast, ligne Missing + Locate + replaceRepoPath testé dans repoList) ; item 2 → Tasks 6-9 (capture forme sync <rev> + status behind + refus arbre sale, commande streaming réutilisant run_lore_op/op_id_or_default, chip behind avec précédence testée merge > staged > behind, garde Commit conservatrice, bouton masqué sur le tip et désactivé sur arbre sale, confirmation au wording de la spec) ; Task 10 = suites + mock + 2 vérifs réelles. Hors périmètre respecté (pas de restore par fichier, pas de commit en détaché, pas de create).
- **Placeholders** : aucun TBD — les « adapter » sont les pins de capture (Tasks 2, 6), bornés avec les variantes écrites ((a)/(b)/(c), hash vs numéro).
- **Cohérence de types** : `missingRepos(paths, Map) -> Set` (Task 4) consommé Task 5 ; `pathExists`/`updateRepoPath` (types/tauri/mock, Task 4) ↔ `os_path_exists`/`lore_update_path` (Task 3, camelCase Tauri) ; `chipFor` étendu `{kind:'behind', revision}` (Task 8) consommé StatusBar/Changes ; `syncToRevision` (LoreApi Task 8) ↔ `lore_sync_to` (Task 7, op_id Option pattern P1) ↔ action store (Task 9) ; `isLocalTip` réutilisé de P2 (historySelection).

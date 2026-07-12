# Lore Desktop — A4 « Restaurer une ancienne version d'un fichier » Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Un bouton « Restore this version » sur chaque révision du file-history d'un asset : il ramène ce fichier à son contenu d'une ancienne révision **comme changement en attente** (à committer), texte OU binaire, sans quitter le head.

**Architecture:** `lore` n'a pas de restore-forward par fichier ni de `file cat`, mais `lore sync <rev> --root-file <path>` reconstruit le fichier d'une ancienne révision sur le disque (en déplaçant la révision synchronisée). Le restore est donc un round-trip **local** (aucun push) : sync scopé → lire les octets → re-sync au head → réécrire les octets → le fichier devient une modif/ajout en attente. Le lock exclusif est géré côté front (bloquer=non ; on acquiert le lock si libre, on laisse passer sans lock si un tiers le tient — le fichier tombe alors dans la section « Locked by teammates » non-committable de P1). Garde dure : **arbre de travail propre** (le re-sync réinitialise les fichiers modifiés, et Lore n'a pas de stash).

**Tech Stack:** Rust (Tauri v2, `run_lore` / `run_lore_op` streaming), Svelte 5 runes, TypeScript, vitest (jsdom), PowerShell 7 pour la vérif réelle.

---

## Contexte & conventions (à lire avant toute tâche)

- **Repo de test réel** : working copy `C:\Users\jimmy\lore-test-repo` (repo `desktoptest1`) sur `lore://lore.example.com:41337`, branche `feature/test`. Binaire CLI : `C:\Users\jimmy\bin\lore.exe`. Actuellement propre au rev 25.
- **Capture faite (Task 0, committée)** : `src-tauri/tests/fixtures/sync_root_file.ndjson` + section README « Scoped sync ». Constats pinnés : `lore sync <revSig> --root-file <path> --json` émet `revisionSyncTarget{sourceRevision(+Number),targetRevision(+Number),isLatest,local}`, `dependencyResolveBegin/End{rootCount,resolvedCount}`, `revisionSyncProgress{fileUpdate(Total),bytesUpdate(Total),discoveryComplete}`, `revisionSyncFile{path,size,action,flagFile}`, `revisionSyncRevision{revision,revisionNumber,flagMerge,flagConflict}`, `complete`. La commande **DÉPLACE la révision synchronisée** vers la cible (repo « behind » sur ce fichier). Le round-trip complet a été validé en réel : après re-sync au head + réécriture des octets, `status --scan` montre le fichier en `action:"add"` (ou modify) `flagDirty:true`, `adds:1` — un vrai changement en attente au head. Un « staged vide » résiduel peut apparaître → `lore unstage .` le nettoie.
- **Gotcha cwd (pinné)** : `lore` résout les chemins relatifs contre le **cwd du process**, PAS `--repository`. Toujours passer un chemin **absolu** (`Path::new(&repo_path).join(&path)`), comme `lore_diff` le fait déjà.
- **`--source/--target/--root-file` prennent un HASH de révision** (pas un numéro).
- **Contrainte vitest** : `vitest.config.ts` n'a PAS le plugin Svelte — les tests n'importent NI `.svelte` NI `.svelte.ts`. La logique testée vit dans des modules purs (`restoreGuard.ts`) ; le wiring composant/store se vérifie navigateur.
- **Commandes de test** :
  - Rust : `cargo test --manifest-path src-tauri/Cargo.toml --lib <filtre>` — attendu `test result: ok`.
  - Vitest ciblé : `npx vitest run src/lib/<fichier>.test.ts`. Complet : `npx vitest run`.
  - Typecheck : `npm run check` — attendu 0 errors 0 warnings.
  - Dev navigateur (mock) : `npm run dev` → http://localhost:5173.
- **Décisions de design (tranchées avec Jimmy — ne pas re-débattre)** :
  1. **Restore = LOCAL** : produit un changement en attente ; l'utilisateur commit/push lui-même. Aucune écriture serveur.
  2. **Garde dure « arbre propre »** : le bouton est désactivé s'il y a des changements en attente OU si on est en time-travel (`revisionNumber < localRevisionNumber`), car le re-sync au head réinitialiserait les fichiers modifiés et Lore n'a pas de stash. Raison affichée en tooltip.
  3. **Lock** : libre → on **acquiert le lock** (`setLock true`) avant le restore (pour que ce soit committable) ; locké par soi → restore direct ; locké par un tiers → restore **local quand même**, SANS acquérir, avec une note « pas committable tant que X tient le lock » (le fichier tombe dans la section teammate-locked existante de P1). Un tiers-lock ne DÉSACTIVE pas le bouton.
  4. **Révision courante** : pas de bouton restore sur la révision qui est déjà la copie de travail (no-op).
  5. Hors périmètre v1 : restaurer avec un arbre sale (nécessiterait un re-sync scopé + test réel), et le restore par lot.

## Carte des fichiers

**Créés :**
- `src/lib/restoreGuard.ts` (+ `restoreGuard.test.ts`) — décision pure « peut-on restaurer cette révision + pourquoi + catégorie de lock ».

**Modifiés :**
- `src-tauri/src/commands.rs` — `head_revision_from` (helper) + `lore_restore_file` (commande streaming) + test du helper.
- `src-tauri/src/lib.rs` — enregistrement de `lore_restore_file`.
- `src/lib/types.ts` — `LoreApi.restoreFile`.
- `src/lib/tauri.ts` — `restoreFile` (invoke streaming).
- `src/lib/mock.ts` (+ `mock.test.ts`) — `restoreFile` mock (ajoute un changement en attente + gère le lock).
- `src/lib/repo.svelte.ts` — action `restoreFile` (gardes + lock + toast + bascule vers Changes).
- `src/lib/FileHistorySection.svelte` — bouton « Restore » par révision, câblé sur la garde + l'action.

---

### Task 1: Backend — `head_revision_from` + `lore_restore_file`

**Files:**
- Modify: `src-tauri/src/commands.rs` (ajouter après `lore_sync_to` / la zone des commandes sync), `src-tauri/src/lib.rs`

- [ ] **Step 1: Écrire le test du helper (échoue)**

Ajouter dans le module de tests de status de `commands.rs` (à côté de `behind_fixture_reports_a_past_revision`) :

```rust
    #[test]
    fn head_revision_is_the_local_head_hash() {
        let events = parse_events(include_str!("../tests/fixtures/status.ndjson")).unwrap();
        // revisionLocal is the local head signature we sync back to after a restore.
        let head = head_revision_from(&events).expect("status carries a local head");
        assert_eq!(head.len(), 64, "a full revision hash, was {head:?}");
    }
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib head_revision_is_the_local_head_hash` — Expected: erreur de compilation (`head_revision_from` inexistant).

- [ ] **Step 2: Implémenter le helper + la commande**

Ajouter dans `commands.rs` (près des autres helpers de status ; `events_with_tag` renvoie `Vec<&serde_json::Value>`) :

```rust
/// The local head revision signature from a `status` stream (the revision we
/// sync BACK to after a scoped restore). `revisionLocal` is the local head even
/// when the working copy is time-traveled below it.
fn head_revision_from(events: &[LoreEvent]) -> Option<String> {
    events_with_tag(events, "repositoryStatusRevision")
        .into_iter()
        .next()
        .and_then(|d| d.get("revisionLocal").and_then(|v| v.as_str()))
        .filter(|h| !h.is_empty())
        .map(String::from)
}
```

et la commande (mirror de `lore_sync_to` pour le streaming ; `op_id`/`run_lore_op` relaient la progression sync) :

```rust
/// Restore one file to the content it had at `revision`, as a working-copy
/// change on top of the current head. LOCAL only — nothing is pushed.
///
/// `lore sync <rev> --root-file <path>` rebuilds the file at that revision on
/// disk but moves the repo-wide synced pointer, so we round-trip: scoped sync to
/// `revision`, read the bytes, sync back to the head, write the bytes back. The
/// file then shows as a pending add/modify. Guards (clean tree, locks) live in
/// the frontend. On any failure after the first sync we best-effort sync back to
/// the head so the repo is never left time-traveled.
#[tauri::command]
pub async fn lore_restore_file(
    app: tauri::AppHandle,
    repo_path: String,
    path: String,
    revision: String,
    op_id: Option<String>,
) -> Result<(), String> {
    blocking(move || {
        let op_id = op_id_or_default(op_id);
        let head = head_revision_from(&run_lore(&["status", "--repository", &repo_path])?)
            .ok_or_else(|| "couldn't read the current head revision".to_string())?;
        let abs = std::path::Path::new(&repo_path).join(&path);
        let abs_str = abs.to_string_lossy().to_string();

        // 1. Scoped sync: bring the file's `revision` content onto disk.
        run_lore_op(&app, "sync", &op_id, &["sync", &revision, "--root-file", &abs_str, "--repository", &repo_path])?;
        // 2. Read the restored bytes (do this BEFORE syncing back — the sync-back resets the file).
        let bytes = match std::fs::read(&abs) {
            Ok(b) => b,
            Err(e) => {
                let _ = run_lore_op(&app, "sync", &op_id, &["sync", &head, "--repository", &repo_path]);
                return Err(format!("reading the restored file: {e}"));
            }
        };
        // 3. Sync back to the head (unscoped): returns the repo to the tip; the file is reset to its head state.
        run_lore_op(&app, "sync", &op_id, &["sync", &head, "--repository", &repo_path])?;
        // 4. Write the old bytes back — the file is now a pending change at the head.
        std::fs::write(&abs, &bytes).map_err(|e| format!("writing the restored file: {e}"))?;
        // 5. Clear any residual empty-staged marker the round-trip may leave (best-effort).
        let _ = run_lore(&["unstage", ".", "--repository", &repo_path]);
        Ok(())
    })
    .await
}
```

⚠ Vérifier la signature EXACTE de `run_lore_op` / `op_id_or_default` / `lore_sync_to` dans le fichier réel et calquer dessus (ordre des args `(&app, kind, &op_id, &args)`, type de `op_id`). Ne pas inventer d'helper.

Dans `src-tauri/src/lib.rs`, ajouter à l'`invoke_handler` (près de `commands::lore_sync_to,`) :

```rust
        commands::lore_restore_file,
```

- [ ] **Step 3: Vérifier**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib head_revision_is_the_local_head_hash` — Expected: `test result: ok`.
Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib` — Expected: `ok`, 0 failed (la commande I/O n'a pas de test unitaire — couverte par la vérif réelle Task 6).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(restore): lore_restore_file — scoped-sync round-trip for a per-file restore"
```

---

### Task 2: Module pur `restoreGuard.ts` (TDD)

**Files:**
- Create: `src/lib/restoreGuard.ts`, `src/lib/restoreGuard.test.ts`

- [ ] **Step 1: Écrire les tests qui échouent**

Créer `src/lib/restoreGuard.test.ts` :

```ts
import { describe, it, expect } from 'vitest'
import { restoreAvailability } from './restoreGuard'

const base = { isCurrent: false, dirtyTree: false, timeTraveled: false, lockHolder: null as string | null }

describe('restoreAvailability', () => {
  it('allows a restore on a clean tree at the tip, file free', () => {
    expect(restoreAvailability(base)).toEqual({ canRestore: true, reason: null, lock: 'free' })
  })
  it('classifies the lock: mine / teammate / free', () => {
    expect(restoreAvailability({ ...base, lockHolder: 'you' }).lock).toBe('mine')
    expect(restoreAvailability({ ...base, lockHolder: 'Maya R' }).lock).toBe('teammate')
    expect(restoreAvailability({ ...base, lockHolder: null }).lock).toBe('free')
  })
  it('a teammate lock does NOT disable the restore (it stays local, just not committable)', () => {
    const r = restoreAvailability({ ...base, lockHolder: 'Maya R' })
    expect(r.canRestore).toBe(true)
    expect(r.lock).toBe('teammate')
  })
  it('disables the current revision (already the working copy)', () => {
    expect(restoreAvailability({ ...base, isCurrent: true })).toMatchObject({ canRestore: false })
    expect(restoreAvailability({ ...base, isCurrent: true }).reason).toMatch(/current/i)
  })
  it('disables on a dirty tree (no stash to park pending work)', () => {
    const r = restoreAvailability({ ...base, dirtyTree: true })
    expect(r.canRestore).toBe(false)
    expect(r.reason).toMatch(/commit or discard/i)
  })
  it('disables while time-traveled (sync back first)', () => {
    const r = restoreAvailability({ ...base, timeTraveled: true })
    expect(r.canRestore).toBe(false)
    expect(r.reason).toMatch(/latest/i)
  })
  it('precedence: current > dirty > time-traveled', () => {
    expect(restoreAvailability({ isCurrent: true, dirtyTree: true, timeTraveled: true, lockHolder: null }).reason).toMatch(/current/i)
    expect(restoreAvailability({ isCurrent: false, dirtyTree: true, timeTraveled: true, lockHolder: null }).reason).toMatch(/commit or discard/i)
  })
})
```

Run: `npx vitest run src/lib/restoreGuard.test.ts` — Expected: FAIL (`Cannot find module`).

- [ ] **Step 2: Implémenter**

Créer `src/lib/restoreGuard.ts` :

```ts
export type RestoreLock = 'free' | 'mine' | 'teammate'

export interface RestoreAvailability {
  /** Whether the "Restore this version" action is offered. */
  canRestore: boolean
  /** Why it's disabled (tooltip), or null when enabled. */
  reason: string | null
  /** Lock category of the file — drives whether we acquire the lock + the note. */
  lock: RestoreLock
}

export interface RestoreContext {
  /** This revision is already the working-copy tip — restoring it is a no-op. */
  isCurrent: boolean
  /** Any pending working changes — a restore's sync round-trip would clobber them. */
  dirtyTree: boolean
  /** Working copy sits below the local head (P3 time-travel). */
  timeTraveled: boolean
  /** Lock holder of THIS file: 'you', a teammate's name, or null (unlocked). */
  lockHolder: string | null
}

/**
 * Can this file revision be restored, and how? A teammate lock does NOT disable
 * it — the restore is local (the file lands in the teammate-locked, non-committable
 * section). The hard guards are: not the current revision, a clean working tree
 * (no stash in Lore), and not time-traveled.
 */
export function restoreAvailability(ctx: RestoreContext): RestoreAvailability {
  const lock: RestoreLock = ctx.lockHolder == null ? 'free' : ctx.lockHolder === 'you' ? 'mine' : 'teammate'
  if (ctx.isCurrent) return { canRestore: false, reason: 'This is the current version', lock }
  if (ctx.dirtyTree) return { canRestore: false, reason: 'Commit or discard your pending changes first', lock }
  if (ctx.timeTraveled) return { canRestore: false, reason: 'Sync back to the latest first', lock }
  return { canRestore: true, reason: null, lock }
}
```

- [ ] **Step 3: Vérifier**

Run: `npx vitest run src/lib/restoreGuard.test.ts` — Expected: PASS (7 tests).

- [ ] **Step 4: Commit**

```bash
git add src/lib/restoreGuard.ts src/lib/restoreGuard.test.ts
git commit -m "feat(restore): pure restoreAvailability guard (clean tree / lock category)"
```

---

### Task 3: Surface d'API `restoreFile` (types / tauri / mock) + test mock

**Files:**
- Modify: `src/lib/types.ts` (`LoreApi`), `src/lib/tauri.ts`, `src/lib/mock.ts`, `src/lib/mock.test.ts`

- [ ] **Step 1: Test mock qui échoue**

Ajouter à la fin de `src/lib/mock.test.ts` :

```ts
describe('mock restoreFile', () => {
  it('adds the restored file as a pending change (held by you when it was free)', async () => {
    const repo = 'C:/repos/restore'
    const before = await mock.getStatus(repo)
    const hadIt = before.files.some((f) => f.path === 'Content/Maps/Level_01.umap')
    await mock.restoreFile(repo, 'Content/Maps/Level_01.umap', 'deadbeef')
    const after = await mock.getStatus(repo)
    const f = after.files.find((x) => x.path === 'Content/Maps/Level_01.umap')
    expect(f, hadIt ? 'file already present' : 'file now pending').toBeTruthy()
    expect(f!.lockedBy).toBe('you')
  })
})
```

Run: `npx vitest run src/lib/mock.test.ts` — Expected: FAIL (`restoreFile` inexistant).

- [ ] **Step 2: Type + API + mock**

`src/lib/types.ts` — dans `LoreApi`, après `syncToRevision(...)` :

```ts
  /** Restore one file to its content at `revision` as a working change (LOCAL — nothing pushed). Progress ticks stream via onProgress. */
  restoreFile(repoPath: string, path: string, revision: string, onProgress?: (p: OpProgress) => void): Promise<void>
```

`src/lib/tauri.ts` — après `syncToRevision:` (mirror la ligne streaming) :

```ts
  restoreFile: (repoPath, path, revision, onProgress) =>
    invokeWithProgress<void>('lore_restore_file', { repoPath, path, revision }, onProgress),
```

`src/lib/mock.ts` — après `syncToRevision` (simule un changement en attente ; réutiliser `stateFor` + le type `ChangedFile` déjà importés) :

```ts
  async restoreFile(repoPath: string, path: string, _revision: string, onProgress?: (p: OpProgress) => void) {
    for (let i = 1; i <= 6; i++) {
      await delay(70)
      onProgress?.({ done: i, total: 6, unit: 'files' })
    }
    const s = stateFor(repoPath)
    const existing = s.files.find((f) => f.path === path)
    if (existing) {
      existing.action = 'modify'
    } else {
      s.files = [
        { path, action: 'add', isBinary: /\.(umap|uasset|png|wav|obj|fbx)$/i.test(path), size: 2_400_000, lockedBy: 'you' },
        ...s.files,
      ]
    }
  },
```

(⚠ adapter les champs de `ChangedFile` au type réel dans `types.ts` — `path/action/isBinary/size/lockedBy`. Le mock met `lockedBy:'you'` car la voie testée acquiert le lock ; la vraie logique lock/teammate vit dans `repo.svelte.ts`, pas dans le mock.)

- [ ] **Step 3: Vérifier**

Run: `npx vitest run src/lib/mock.test.ts` — Expected: PASS.
Run: `npm run check` — Expected: 0 errors, 0 warnings.

- [ ] **Step 4: Commit**

```bash
git add src/lib/types.ts src/lib/tauri.ts src/lib/mock.ts src/lib/mock.test.ts
git commit -m "feat(restore): restoreFile API surface + mock pending-change simulation"
```

---

### Task 4: Action `restoreFile` dans `repo.svelte.ts` (gardes + lock)

**Files:**
- Modify: `src/lib/repo.svelte.ts`

- [ ] **Step 1: Écrire l'action**

READ d'abord `repo.svelte.ts` : repérer les imports (`api`, `session`, `opProgress`, `toastError`/`toastInfo`, `setView` de `ui.svelte`, `locks`, `baseName` s'il existe), le pattern de `syncToRevision`/`push`, et `refreshStatus`/`refreshLocks`. Ajouter (après `syncToRevision`) :

```ts
// Restore one file to an older revision as a pending change (LOCAL). Callers
// (FileHistorySection) only enable this when the tree is clean and we're at the
// tip — see restoreGuard. Lock handling per the design: free → acquire the lock
// so the change is committable; teammate-locked → restore anyway without locking
// (it lands in the excluded "Locked by teammates" section); mine → just restore.
export async function restoreFile(path: string, revision: string, lockHolder: string | null) {
  const p = session.config.currentRepo
  if (!p || repo.busy) return
  const teammateLocked = lockHolder != null && lockHolder !== 'you'
  const short = path.split(/[\\/]/).pop() ?? path
  repo.busy = 'sync'
  try {
    if (lockHolder == null) {
      // Free file → take the lock first so the restored change is committable.
      try {
        await api.setLock(p, path, true)
      } catch (e) {
        repo.busy = ''
        toastError("Couldn't lock the file — someone may hold it now", e)
        return
      }
    }
    try {
      await api.restoreFile(p, path, revision, (prog) => { opProgress.sync = prog })
    } finally {
      opProgress.sync = null
    }
  } catch (e) {
    repo.busy = ''
    toastError('Restore failed', e)
    return
  }
  await refreshStatus() // resets repo.busy in its finally; surfaces the pending change
  refreshLocks(true)
  setView('changes')
  toastInfo(
    teammateLocked
      ? `Restored ${short} — you can't commit it while someone else holds the lock`
      : `Restored ${short} — review and commit it in Changes`,
  )
}
```

⚠ Adapter aux exports réels : si `setView` vit dans `ui.svelte` et n'est pas déjà importé dans `repo.svelte.ts`, l'importer ; si un helper `baseName` existe déjà, le réutiliser au lieu du split inline ; vérifier que `repo.busy` accepte `'sync'` (union `'commit'|'push'|'sync'|'status'`). Si l'import de `setView` crée un cycle, faire la bascule de vue côté composant (Task 5) à la place et retirer `setView` ici — noter le choix.

- [ ] **Step 2: Typecheck**

Run: `npm run check` — Expected: 0 errors, 0 warnings.

- [ ] **Step 3: Commit**

```bash
git add src/lib/repo.svelte.ts
git commit -m "feat(restore): repo action — acquire lock when free, restore, jump to Changes"
```

---

### Task 5: Bouton « Restore » dans `FileHistorySection.svelte`

**Files:**
- Modify: `src/lib/FileHistorySection.svelte`

- [ ] **Step 1: Câbler la garde + le bouton**

READ `FileHistorySection.svelte` : il reçoit `path` + `revisions` (bindable) et rend la timeline. Repérer la révision « courante » (la plus récente = `revisions[0]`, la copie de travail) et la structure d'une ligne de révision.

Imports à ajouter :

```ts
  import { repo, restoreFile } from './repo.svelte'
  import { locks } from './repo.svelte' // si `locks` est exporté depuis repo.svelte ; sinon depuis son module réel — vérifier
  import { restoreAvailability } from './restoreGuard'
  import { confirmAction } from './confirm' // le helper déjà utilisé par History (undo/syncTo) — vérifier le chemin réel
```

Dérivés (le lock du fichier courant + l'état de l'arbre) :

```ts
  const lockHolder = $derived(locks.list.find((l) => l.path === path)?.holder ?? null)
  const dirtyTree = $derived((repo.status?.files.length ?? 0) > 0)
  const timeTraveled = $derived(
    (repo.status?.revisionNumber ?? 0) > 0 &&
    repo.status!.revisionNumber < (repo.status?.localRevisionNumber ?? repo.status!.revisionNumber),
  )

  async function onRestore(rev: { revision: string; revisionNumber: number }) {
    const avail = restoreAvailability({ isCurrent: false, dirtyTree, timeTraveled, lockHolder })
    if (!avail.canRestore) return
    const note = avail.lock === 'teammate'
      ? ` It's locked by someone else — you'll be able to restore it locally, but not commit it until they release it.`
      : avail.lock === 'free'
        ? ` The file will be locked to you so you can commit it.`
        : ''
    const ok = await confirmAction(
      `Restore ${path.split(/[\\/]/).pop()} to its version at #${rev.revisionNumber}? It becomes a pending change in Changes.${note}`,
      'Restore this version',
    )
    if (ok) restoreFile(path, rev.revision, lockHolder)
  }
```

Markup — pour chaque révision NON courante (donc pas `revisions[0]`), un bouton discret. Calculer l'availability par ligne (`isCurrent = index === 0`) et rendre :

```svelte
  {#each revisions as rev, i (rev.revision)}
    {@const avail = restoreAvailability({ isCurrent: i === 0, dirtyTree, timeTraveled, lockHolder })}
    <!-- ...ligne de révision existante... -->
    {#if i !== 0}
      <button class="restore" disabled={!avail.canRestore || !!repo.busy}
              title={avail.reason ?? 'Restore this version as a pending change'}
              onclick={() => onRestore(rev)}>Restore</button>
    {/if}
  {/each}
```

(Placer le bouton dans la ligne de révision existante, à droite ; garder le markup de la ligne inchangé par ailleurs. Adapter les noms de champs de `FileRevision` — `revision`/`revisionNumber` — au type réel dans `types.ts`.)

CSS (à la fin du `<style>`, ton discret cohérent avec les mini-boutons existants) :

```css
  .restore { flex: none; padding: 2px 8px; font-size: 10.5px; color: var(--text-muted); background: var(--bg); border: 1px solid var(--border); border-radius: 5px; }
  .restore:hover:not(:disabled) { color: var(--text); background: var(--panel-hover); }
```

- [ ] **Step 2: Typecheck**

Run: `npm run check` — Expected: 0 errors, 0 warnings.

- [ ] **Step 3: Vérification navigateur (mock)**

`npm run dev` → http://localhost:5173. **Depuis History** (arbre propre) : ouvrir un commit → preview d'un fichier → la section « History · N revisions » montre un bouton « Restore » sur les révisions passées (pas la #tip). Cliquer → confirm → progression → bascule sur Changes, le fichier apparaît comme changement en attente, toast « Restored … ». **Depuis Changes** (arbre sale) : le bouton Restore est désactivé avec le tooltip « Commit or discard your pending changes first ». Vérifier la console sans erreur.

- [ ] **Step 4: Commit**

```bash
git add src/lib/FileHistorySection.svelte
git commit -m "feat(restore): Restore this version button in the file-history timeline"
```

---

### Task 6: Vérification finale (suites + navigateur + réel)

**Files:** aucun nouveau — vérification.

- [ ] **Step 1: Suites complètes**

```powershell
npx vitest run
cargo test --manifest-path src-tauri/Cargo.toml --lib
npm run check
```

Expected : vitest tous fichiers passed (nouveaux `restoreGuard.test.ts` + mock étendu), 0 failed ; cargo `ok` (nouveau `head_revision_is_the_local_head_hash`) ; svelte-check 0/0.

- [ ] **Step 2: Parcours navigateur mock**

Dérouler et cocher : bouton Restore visible seulement sur les révisions non-tip ; désactivé + raison si arbre sale ; clic sur arbre propre → confirm (avec la note de lock selon le cas) → changement en attente + toast + vue Changes ; console propre.

- [ ] **Step 3: RÉEL — restore bout-en-bout (Tauri)**

Sur `C:\Users\jimmy\lore-test-repo` (propre, rev 25). Le fichier `p6-difftest.txt` a existé (ajouté rev23, modifié rev24, supprimé rev25) — cible idéale. Dans l'app Tauri (`npm run tauri dev`) : ouvrir le file-history de `p6-difftest.txt` (via un commit où il apparaît), cliquer « Restore » sur la révision #24 → attendu : le fichier revient en `Content`/racine comme ajout en attente (3 lignes), verrouillé par toi, vue Changes. **Cleanup** : discarder le changement (`lore file reset p6-difftest.txt --purge` ou bouton Discard) + `lore unlock`/release, et `& $lore status --scan --repository C:\Users\jimmy\lore-test-repo --json` doit remontrer un arbre propre (isLocalAhead:0, revisionStaged 0, summary vide). Si non pilotable en auto (fenêtre native), documenter l'équivalence CLI déjà validée en discovery (round-trip → `action:add flagDirty:true adds:1`, `unstage` nettoie le staged résiduel).

- [ ] **Step 4: Marquer le plan exécuté**

Ajouter un bloc « STATUT : EXÉCUTÉ ET VÉRIFIÉ le <date> » (compte de tests, points navigateur, constat réel). Commit :

```bash
git add docs/superpowers/plans/2026-07-12-lore-desktop-a4-file-restore.md
git commit -m "docs: mark A4 file-restore plan executed and verified"
```

---

## Self-review (fait à l'écriture du plan)

- **Couverture design** : mécanique Flow B → Task 1 (commande streaming round-trip + garde-fou recovery sync-back on error + nettoyage staged) ; restore LOCAL → Task 1 (aucun push) ; garde arbre-propre + time-travel → Task 2 (`restoreAvailability`) consommée Task 5 ; lock (libre→acquire / soi→direct / tiers→local sans lock + note) → Task 4 (action) + Task 5 (confirm/note) ; teammate-lock ne désactive pas → Task 2 test explicite ; révision courante sans bouton → Task 5 (`i !== 0`) ; bascule vers Changes + toast → Task 4 ; texte ET binaire → copie d'octets (Task 1), pas de dépendance à `file cat`.
- **Placeholders** : aucun TBD/TODO ; chaque step porte son code ; les seuls « adapter/vérifier » sont des points d'intégration bornés (signatures `run_lore_op`, exports de stores, champs de `ChangedFile`/`FileRevision`, chemin de `confirmAction`/`locks`) explicitement nommés.
- **Cohérence de types** : `restoreAvailability(ctx)→{canRestore,reason,lock}` (Task 2) consommé Tasks 5 ; `RestoreLock` = 'free'|'mine'|'teammate' ; `restoreFile(repoPath,path,revision,onProgress?)` (Task 3 API) ↔ `lore_restore_file(repo_path,path,revision,op_id)` (Task 1 Rust, camelCase via serde) ↔ action `restoreFile(path,revision,lockHolder)` (Task 4, front, résout le lock puis appelle l'API) ↔ appel `restoreFile(path, rev.revision, lockHolder)` (Task 5).

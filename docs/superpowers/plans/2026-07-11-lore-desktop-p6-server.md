# Lore Desktop — Lot P6 « serveur & création » Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Livrer les 3 items du lot P6 (spec `docs/superpowers/specs/2026-07-11-lore-desktop-p6-server-design.md`) dans l'ordre 2 → 1 → 3 : toast « Sync & push » sur push refusé non-fast-forward (Tasks 1–4), case/toggle « Use shared store » au clone (Tasks 5–8), et création de repository depuis l'app (Tasks 9–12, **⚠ GATED : n'exécuter que sur arbitrage explicite de Jimmy** — sans arbitrage, sauter directement à la Task 13).

**Architecture:** Deux items commencent par une DISCOVERY/CAPTURE réelle contre le CLI `lore` (le troisième aussi, s'il est arbitré) : les encodages/flags exacts sont pinnés en fixtures avant tout code, et les tâches aval existent en **variantes complètes** (A/B) — l'exécuteur choisit selon le constat, il ne réinvente rien. Frontend Svelte 5 : la logique testable vit dans des modules purs (`pushErrors.ts`, `sharedStore.ts`, `repoName.ts`) car vitest ne compile pas les runes ; le wiring vit dans `repo.svelte.ts` / composants et se vérifie navigateur (mock) puis en réel. Backend Rust : nouvelles commandes `lore_shared_store_*` (et `lore_repository_create` si arbitré) sur le pattern `run_lore` + parseur pinné fixture de `commands.rs`.

**Tech Stack:** Svelte 5 runes, TypeScript, vitest (jsdom), Rust (Tauri v2, `run_lore`/`run_lore_streaming`), PowerShell 7 pour les captures et vérifications réelles.

---

## Contexte & conventions (à lire avant toute tâche)

- **Repo de test réel** : working copy `C:\Users\jimmy\lore-test-repo` (repo serveur `desktoptest1`, id `019f333af5e073d28bb117ad1596784a` — à reconfirmer via `repository list`) sur `lore://lore.example.com:41337`. Binaire CLI : `C:\Users\jimmy\bin\lore.exe`.
- **Gotcha cwd** : toute commande `lore` file-scoped (`stage`, `diff`, `lock`, …) résout les chemins relatifs contre le cwd du process, PAS contre `--repository`. Dans les scripts de capture, préférer `--repository <abs>` + chemins absolus.
- **Ce que voit le frontend en cas d'erreur** (chaîne du matcher, Task 2) : `check_ok` (`src-tauri/src/lore.rs:33-48`) renvoie soit la **sérialisation JSON du `data` du premier événement `error`** (ex. `{"errorInner":"…"}`), soit `lore exited with status N` s'il n'y a pas d'événement `error`. Le runner streaming (push/sync/clone) jette la **stderr** (`Stdio::null()`) — seul le stdout NDJSON compte. Tauri `invoke` rejette avec cette **string brute** (pas un `Error`) ; le mock, lui, jette un `Error`. D'où le helper `errorMessage` (Task 2).
- **Contrainte vitest** : `vitest.config.ts` n'a PAS le plugin Svelte — les tests ne peuvent importer NI un `.svelte` NI un `.svelte.ts` à runes. Toute logique testée vit dans des modules purs ; le wiring se vérifie navigateur (mock) et en réel (Task 13).
- **Fixtures** : PowerShell 7 — `Out-File -Encoding utf8` écrit de l'UTF-8 **sans BOM** (obligatoire : un BOM casserait le parse de la première ligne dans `include_str!`).
- **Commandes de test** :
  - Vitest ciblé : `npx vitest run src/lib/<fichier>.test.ts` — attendu `Test Files  1 passed`.
  - Vitest complet : `npx vitest run` — attendu `Test Files  N passed`, 0 failed.
  - Rust : `cargo test --manifest-path src-tauri/Cargo.toml <filtre>` — attendu `test result: ok`.
  - Typecheck : `npm run check` — attendu `svelte-check found 0 errors and 0 warnings`.
  - Dev navigateur (mock) : `npm run dev` → http://localhost:5173.
- **Hypothèses pinnées par les discoveries** (chaque valeur marquée `⚠ pin Task N` dans le code ci-dessous est une hypothèse plausible, PAS une valeur constatée — l'exécuteur DOIT la remplacer par la capture avant d'exécuter la tâche qui la contient) :
  - Task 1 pinne : (a) le message exact du push non-fast-forward (event `error` discriminant ou status seul), (b) l'existence de `push --fast-forward-merge`, (c) le comportement du sync de rattrapage (automerge auto-committé / merge à committer / conflit).
  - Task 5 pinne : tags/champs de `shared-store info`, mécanique d'activation (flag de clone par-clone vs réglage global `set-use-automatically`), emplacement du store.
  - Task 9 (GATED) pinne : forme exacte de `repository create`, tag/champ de l'id retourné, charset des noms, existence de `repository delete`.

## Carte des fichiers

**Créés :**
- `src/lib/pushErrors.ts` + `src/lib/pushErrors.test.ts` — matcher pinné du push refusé + `errorMessage`.
- `src/lib/sharedStore.ts` + `src/lib/sharedStore.test.ts` — état par défaut + hint de la case shared-store.
- `src/lib/repoName.ts` + `src/lib/repoName.test.ts` — validation du nom de repo (**GATED**).
- `src-tauri/tests/fixtures/push_nonff.ndjson` — capture Task 1.
- `src-tauri/tests/fixtures/shared_store_info.ndjson` (+ `shared_store_info_none.ndjson` si le cas « aucun store » sort en status 0) — captures Task 5.
- `src-tauri/tests/fixtures/repo_create.ndjson` — capture Task 9 (**GATED**).

**Modifiés :**
- `src/lib/repo.svelte.ts` — `act` → booléen + `onError`, push avec toast action, `syncAndPush`.
- `src/lib/mock.ts` + `src/lib/mock.test.ts` — leviers `pushNonFF` / `sharedStore`, `sharedStoreStatus/Enable/Disable`, `createRepo` (**GATED**).
- `src/lib/types.ts`, `src/lib/tauri.ts` — API shared-store, param `useSharedStore` du clone (variante A), `createRepo` (**GATED**).
- `src/lib/repoActions.ts` — `cloneServerRepo(entry, useSharedStore)` (variante A), `createServerRepo` (**GATED**).
- `src/lib/RepoPicker.svelte`, `src/lib/RepoSwitcher.svelte` — case « Use shared store » (variante A), « New repository… » (**GATED**).
- `src/lib/AvatarMenu.svelte` — toggle global « Use shared store for clones » (variante B uniquement).
- `src-tauri/src/commands.rs` — `lore_shared_store_status/enable/disable`, flag de clone (variante A), `lore_repository_create` (**GATED**).
- `src-tauri/src/lib.rs` — enregistrement des nouvelles commandes.
- `src-tauri/tests/fixtures/README.md` — encodages pinnés (Tasks 1, 5, 9).

---

# Item 2 — « Sync & push » sur push refusé (Tasks 1–4)

### Task 1: CAPTURE — fabriquer et pinner le push non-fast-forward réel

**Files:**
- Create: `src-tauri/tests/fixtures/push_nonff.ndjson`
- Modify: `src-tauri/tests/fixtures/README.md` (section « Push non-fast-forward »)

Objectif : produire un vrai refus de push (le remote a avancé pendant qu'un commit local existait), capturer le NDJSON exact, et répondre à trois questions qui aiguillent les Tasks 2–3.

- [ ] **Step 1: Vérifier l'état de départ et l'id du repo**

```powershell
$lore = "C:\Users\jimmy\bin\lore.exe"
$srv  = "lore://lore.example.com:41337"
$main = "C:\Users\jimmy\lore-test-repo"
& $lore repository list $srv --json
& $lore status --scan --repository $main --json
```

Attendu : `desktoptest1` dans la liste (noter son `id` — normalement `019f333af5e073d28bb117ad1596784a`) ; status du repo principal **propre** (aucun `repositoryStatusFile`, `isLocalAhead`/`isRemoteAhead` false). Si le repo n'est pas propre, committer/pousser d'abord ce qui traîne (sinon la capture serait polluée).

- [ ] **Step 2: Second clone temporaire + « avance distante » simulée**

```powershell
$tmp = "C:\Users\jimmy\lore-tmp-p6-clone"
if (Test-Path $tmp) { Remove-Item -Recurse -Force $tmp }
& $lore clone "$srv/019f333af5e073d28bb117ad1596784a" $tmp --json
Set-Content -Path "$tmp\p6-remote.txt" -Value "remote advance $(Get-Date -Format o)"
& $lore stage . --scan --repository $tmp --json
& $lore commit "p6: simulated remote advance" --repository $tmp --json
& $lore push --repository $tmp --json
```

Attendu : chaque commande finit sur `{"tagName":"complete","data":{"status":0}}`. (Fichier `p6-remote.txt` ≠ fichier de l'étape 3 → le sync de rattrapage n'aura PAS de conflit — c'est voulu, le chemin conflictuel est géré par le chip merge existant, pas par ce lot.)

- [ ] **Step 3: Commit local dans le repo principal (maintenant en retard)**

```powershell
Set-Content -Path "$main\p6-local.txt" -Value "local commit $(Get-Date -Format o)"
& $lore stage . --scan --repository $main --json
& $lore commit "p6: local commit behind remote" --repository $main --json
```

- [ ] **Step 4: LA CAPTURE — push refusé**

```powershell
$fix = "C:\Users\jimmy\Documents\SoonerOrLater\lore-desktop\src-tauri\tests\fixtures"
& $lore push --repository $main --json | Out-File -Encoding utf8 "$fix\push_nonff.ndjson"
"exit=$LASTEXITCODE"
& $lore push --repository $main 2>&1 | Out-File -Encoding utf8 "$env:TEMP\p6-push-stderr-check.txt"   # 2e essai SANS --json : juste pour voir le texte humain (info)
Get-Content "$fix\push_nonff.ndjson"
```

Attendu : exit ≠ 0. **Répondre par écrit (dans la section README du Step 7) aux constats :**
- **(a) Forme de l'erreur** : y a-t-il un événement `{"tagName":"error","data":{…}}` dans le stdout `--json` ? Si oui, noter la sérialisation JSON exacte de son `data` (c'est CETTE chaîne que le frontend reçoit, cf. `check_ok`) et choisir dedans une **sous-chaîne discriminante** (ex. le texte d'`errorInner`) → Task 2 **variante 2A**. S'il n'y a QUE `complete status N` (pas d'événement error), le frontend ne voit que `lore exited with status N` → Task 2 **variante 2B**.
- **(b) `--fast-forward-merge`** : `& $lore push --help` — l'option existe-t-elle ? Si oui, Task 3 **variante 3A**, sinon **variante 3B**.

```powershell
& $lore push --help
```

- [ ] **Step 5: Constat (c) — comportement du sync de rattrapage, puis résolution**

```powershell
& $lore sync --repository $main --json
& $lore status --scan --repository $main --json
```

Noter : après ce sync sans conflit, le status montre-t-il un merge/staged résiduel (`revisionMerged`/`revisionStaged` non-zéro → il faut un `commit` avant de pousser) ou est-ce auto-committé (`isLocalAhead` true, prêt à pousser) ? **Si un commit est requis**, l'exécuter et le documenter — la variante 3B de la Task 3 contient déjà le garde-fou (arrêt sur `mergeInProgress`/`stagedPending`), mais le constat décide du message utilisateur :

```powershell
# SEULEMENT si le status montre revisionStaged/revisionMerged non-zéro :
& $lore commit "p6: merge remote advance" --repository $main --json
```

Puis pousser et vérifier le retour à un état propre :

```powershell
& $lore push --repository $main --json
& $lore status --scan --repository $main --json
```

Attendu : push status 0, status final propre, `isLocalAhead`/`isRemoteAhead` false.

- [ ] **Step 6: Cleanup du clone temporaire**

```powershell
Remove-Item -Recurse -Force "C:\Users\jimmy\lore-tmp-p6-clone"
```

(Les fichiers `p6-remote.txt`/`p6-local.txt` restent dans le repo de test — même politique que `notify-test.txt` des lots précédents.)

- [ ] **Step 7: Pinner dans le README des fixtures**

Ajouter à la fin de `src-tauri/tests/fixtures/README.md` :

```markdown
**Push non-fast-forward** (fixture push_nonff.ndjson, capturée le 2026-07-11 en
fabriquant la course : second clone temp → commit+push distant → commit local →
push refusé) : <décrire ici la forme exacte constatée — événement `error` et la
sérialisation de son `data` telle que `check_ok` la renvoie, OU « pas d'événement
error, seulement complete status N »>. Sous-chaîne discriminante retenue pour le
matcher frontend (pushErrors.ts) : `<la sous-chaîne>`. `push --fast-forward-merge` :
<existe / n'existe pas — sortie de push --help>. Sync de rattrapage sans conflit :
<auto-committé / laisse un staged à committer>.
```

- [ ] **Step 8: Commit**

```bash
git add src-tauri/tests/fixtures/push_nonff.ndjson src-tauri/tests/fixtures/README.md
git commit -m "test(fixtures): capture non-fast-forward push refusal"
```

---

### Task 2: TDD — matcher pur `isNonFastForwardPush` pinné sur la capture

**Files:**
- Create: `src/lib/pushErrors.ts`
- Test: `src/lib/pushErrors.test.ts`

Deux variantes **exclusives** selon le constat (a) de la Task 1. Dans les deux cas : PAS de match large — un `Push failed` générique, un timeout, un « not authorized » ne doivent JAMAIS matcher.

- [ ] **Step 1: Écrire le test qui échoue**

**Variante 2A (événement `error` discriminant capturé)** — créer `src/lib/pushErrors.test.ts` :

```ts
import { describe, it, expect } from 'vitest'
import { isNonFastForwardPush, errorMessage, NON_FF_PUSH_SAMPLE } from './pushErrors'

describe('isNonFastForwardPush', () => {
  it('recognizes the captured non-fast-forward refusal', () => {
    // NON_FF_PUSH_SAMPLE est la copie verbatim du message que check_ok renvoie
    // pour la fixture push_nonff.ndjson (⚠ pin Task 1).
    expect(isNonFastForwardPush(NON_FF_PUSH_SAMPLE)).toBe(true)
  })
  it('rejects every other failure shape (no broad match)', () => {
    expect(isNonFastForwardPush('lore exited with status 1')).toBe(false)
    expect(isNonFastForwardPush('lore made no progress for 60 s — operation aborted')).toBe(false)
    expect(isNonFastForwardPush('failed to launch lore: program not found')).toBe(false)
    expect(isNonFastForwardPush('{"errorInner":"not authorized"}')).toBe(false)
    expect(isNonFastForwardPush('Push failed')).toBe(false)
    expect(isNonFastForwardPush('')).toBe(false)
  })
})

describe('errorMessage', () => {
  it('unwraps Error and stringifies the rest (tauri rejects with a plain string)', () => {
    expect(errorMessage(new Error('boom'))).toBe('boom')
    expect(errorMessage('plain string')).toBe('plain string')
    expect(errorMessage(42)).toBe('42')
  })
})
```

**Variante 2B (pas d'événement error — status seul)** — le message `lore exited with status N` n'est pas discriminant ; la détection devient message générique **ET** `remoteAhead > 0` (re-status silencieux au moment de l'échec) :

```ts
import { describe, it, expect } from 'vitest'
import { isNonFastForwardPush, errorMessage, NON_FF_PUSH_SAMPLE } from './pushErrors'

describe('isNonFastForwardPush (message + remoteAhead)', () => {
  it('recognizes a push failure while the remote is ahead', () => {
    expect(isNonFastForwardPush(NON_FF_PUSH_SAMPLE, 1)).toBe(true)
    expect(isNonFastForwardPush(NON_FF_PUSH_SAMPLE, 3)).toBe(true)
  })
  it('never fires when the remote is not ahead (real error, not a race)', () => {
    expect(isNonFastForwardPush(NON_FF_PUSH_SAMPLE, 0)).toBe(false)
  })
  it('never fires on non-CLI failures even with remoteAhead', () => {
    expect(isNonFastForwardPush('failed to launch lore: program not found', 2)).toBe(false)
    expect(isNonFastForwardPush('lore made no progress for 60 s — operation aborted', 2)).toBe(false)
  })
})

describe('errorMessage', () => {
  it('unwraps Error and stringifies the rest', () => {
    expect(errorMessage(new Error('boom'))).toBe('boom')
    expect(errorMessage('plain string')).toBe('plain string')
  })
})
```

- [ ] **Step 2: Vérifier l'échec**

Run: `npx vitest run src/lib/pushErrors.test.ts`
Expected: FAIL — `Cannot find module './pushErrors'`.

- [ ] **Step 3: Implémentation minimale**

**Variante 2A** — créer `src/lib/pushErrors.ts` :

```ts
/**
 * Détection du push refusé non-fast-forward (le remote a avancé).
 *
 * Le backend remonte l'événement `error` du CLI comme la sérialisation JSON de
 * son `data` (check_ok, src-tauri/src/lore.rs) ; tauri invoke rejette avec
 * cette string brute. Le matcher est PINNÉ sur la capture réelle
 * src-tauri/tests/fixtures/push_nonff.ndjson — pas de match large.
 */

/** Copie verbatim du message vu par le frontend pour la fixture push_nonff.
 *  ⚠ pin Task 1 : remplacer par la sérialisation réellement capturée. */
export const NON_FF_PUSH_SAMPLE = '{"errorInner":"remote is ahead — sync required"}' // ⚠ pin Task 1

/** Sous-chaîne discriminante du refus, extraite de NON_FF_PUSH_SAMPLE.
 *  ⚠ pin Task 1 : remplacer par la sous-chaîne réellement capturée. */
export const NON_FF_PUSH_MARKER = 'remote is ahead' // ⚠ pin Task 1

export function isNonFastForwardPush(message: string): boolean {
  return message.includes(NON_FF_PUSH_MARKER)
}

/** Tauri invoke rejette avec une string ; le mock jette un Error. */
export function errorMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e)
}
```

**Variante 2B** — créer `src/lib/pushErrors.ts` :

```ts
/**
 * Détection du push refusé non-fast-forward (le remote a avancé).
 *
 * Constat Task 1 : le CLI n'émet PAS d'événement `error` discriminant — le
 * frontend ne voit que « lore exited with status N ». La détection combine
 * donc l'échec CLI générique avec `remoteAhead > 0` re-lu au moment de
 * l'échec : un push qui échoue ALORS QUE le remote est en avance est le cas
 * non-fast-forward ; toute autre forme (launch failed, stall) est exclue.
 */

/** Copie verbatim du message vu par le frontend pour la fixture push_nonff.
 *  ⚠ pin Task 1. */
export const NON_FF_PUSH_SAMPLE = 'lore exited with status 1' // ⚠ pin Task 1

export function isNonFastForwardPush(message: string, remoteAhead: number): boolean {
  if (remoteAhead <= 0) return false
  // Uniquement l'échec CLI « propre » (status non-zéro) — jamais un problème
  // de lancement ni un stall, qui ont leurs propres messages.
  return message.includes('lore exited with status')
}

/** Tauri invoke rejette avec une string ; le mock jette un Error. */
export function errorMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e)
}
```

- [ ] **Step 4: Vérifier le pass**

Run: `npx vitest run src/lib/pushErrors.test.ts`
Expected: PASS (`Test Files  1 passed`).

- [ ] **Step 5: Commit**

```bash
git add src/lib/pushErrors.ts src/lib/pushErrors.test.ts
git commit -m "feat(push): pinned non-fast-forward push detector"
```

---

### Task 3: Toast « Remote has new changes » + enchaînement Sync & push

**Files:**
- Modify: `src/lib/repo.svelte.ts:126-164` (`act`, `push`) + nouvel export `syncAndPush`
- (Variante 3A uniquement) Modify: `src/lib/types.ts:161`, `src/lib/tauri.ts:82`, `src-tauri/src/commands.rs:789-797` (`lore_push`)

Socle commun aux deux variantes : `act` retourne un booléen de succès et accepte un intercepteur d'erreur (sync ou async — la variante 2B a besoin d'un re-status au moment de l'échec).

- [ ] **Step 1: Refondre `act` (commun aux deux variantes)**

Dans `src/lib/repo.svelte.ts`, remplacer la fonction `act` (lignes 126-141) par :

```ts
// Returns true when the action ran to completion (refresh included) — the
// Sync & push chain relies on it. `onError` may claim an error (resolve true)
// to replace the generic toastError with its own handling; it may be async
// (the non-FF detector may need a fresh status).
async function act(
  kind: 'commit' | 'push' | 'sync',
  run: (path: string) => Promise<void>,
  onError?: (e: unknown) => boolean | Promise<boolean>,
): Promise<boolean> {
  const path = session.config.currentRepo
  if (!path) return false
  repo.busy = kind
  try { await run(path) }
  catch (e) {
    if (!(await onError?.(e))) toastError(`${kind[0].toUpperCase()}${kind.slice(1)} failed`, e)
    repo.busy = ''
    return false
  }
  await refreshStatus()
  // commit/push/sync all change the history — refresh it in the background
  // (cached commits stay visible, no loading screen).
  refreshHistory(true)
  return true
}
```

Et ajouter l'import en tête de fichier (ligne 4, à côté de `toastError, toastAction`) :

```ts
import { isNonFastForwardPush, errorMessage } from './pushErrors'
```

- [ ] **Step 2: Câbler la détection dans `push` + l'action `syncAndPush`**

**Variante 3B (défaut — pas de `push --fast-forward-merge`) :** remplacer le bloc `push` (lignes 149-164 actuelles) par :

```ts
// Push, then offer to release the locks the user held on files that were part of
// this push (the lock-workflow's "done editing" step). The candidate set must be
// computed BEFORE the push, while the remote and local tips still differ.
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
}, (e) => {
  // Non-fast-forward refusal (the remote advanced under us): offer the
  // sync-then-push chain instead of a dead-end "Push failed".
  if (!isNonFastForwardPush(errorMessage(e))) return false
  toastAction('Remote has new changes', { label: 'Sync & push', run: () => { void syncAndPush() } })
  refreshStatus(true) // silent: surface remoteAhead in the title bar
  return true
})

// The "Sync & push" toast action: sync, then push — UNLESS the sync failed
// (its own toast already showed) or opened a merge/staged state (the merge
// chip takes over; NEVER auto-push mid-merge).
export async function syncAndPush() {
  if (!(await sync())) return
  if (repo.status?.mergeInProgress || repo.status?.stagedPending) return
  await push()
}
```

Si la Task 2 a retenu la **variante 2B** (signature `(message, remoteAhead)`), l'intercepteur devient async avec re-status :

```ts
}, async (e) => {
  const p = session.config.currentRepo
  if (!p) return false
  let remoteAhead = 0
  try { remoteAhead = (await api.getStatus(p)).remoteAhead } catch { return false }
  if (!isNonFastForwardPush(errorMessage(e), remoteAhead)) return false
  toastAction('Remote has new changes', { label: 'Sync & push', run: () => { void syncAndPush() } })
  refreshStatus(true)
  return true
})
```

**Variante 3A (EXCLUSIVE — `push --fast-forward-merge` constaté en Task 1)** : le CLI fait sync+push en un coup, pas de chaîne frontend.

`src/lib/types.ts` — remplacer la ligne `push(repoPath: string, onProgress?: (p: OpProgress) => void): Promise<void>` par :

```ts
  /** `fastForwardMerge` retente le push en absorbant l'avance distante (lore push --fast-forward-merge). */
  push(repoPath: string, onProgress?: (p: OpProgress) => void, fastForwardMerge?: boolean): Promise<void>
```

`src/lib/tauri.ts` — remplacer la ligne `push:` par :

```ts
  push: (repoPath, onProgress, fastForwardMerge) =>
    invokeWithProgress<void>('lore_push', { repoPath, fastForwardMerge: fastForwardMerge ?? false }, onProgress),
```

`src-tauri/src/commands.rs` — remplacer `lore_push` par :

```rust
#[tauri::command]
pub async fn lore_push(
    app: tauri::AppHandle,
    repo_path: String,
    op_id: Option<String>,
    fast_forward_merge: Option<bool>,
) -> Result<(), String> {
    blocking(move || {
        let op_id = op_id_or_default(op_id);
        let mut args: Vec<&str> = vec!["push"];
        if fast_forward_merge.unwrap_or(false) {
            args.push("--fast-forward-merge"); // ⚠ orthographe exacte pinnée Task 1 (push --help)
        }
        args.extend(["--repository", &repo_path]);
        run_lore_op(&app, "push", &op_id, &args)?;
        Ok(())
    })
    .await
}
```

`src/lib/repo.svelte.ts` — le push devient paramétrable et le toast relance `push(true)` :

```ts
export const push = (fastForwardMerge = false) => act('push', async (p) => {
  let candidates: string[] = []
  try { candidates = await api.pushedLockFiles(p) } catch { /* best-effort; never block the push */ }
  try { await api.push(p, (prog) => { opProgress.push = prog }, fastForwardMerge) }
  finally { opProgress.push = null }
  if (candidates.length) {
    const n = candidates.length
    toastAction(`${n} locked file${n > 1 ? 's' : ''} pushed`, {
      label: 'Release locks',
      run: () => releaseLocks(candidates),
    })
  }
}, (e) => {
  if (fastForwardMerge) return false // le retry a échoué à son tour (probable conflit) → toast d'erreur normal, le chip merge/status prendra le relais au refresh
  if (!isNonFastForwardPush(errorMessage(e))) return false
  toastAction('Remote has new changes', { label: 'Sync & push', run: () => { void push(true) } })
  refreshStatus(true)
  return true
})
```

(En 3A il n'y a pas de `syncAndPush` ; le mock Task 4 doit alors honorer le 3e paramètre — voir la note en Task 4. Vérifier que les appelants existants de `push()` — `TitleBar`/`Changes` — passent zéro argument : `git grep -n "push()" src` ; le défaut `false` les couvre.)

- [ ] **Step 3: Typecheck**

Run: `npm run check`
Expected: `svelte-check found 0 errors and 0 warnings`.

(Variante 3A seulement) Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: `test result: ok` (aucun test cassé par la signature).

- [ ] **Step 4: Commit**

```bash
git add src/lib/repo.svelte.ts src/lib/pushErrors.ts src/lib/types.ts src/lib/tauri.ts src-tauri/src/commands.rs
git commit -m "feat(push): Sync & push action toast on non-fast-forward refusal"
```

(Ne stager `types.ts`/`tauri.ts`/`commands.rs` que si la variante 3A les a touchés.)

---

### Task 4: Levier mock du push refusé + vérification navigateur

**Files:**
- Modify: `src/lib/mock.ts:278-291` (`push`, `sync`)
- Test: `src/lib/mock.test.ts` (nouveau bloc `describe`)

- [ ] **Step 1: Écrire les tests qui échouent**

Ajouter en tête de `src/lib/mock.test.ts` (après les imports existants) :

```ts
import { isNonFastForwardPush, errorMessage } from './pushErrors'
```

et à la fin du fichier :

```ts
describe('mock push non-fast-forward lever', () => {
  it('push rejects with the pinned refusal while the lever is set', async () => {
    localStorage.setItem('loredesktop.mock.pushNonFF', '1')
    const err = await mock.push('C:/repos/nff').catch((e) => e)
    expect(err).toBeInstanceOf(Error)
    expect(isNonFastForwardPush(errorMessage(err))).toBe(true)
  })
  it('a successful sync clears the lever, then push succeeds', async () => {
    localStorage.setItem('loredesktop.mock.pushNonFF', '1')
    await mock.sync('C:/repos/nff')
    expect(localStorage.getItem('loredesktop.mock.pushNonFF')).toBeNull()
    await expect(mock.push('C:/repos/nff')).resolves.toBeUndefined()
  })
})
```

(Si la Task 2 a retenu la **variante 2B**, le premier `expect` devient `expect(isNonFastForwardPush(errorMessage(err), 1)).toBe(true)` — le mock garde `remoteAhead: 1` par défaut, cf. `stateFor`.)

- [ ] **Step 2: Vérifier l'échec**

Run: `npx vitest run src/lib/mock.test.ts`
Expected: FAIL — le push mock résout au lieu de rejeter.

- [ ] **Step 3: Implémenter le levier**

Dans `src/lib/mock.ts`, ajouter l'import (ligne 1, à côté des imports existants) :

```ts
import { NON_FF_PUSH_SAMPLE } from './pushErrors'
```

au niveau module (sous `const AUTH_KEY = …`) :

```ts
// Dev lever: simulate a push refused because the remote advanced —
// `localStorage.setItem('loredesktop.mock.pushNonFF', '1')` in the devtools.
// Push throws the pinned refusal (same message as the captured fixture, so the
// matcher exercises the real path); a successful sync clears the lever
// (the remote is caught up).
const PUSH_NONFF_KEY = 'loredesktop.mock.pushNonFF'
```

et remplacer `push` / `sync` par :

```ts
  async push(repoPath: string, onProgress?: (p: OpProgress) => void) {
    if (localStorage.getItem(PUSH_NONFF_KEY) === '1') {
      await delay(250)
      throw new Error(NON_FF_PUSH_SAMPLE)
    }
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
    localStorage.removeItem(PUSH_NONFF_KEY)
  },
```

(**Variante 3A seulement** : la signature devient `async push(repoPath, onProgress?, fastForwardMerge?)` et le refus n'a lieu que `if (!fastForwardMerge && localStorage.getItem(PUSH_NONFF_KEY) === '1')` ; un push `fastForwardMerge` réussi fait en plus `stateFor(repoPath).remoteAhead = 0; localStorage.removeItem(PUSH_NONFF_KEY)`.)

- [ ] **Step 4: Vérifier le pass + suite complète**

Run: `npx vitest run src/lib/mock.test.ts` → PASS.
Run: `npx vitest run` → tous les fichiers passent (le test existant `commit clears files and bumps ahead; push zeroes ahead` doit rester vert — le levier est absent par défaut, et `beforeEach` fait `localStorage.clear()`).

- [ ] **Step 5: Vérification navigateur du flux complet**

`npm run dev` → http://localhost:5173 (se signer/ouvrir un repo mock si besoin). Dans la devtools : `localStorage.setItem('loredesktop.mock.pushNonFF', '1')`. Committer un changement (bouton Commit), puis **Push** → attendu : PAS de toast rouge « Push failed », mais un toast accent « Remote has new changes » avec bouton **Sync & push**. Cliquer → la barre de sync progresse, puis le push part et réussit (le sync mock a levé le levier), le titre revient à un état propre. Vérifier aussi le négatif : sans levier, Push fonctionne comme avant.

- [ ] **Step 6: Commit**

```bash
git add src/lib/mock.ts src/lib/mock.test.ts
git commit -m "feat(mock): non-fast-forward push lever for the Sync & push flow"
```

---

# Item 1 — Shared store au clone (Tasks 5–8)

### Task 5: DISCOVERY — sémantique `lore shared-store` + capture

**Files:**
- Create: `src-tauri/tests/fixtures/shared_store_info.ndjson` (+ `shared_store_info_none.ndjson` si exploitable)
- Modify: `src-tauri/tests/fixtures/README.md` (section « Shared store »)

Le CLI n'a jamais été capturé sur ce sujet — RIEN de la Task 6+ ne s'exécute avant cette discovery.

- [ ] **Step 1: Cartographier les sous-commandes**

```powershell
$lore = "C:\Users\jimmy\bin\lore.exe"
& $lore shared-store --help
& $lore shared-store create --help
& $lore shared-store info --help
& $lore shared-store set-use-automatically --help
& $lore clone --help
```

Noter : les arguments exacts de `create` (chemin optionnel ? emplacement par défaut ?), de `set-use-automatically` (booléen ? `true/false` ? `on/off` ? sans argument ?), et **si `clone` expose un flag shared-store** (c'est LE constat qui départage la variante A de la B en Task 8).

- [ ] **Step 2: Capturer `info` quand AUCUN store n'existe**

```powershell
$fix = "C:\Users\jimmy\Documents\SoonerOrLater\lore-desktop\src-tauri\tests\fixtures"
& $lore shared-store info --json | Out-File -Encoding utf8 "$fix\shared_store_info_none.ndjson"
"exit=$LASTEXITCODE"
Get-Content "$fix\shared_store_info_none.ndjson"
```

Noter : status 0 avec un événement « vide », ou status non-zéro ? (La Task 6 traite le non-zéro comme « pas de store », pas comme une erreur.) Si le fichier est inexploitable (status non-zéro sans événement), le supprimer et le noter au README — le test Rust du cas « none » restera synthétique.

- [ ] **Step 3: Créer le store réel + capturer `info` avec store**

```powershell
& $lore shared-store create --json          # + chemin/options selon le --help du Step 1
& $lore shared-store info --json | Out-File -Encoding utf8 "$fix\shared_store_info.ndjson"
Get-Content "$fix\shared_store_info.ndjson"
```

Noter : le **tagName** et les **champs** de l'événement d'info (hypothèse de la Task 6 : `sharedStoreInfo { path }` — à corriger au constat), l'emplacement disque réel du store, et si l'info expose l'état « use automatically » (champ à reporter dans le DTO, cf. Task 6). Le store créé RESTE en place (c'est la machine de dev, la feature s'en servira) — le documenter.

- [ ] **Step 4: Essai réel — comment un clone « utilise » le store**

```powershell
& $lore shared-store set-use-automatically true --json    # args exacts du Step 1
$tmp2 = "C:\Users\jimmy\lore-tmp-p6-store-clone"
if (Test-Path $tmp2) { Remove-Item -Recurse -Force $tmp2 }
& $lore clone "lore://lore.example.com:41337/019f333af5e073d28bb117ad1596784a" $tmp2 --json
# Le clone utilise-t-il le store ? Vérifier la structure disque :
Get-ChildItem -Recurse -Force $tmp2\.lore | Select-Object FullName -First 30
& $lore shared-store info --json
Remove-Item -Recurse -Force $tmp2
```

**Constat décisif à écrire au README** : la mécanique est-elle **(A) par-clone** (flag de `clone`, ou activation ponctuelle avant chaque clone) ou **(B) globale une-fois** (`set-use-automatically` persiste pour tous les clones futurs) ? Comment un clone matérialise l'usage du store (lien/référence dans `.lore/`) ?

- [ ] **Step 5: Pinner au README**

Ajouter à `src-tauri/tests/fixtures/README.md` :

```markdown
**Shared store** (fixtures shared_store_info.ndjson / shared_store_info_none.ndjson,
capturées le 2026-07-11) : `shared-store info --json` émet <tagName + champs exacts,
dont le chemin du store et, s'il existe, l'état use-automatically>. Sans store :
<status 0 événement vide / status non-zéro — préciser>. Création : `shared-store
create <args exacts>` ; store créé dans <emplacement>. Activation pour les clones :
<mécanique constatée — flag de clone `<flag>` / réglage global `set-use-automatically
<args>` persistant>. Décision UI (spec item 1) : variante <A — case par-clone dans le
flux Clone / B — toggle global AvatarMenu>.
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/tests/fixtures/shared_store_info.ndjson src-tauri/tests/fixtures/shared_store_info_none.ndjson src-tauri/tests/fixtures/README.md
git commit -m "test(fixtures): capture shared-store info + pin activation semantics"
```

(Omettre `shared_store_info_none.ndjson` du `git add` s'il a été jugé inexploitable au Step 2.)

---

### Task 6: TDD Rust — `lore_shared_store_status` / `enable` / `disable`

**Files:**
- Modify: `src-tauri/src/commands.rs` (nouvelles commandes + tests, à placer après `lore_clone`)
- Modify: `src-tauri/src/lib.rs:28-69` (enregistrement)

- [ ] **Step 1: Écrire les tests qui échouent**

Ajouter dans `src-tauri/src/commands.rs` :

```rust
#[cfg(test)]
mod shared_store_tests {
    use super::*;
    use crate::lore::parse_events;

    #[test]
    fn parses_existing_store_fixture() {
        let events = parse_events(include_str!("../tests/fixtures/shared_store_info.ndjson")).unwrap();
        let s = shared_store_status_from(&events);
        assert!(s.exists, "the captured fixture has a store");
        assert!(s.path.as_deref().is_some_and(|p| !p.is_empty()), "path was {:?}", s.path);
    }

    #[test]
    fn no_info_event_means_no_store() {
        let events = parse_events(r#"{"tagName":"complete","data":{"status":0}}"#).unwrap();
        let s = shared_store_status_from(&events);
        assert!(!s.exists);
        assert_eq!(s.path, None);
    }
}
```

(Si `shared_store_info_none.ndjson` est exploitable — status 0 —, ajouter un troisième test identique au second mais sur `include_str!("../tests/fixtures/shared_store_info_none.ndjson")`.)

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test --manifest-path src-tauri/Cargo.toml shared_store`
Expected: erreur de compilation — `shared_store_status_from` inexistant.

- [ ] **Step 3: Implémenter**

Ajouter dans `src-tauri/src/commands.rs` (après `lore_clone`) — **⚠ tag `sharedStoreInfo`, champs `path`/`useAutomatically`, et sous-commandes/args sont les hypothèses à remplacer par le constat Task 5** :

```rust
#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SharedStoreStatusDto {
    pub exists: bool,
    pub path: Option<String>,
    /// Whether clones use the store automatically, when the CLI reports it.
    pub auto_use: Option<bool>,
}

/// Parse `lore shared-store info --json`. Tag + field names pinned against
/// tests/fixtures/shared_store_info.ndjson (Task 5). ⚠ pin Task 5.
fn shared_store_status_from(events: &[LoreEvent]) -> SharedStoreStatusDto {
    let info = events_with_tag(events, "sharedStoreInfo").into_iter().next(); // ⚠ pin Task 5
    let path = info
        .and_then(|d| d.get("path").and_then(|v| v.as_str())) // ⚠ pin Task 5
        .filter(|p| !p.is_empty())
        .map(String::from);
    let auto_use = info.and_then(|d| d.get("useAutomatically").map(json_truthy)); // ⚠ pin Task 5
    SharedStoreStatusDto { exists: path.is_some(), path, auto_use }
}

/// Whether a shared object store exists on this machine (and where). A CLI
/// error here means "no store yet" (pinned in Task 5) — a normal answer, not
/// a failure to surface.
#[tauri::command]
pub async fn lore_shared_store_status() -> Result<SharedStoreStatusDto, String> {
    blocking(move || {
        match run_lore(&["shared-store", "info"]) {
            Ok(events) => Ok(shared_store_status_from(&events)),
            Err(_) => Ok(SharedStoreStatusDto { exists: false, path: None, auto_use: None }),
        }
    })
    .await
}

/// Enable the shared store for clones: create it if none exists, then turn on
/// automatic use. Exact subcommands/args pinned by the Task 5 capture.
#[tauri::command]
pub async fn lore_shared_store_enable() -> Result<(), String> {
    blocking(move || {
        let exists = run_lore(&["shared-store", "info"])
            .map(|ev| shared_store_status_from(&ev).exists)
            .unwrap_or(false);
        if !exists {
            run_lore(&["shared-store", "create"])?; // ⚠ args exacts pinnés Task 5
        }
        run_lore(&["shared-store", "set-use-automatically", "true"])?; // ⚠ args exacts pinnés Task 5
        Ok(())
    })
    .await
}

/// Turn off automatic shared-store use (the store itself is kept on disk).
#[tauri::command]
pub async fn lore_shared_store_disable() -> Result<(), String> {
    blocking(move || {
        run_lore(&["shared-store", "set-use-automatically", "false"])?; // ⚠ args exacts pinnés Task 5
        Ok(())
    })
    .await
}
```

Dans `src-tauri/src/lib.rs`, ajouter au `generate_handler!` (après `commands::lore_clone,`) :

```rust
        commands::lore_shared_store_status,
        commands::lore_shared_store_enable,
        commands::lore_shared_store_disable,
```

- [ ] **Step 4: Vérifier le pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml shared_store`
Expected: `test result: ok` (2 ou 3 tests).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(store): shared-store status/enable/disable commands"
```

---

### Task 7: TDD — helper pur de la case + API front (types/tauri/mock)

**Files:**
- Create: `src/lib/sharedStore.ts`
- Test: `src/lib/sharedStore.test.ts`
- Modify: `src/lib/types.ts` (interface + LoreApi), `src/lib/tauri.ts`, `src/lib/mock.ts`, `src/lib/mock.test.ts`

- [ ] **Step 1: Écrire les tests qui échouent**

Créer `src/lib/sharedStore.test.ts` :

```ts
import { describe, it, expect } from 'vitest'
import { sharedStoreDefaultChecked, sharedStoreHint } from './sharedStore'

describe('sharedStoreDefaultChecked', () => {
  it('is checked when a store already exists (spec item 1)', () => {
    expect(sharedStoreDefaultChecked({ exists: true, path: 'C:/store' })).toBe(true)
  })
  it('is unchecked when none exists or status is unknown', () => {
    expect(sharedStoreDefaultChecked({ exists: false, path: null })).toBe(false)
    expect(sharedStoreDefaultChecked(null)).toBe(false)
  })
})

describe('sharedStoreHint', () => {
  it('mentions creation when no store exists yet', () => {
    expect(sharedStoreHint({ exists: false, path: null })).toContain('Creates')
    expect(sharedStoreHint(null)).toContain('Creates')
  })
  it('mentions reuse — with the path when known', () => {
    expect(sharedStoreHint({ exists: true, path: 'C:/store' })).toContain('C:/store')
    expect(sharedStoreHint({ exists: true, path: null })).toContain('Reuses')
  })
})
```

Ajouter à la fin de `src/lib/mock.test.ts` :

```ts
describe('mock shared store', () => {
  it('status flips with enable/disable and clones remember a store exists', async () => {
    expect((await mock.sharedStoreStatus()).exists).toBe(false)
    await mock.sharedStoreEnable()
    const on = await mock.sharedStoreStatus()
    expect(on.exists).toBe(true)
    expect(on.path).toBeTruthy()
    await mock.sharedStoreDisable()
    expect((await mock.sharedStoreStatus()).exists).toBe(false)
  })
})
```

- [ ] **Step 2: Vérifier l'échec**

Run: `npx vitest run src/lib/sharedStore.test.ts src/lib/mock.test.ts`
Expected: FAIL (module et méthodes inexistants).

- [ ] **Step 3: Implémenter**

`src/lib/types.ts` — ajouter après l'interface `RepoEntry` :

```ts
export interface SharedStoreStatus {
  exists: boolean
  path: string | null
  /** Whether clones use the store automatically (when the CLI reports it). */
  autoUse?: boolean
}
```

et dans `LoreApi`, après la ligne `cloneRepo(…)` :

```ts
  /** Whether a shared object store exists on this machine (and where). */
  sharedStoreStatus(): Promise<SharedStoreStatus>
  /** Create the store if needed, then enable automatic use for clones. */
  sharedStoreEnable(): Promise<void>
  /** Turn off automatic shared-store use (the store itself is kept). */
  sharedStoreDisable(): Promise<void>
```

`src/lib/tauri.ts` — ajouter `SharedStoreStatus` à l'import de types (ligne 5) et, après `cloneRepo:` :

```ts
  sharedStoreStatus: () => invoke<SharedStoreStatus>('lore_shared_store_status'),
  sharedStoreEnable: () => invoke<void>('lore_shared_store_enable'),
  sharedStoreDisable: () => invoke<void>('lore_shared_store_disable'),
```

`src/lib/mock.ts` — au niveau module (sous `const PUSH_NONFF_KEY = …`) :

```ts
// Dev lever: `localStorage.setItem('loredesktop.mock.sharedStore', '1')`
// simulates an existing shared store (the checkbox then defaults to checked).
const SHARED_STORE_KEY = 'loredesktop.mock.sharedStore'
```

et dans l'objet `mock` (après `cloneRepo`) :

```ts
  async sharedStoreStatus() {
    await delay(120)
    const on = localStorage.getItem(SHARED_STORE_KEY) === '1'
    return { exists: on, path: on ? 'C:/Users/jimmy/.lore/shared-store' : null, autoUse: on }
  },
  async sharedStoreEnable() {
    await delay(200)
    localStorage.setItem(SHARED_STORE_KEY, '1')
  },
  async sharedStoreDisable() {
    await delay(200)
    localStorage.removeItem(SHARED_STORE_KEY)
  },
```

Créer `src/lib/sharedStore.ts` :

```ts
import type { SharedStoreStatus } from './types'

/** Checked by default only when a store already exists (spec item 1). */
export function sharedStoreDefaultChecked(status: SharedStoreStatus | null): boolean {
  return status?.exists ?? false
}

/** Hint under the checkbox: reuse vs first-time creation. */
export function sharedStoreHint(status: SharedStoreStatus | null): string {
  if (status?.exists)
    return status.path ? `Reuses the shared object store (${status.path})` : 'Reuses the shared object store'
  return 'Creates a shared object store — clones share disk objects'
}
```

- [ ] **Step 4: Vérifier le pass + typecheck**

Run: `npx vitest run src/lib/sharedStore.test.ts src/lib/mock.test.ts` → PASS.
Run: `npm run check` → 0 erreur 0 warning.

- [ ] **Step 5: Commit**

```bash
git add src/lib/sharedStore.ts src/lib/sharedStore.test.ts src/lib/types.ts src/lib/tauri.ts src/lib/mock.ts src/lib/mock.test.ts
git commit -m "feat(store): shared-store API surface + checkbox helpers"
```

---

### Task 8: UI shared store — variante A (case au clone) OU variante B (toggle global)

**Files (variante A):**
- Modify: `src/lib/types.ts` (`cloneRepo`), `src/lib/tauri.ts` (`cloneRepo`), `src/lib/mock.ts` (`cloneRepo`), `src-tauri/src/commands.rs` (`lore_clone`), `src/lib/repoActions.ts` (`cloneServerRepo`), `src/lib/RepoPicker.svelte`, `src/lib/RepoSwitcher.svelte`

**Files (variante B):**
- Modify: `src/lib/AvatarMenu.svelte`

Le choix A/B est le constat décisif de la Task 5 : **A** si la mécanique est par-clone (flag de `clone` ou activation ponctuelle), **B** si `set-use-automatically` est un réglage global une-fois. N'implémenter QU'UNE variante.

- [ ] **Step 1 (variante A): Propager `useSharedStore` jusqu'au backend**

`src/lib/types.ts` — remplacer la déclaration `cloneRepo` de `LoreApi` par :

```ts
  /** Clone <serverUrl>/<repoId> into <destParent>/<repoName>; returns the created path. Progress ticks stream via onProgress. `useSharedStore` clones against the shared object store (created on demand). */
  cloneRepo(serverUrl: string, repoId: string, repoName: string, destParent: string, onProgress?: (p: OpProgress) => void, useSharedStore?: boolean): Promise<string>
```

`src/lib/tauri.ts` — remplacer `cloneRepo:` par :

```ts
  cloneRepo: (serverUrl, repoId, repoName, destParent, onProgress, useSharedStore) =>
    invokeWithProgress<string>('lore_clone', { serverUrl, repoId, repoName, destParent, useSharedStore: useSharedStore ?? false }, onProgress),
```

`src/lib/mock.ts` — remplacer la signature de `cloneRepo` par :

```ts
  async cloneRepo(_serverUrl: string, _repoId: string, repoName: string, destParent: string, onProgress?: (p: OpProgress) => void, useSharedStore?: boolean) {
    if (useSharedStore) localStorage.setItem(SHARED_STORE_KEY, '1') // a store now exists — next clone defaults checked
    // Simulated determinate transfer so the clone progress bar lives in dev.
    const total = 48 * 1024 * 1024
    for (let i = 1; i <= 12; i++) {
      await delay(90)
      onProgress?.({ done: Math.round((total * i) / 12), total, unit: 'bytes' })
    }
    return `${destParent}/${repoName}`
  },
```

`src-tauri/src/commands.rs` — remplacer `lore_clone` par (⚠ la mécanique exacte — flag de clone `--use-shared-store` OU enable préalable — est celle pinnée en Task 5 ; les DEUX écritures sont données, garder la bonne) :

```rust
#[tauri::command]
pub async fn lore_clone(
    app: tauri::AppHandle,
    server_url: String,
    repo_id: String,
    repo_name: String,
    dest_parent: String,
    op_id: Option<String>,
    use_shared_store: Option<bool>,
) -> Result<String, String> {
    blocking(move || {
        let op_id = op_id_or_default(op_id);
        let (url, path) = build_clone_args(&server_url, &repo_id, &repo_name, &dest_parent);
        let shared = use_shared_store.unwrap_or(false);
        // Mécanique A1 (flag de clone constaté en Task 5) :
        let mut args: Vec<&str> = vec!["clone", &url, &path];
        if shared {
            // Le store doit exister avant le clone — création à la demande.
            let exists = run_lore(&["shared-store", "info"])
                .map(|ev| shared_store_status_from(&ev).exists)
                .unwrap_or(false);
            if !exists {
                run_lore(&["shared-store", "create"])?; // ⚠ args exacts pinnés Task 5
            }
            args.push("--use-shared-store"); // ⚠ flag exact pinné Task 5
        }
        run_lore_op(&app, "clone", &op_id, &args)?;
        Ok(path)
    })
    .await
}
```

Mécanique A2 (PAS de flag de clone, mais activation ponctuelle constatée par-clone) — même signature, corps du `if shared` remplacé par un appel à la même séquence que `lore_shared_store_enable` (create si besoin + `set-use-automatically true`), sans flag ajouté à `args` :

```rust
        let mut args: Vec<&str> = vec!["clone", &url, &path];
        if shared {
            let exists = run_lore(&["shared-store", "info"])
                .map(|ev| shared_store_status_from(&ev).exists)
                .unwrap_or(false);
            if !exists {
                run_lore(&["shared-store", "create"])?; // ⚠ pin Task 5
            }
            run_lore(&["shared-store", "set-use-automatically", "true"])?; // ⚠ pin Task 5
        }
        run_lore_op(&app, "clone", &op_id, &args)?;
```

`src/lib/repoActions.ts` — remplacer `cloneServerRepo` par :

```ts
export async function cloneServerRepo(entry: RepoEntry, useSharedStore = false): Promise<boolean> {
  if (cloneInFlight(opProgress.clone)) return false
  opProgress.clone = { done: 0 } // indeterminate until the first real tick
  try {
    const parent = await api.pickFolder()
    if (!parent) return false // cancelled
    const path = await api.cloneRepo(
      session.config.serverUrl!, entry.id, entry.name, parent,
      (p) => { opProgress.clone = p },
      useSharedStore,
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

- [ ] **Step 2 (variante A): Case dans RepoPicker**

Dans `src/lib/RepoPicker.svelte` — imports (compléter les lignes existantes) :

```ts
import { sharedStoreDefaultChecked, sharedStoreHint } from './sharedStore'
import type { RepoEntry, SharedStoreStatus } from './types'
```

états (sous `let busy = $state('')`) :

```ts
let sharedStore = $state<SharedStoreStatus | null>(null)
let useSharedStore = $state(false)

async function loadSharedStore() {
  try {
    sharedStore = await api.sharedStoreStatus()
    useSharedStore = sharedStoreDefaultChecked(sharedStore)
  } catch { /* best-effort: la case reste décochée, hint « creates » */ }
}
```

remplacer `$effect(() => { loadRepos() })` par :

```ts
$effect(() => { loadRepos(); loadSharedStore() })
```

remplacer l'appel `await cloneServerRepo(entry)` (dans `cloneRepo`) par :

```ts
      await cloneServerRepo(entry, useSharedStore)
```

markup — insérer juste après `<h3>On {session.config.serverUrl}</h3>` :

```svelte
  <label class="store">
    <input type="checkbox" bind:checked={useSharedStore} />
    <span>Use shared store</span>
    <span class="muted small">{sharedStoreHint(sharedStore)}</span>
  </label>
```

CSS (à la fin du `<style>`) :

```css
  .store { display: flex; align-items: center; gap: 7px; margin: 2px 0 6px; font-size: 12px; }
  .store input { accent-color: var(--accent); }
```

- [ ] **Step 3 (variante A): Case dans RepoSwitcher (mode clone)**

Dans `src/lib/RepoSwitcher.svelte` — imports :

```ts
import { sharedStoreDefaultChecked, sharedStoreHint } from './sharedStore'
import type { RepoEntry, SharedStoreStatus } from './types'
```

états (sous `let busy = $state('')`) :

```ts
let sharedStore = $state<SharedStoreStatus | null>(null)
let useSharedStore = $state(false)
```

remplacer `enterClone` par :

```ts
  async function enterClone() {
    addOpen = false
    mode = 'clone'
    loading = true
    try {
      serverRepos = await api.listRepos(session.config.serverUrl!)
    } catch (e) {
      toastError("Couldn't list repositories", e)
    } finally {
      loading = false
    }
    try {
      sharedStore = await api.sharedStoreStatus()
      useSharedStore = sharedStoreDefaultChecked(sharedStore)
    } catch { /* best-effort: case décochée */ }
  }
```

remplacer `await cloneServerRepo(entry)` (dans `onClone`) par `await cloneServerRepo(entry, useSharedStore)`.

markup — insérer juste après `<div class="sec">Clone from {session.config.serverUrl}</div>` (le menu fait 320 px : hint complet en tooltip) :

```svelte
    <label class="store" title={sharedStoreHint(sharedStore)}>
      <input type="checkbox" bind:checked={useSharedStore} />
      <span>Use shared store</span>
    </label>
```

CSS (à la fin du `<style>`) :

```css
  .store { display: flex; align-items: center; gap: 7px; margin: 0 12px 6px; font-size: 12px; color: var(--text); }
  .store input { accent-color: var(--accent); }
```

- [ ] **Step 1-bis (variante B, EXCLUSIVE): toggle global dans AvatarMenu**

Ne toucher NI `cloneRepo` NI les composants de clone. Dans `src/lib/AvatarMenu.svelte` — imports :

```ts
import { api } from './api'
import { toastError } from './toast'
```

états + logique (sous `let name = $state(…)`) :

```ts
  // null = statut pas encore chargé (toggle désactivé en attendant).
  let storeOn = $state<boolean | null>(null)

  $effect(() => {
    api.sharedStoreStatus()
      .then((s) => { storeOn = s.autoUse ?? s.exists })
      .catch(() => { storeOn = null })
  })

  async function toggleStore() {
    if (storeOn === null) return
    const target = !storeOn
    const prev = storeOn
    storeOn = target // optimiste — revert en cas d'échec
    try {
      if (target) await api.sharedStoreEnable()
      else await api.sharedStoreDisable()
    } catch (e) {
      storeOn = prev
      toastError(target ? "Couldn't enable the shared store" : "Couldn't disable the shared store", e)
    }
  }
```

markup — insérer entre `</div>` du `.field` et `<div class="div"></div>` :

```svelte
  <label class="storetoggle">
    <input type="checkbox" checked={storeOn === true} disabled={storeOn === null} onchange={toggleStore} />
    <span>Use shared store for clones</span>
  </label>
```

CSS (à la fin du `<style>`) :

```css
  .storetoggle { display: flex; align-items: center; gap: 8px; padding: 0 14px 10px; font-size: 12px; color: var(--text); }
  .storetoggle input { accent-color: var(--accent); }
```

- [ ] **Step 4: Typecheck + suites**

Run: `npm run check` → 0 erreur 0 warning.
Run: `npx vitest run` → tout passe (variante A : le test mock `pickFolder returns a path; cloneRepo returns dest/name` reste vert, le nouveau paramètre est optionnel).
(Variante A) Run: `cargo test --manifest-path src-tauri/Cargo.toml` → `test result: ok`.

- [ ] **Step 5: Vérification navigateur (mock)**

`npm run dev`. **Variante A** : ouvrir le RepoPicker (ou Add → Clone repository… du switcher) → case « Use shared store » **décochée** avec hint « Creates a shared object store… ». Cocher, cloner `game-assets` → l'app bascule sur le repo. Rouvrir le flux clone → la case est maintenant **cochée par défaut** avec le hint « Reuses… (C:/Users/jimmy/.lore/shared-store) » (le mock a mémorisé le store). `localStorage.removeItem('loredesktop.mock.sharedStore')` remet l'état initial. **Variante B** : ouvrir l'AvatarMenu → toggle « Use shared store for clones » off ; l'activer → persiste après réouverture du menu ; le désactiver → revient off.

- [ ] **Step 6: Commit**

```bash
git add -A src/lib src-tauri/src/commands.rs
git commit -m "feat(store): shared-store option in the clone flow"
```

(Variante B : message `feat(store): global shared-store toggle in the avatar menu`.)

---

# Item 3 — Créer un repository depuis l'app — ⚠ GATED (Tasks 9–12)

> **⚠ GATED : n'exécuter les Tasks 9–12 QUE si Jimmy a explicitement arbitré OUI.**
> Sans arbitrage écrit dans la conversation d'exécution, sauter à la Task 13.
> Le plan existe pour ne pas refaire le design le jour venu (spec item 3).

### Task 9 (GATED): DISCOVERY — `lore repository create` réel

**Files:**
- Create: `src-tauri/tests/fixtures/repo_create.ndjson`
- Modify: `src-tauri/tests/fixtures/README.md` (section « Repository create »)

- [ ] **Step 1: Cartographier**

```powershell
$lore = "C:\Users\jimmy\bin\lore.exe"
$srv  = "lore://lore.example.com:41337"
& $lore repository --help
& $lore repository create --help
```

Noter : la forme exacte (`repository create <server> <name>` ? `--name` ? crée-t-il aussi une working copy locale si on passe un chemin ?) et si `repository delete` existe.

- [ ] **Step 2: Créer réellement + capturer**

```powershell
$fix  = "C:\Users\jimmy\Documents\SoonerOrLater\lore-desktop\src-tauri\tests\fixtures"
$name = "desktoptest-created-$(Get-Date -Format yyyyMMdd)"
& $lore repository create $srv $name --json | Out-File -Encoding utf8 "$fix\repo_create.ndjson"   # forme exacte selon le --help
"exit=$LASTEXITCODE"
Get-Content "$fix\repo_create.ndjson"
& $lore repository list $srv --json
```

Attendu : status 0 ; le nouveau repo apparaît dans `repository list`. Noter : le **tagName/champ portant l'id** du repo créé (hypothèse Task 11 : `repositoryCreate { id }`) — s'il n'y a PAS d'id dans la sortie, la Task 11 a déjà le fallback re-list (le noter au README).

- [ ] **Step 3: Charset des noms**

```powershell
& $lore repository create $srv "Invalid Name!" --json
"exit=$LASTEXITCODE"
& $lore repository create $srv "UPPERCASE-test" --json
"exit=$LASTEXITCODE"
```

Noter ce que le serveur accepte/refuse (majuscules ? espaces ? tirets/underscores ? longueur ?) → pinne `REPO_NAME_RE` de la Task 10. **Si un de ces essais réussit**, supprimer ce repo parasite au Step 4 (ou le signaler).

- [ ] **Step 4: Nettoyage**

```powershell
& $lore repository delete --help
# Si delete existe :
& $lore repository delete $srv $name --json          # + les éventuels repos des essais charset
& $lore repository list $srv --json                   # vérifier la disparition
```

Si `repository delete` n'existe pas : le repo `desktoptest-created-<date>` **reste sur le serveur — le signaler explicitement dans le rapport final ET au README**.

- [ ] **Step 5: Pinner au README**

Ajouter à `src-tauri/tests/fixtures/README.md` :

```markdown
**Repository create** (fixture repo_create.ndjson, capturée le <date>) : forme
`repository create <forme exacte>` ; l'id du repo créé est porté par <tagName/champ,
ou « absent — fallback re-list par nom »>. Charset accepté pour les noms : <constat
des essais — regex retenue pour repoName.ts>. `repository delete` : <existe (cleanup
fait) / n'existe pas — repo de test permanent `desktoptest-created-<date>` laissé sur
le serveur>.
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/tests/fixtures/repo_create.ndjson src-tauri/tests/fixtures/README.md
git commit -m "test(fixtures): capture repository create + pin name charset"
```

---

### Task 10 (GATED): TDD — validation du nom de repo

**Files:**
- Create: `src/lib/repoName.ts`
- Test: `src/lib/repoName.test.ts`

- [ ] **Step 1: Écrire les tests qui échouent**

Créer `src/lib/repoName.test.ts` (⚠ ajuster les cas au charset pinné Task 9 — ex. si les majuscules sont acceptées, retirer le cas `UPPER`) :

```ts
import { describe, it, expect } from 'vitest'
import { validateRepoName } from './repoName'

describe('validateRepoName', () => {
  it('accepts simple names', () => {
    expect(validateRepoName('game-assets')).toBeNull()
    expect(validateRepoName('audio_2')).toBeNull()
    expect(validateRepoName('  padded  ')).toBeNull() // trimmed before validation
  })
  it('rejects empty / whitespace-only', () => {
    expect(validateRepoName('')).not.toBeNull()
    expect(validateRepoName('   ')).not.toBeNull()
  })
  it('rejects the charset violations pinned by the Task 9 discovery', () => {
    expect(validateRepoName('has space')).not.toBeNull()
    expect(validateRepoName('UPPER')).not.toBeNull()      // ⚠ pin Task 9
    expect(validateRepoName('-starts-dash')).not.toBeNull()
    expect(validateRepoName('a'.repeat(65))).not.toBeNull()
  })
})
```

- [ ] **Step 2: Vérifier l'échec**

Run: `npx vitest run src/lib/repoName.test.ts`
Expected: FAIL — module inexistant.

- [ ] **Step 3: Implémenter**

Créer `src/lib/repoName.ts` :

```ts
/**
 * Validation du nom d'un nouveau repository serveur. Charset PINNÉ par la
 * discovery Task 9 (essais réels contre le serveur) — ⚠ remplacer la regex
 * par le constat. Retourne le message d'erreur à afficher, ou null si valide.
 */
export const REPO_NAME_RE = /^[a-z0-9][a-z0-9_-]{0,63}$/ // ⚠ pin Task 9

export function validateRepoName(name: string): string | null {
  const n = name.trim()
  if (!n) return 'Name is required'
  if (!REPO_NAME_RE.test(n))
    return 'Use lowercase letters, digits, "-" or "_" (max 64, must start with a letter or digit)' // ⚠ reformuler selon la regex pinnée
  return null
}
```

- [ ] **Step 4: Vérifier le pass**

Run: `npx vitest run src/lib/repoName.test.ts` → PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/repoName.ts src/lib/repoName.test.ts
git commit -m "feat(repo-create): pinned repository name validation"
```

---

### Task 11 (GATED): TDD Rust — `lore_repository_create`

**Files:**
- Modify: `src-tauri/src/commands.rs` (après `lore_repositories`), `src-tauri/src/lib.rs`

- [ ] **Step 1: Écrire les tests qui échouent**

Ajouter dans `src-tauri/src/commands.rs` :

```rust
#[cfg(test)]
mod repo_create_tests {
    use super::*;
    use crate::lore::parse_events;

    #[test]
    fn parses_created_repo_id_from_fixture() {
        let events = parse_events(include_str!("../tests/fixtures/repo_create.ndjson")).unwrap();
        // Si la capture Task 9 a constaté que create ne renvoie PAS d'id,
        // remplacer cette assertion par `assert!(created_repo_id_from(&events).is_none())`
        // (le fallback re-list de la commande couvre ce cas).
        assert!(created_repo_id_from(&events).is_some());
    }

    #[test]
    fn no_create_event_is_none() {
        let events = parse_events(r#"{"tagName":"complete","data":{"status":0}}"#).unwrap();
        assert!(created_repo_id_from(&events).is_none());
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test --manifest-path src-tauri/Cargo.toml repo_create`
Expected: erreur de compilation — `created_repo_id_from` inexistant.

- [ ] **Step 3: Implémenter**

Ajouter dans `src-tauri/src/commands.rs` (après `lore_repositories`) :

```rust
/// Id of the repository created by `repository create`. Tag/field pinned by
/// the Task 9 capture (tests/fixtures/repo_create.ndjson). ⚠ pin Task 9.
fn created_repo_id_from(events: &[LoreEvent]) -> Option<String> {
    events_with_tag(events, "repositoryCreate") // ⚠ pin Task 9
        .into_iter()
        .find_map(|d| d.get("id").and_then(|v| v.as_str()).map(String::from))
}

/// Create `<name>` on the server and return its repository id — either from
/// the create output, or (when the CLI doesn't echo the id) by re-listing and
/// matching on the name.
#[tauri::command]
pub async fn lore_repository_create(server_url: String, name: String) -> Result<String, String> {
    blocking(move || {
        let events = run_lore(&["repository", "create", &server_url, &name])?; // ⚠ forme exacte pinnée Task 9
        if let Some(id) = created_repo_id_from(&events) {
            return Ok(id);
        }
        let list = run_lore(&["repository", "list", &server_url])?;
        repositories_from(&list)
            .into_iter()
            .find(|r| r.name == name)
            .map(|r| r.id)
            .ok_or_else(|| format!("repository '{name}' not found after create"))
    })
    .await
}
```

Dans `src-tauri/src/lib.rs`, ajouter au `generate_handler!` (après `commands::lore_repositories,`) :

```rust
        commands::lore_repository_create,
```

- [ ] **Step 4: Vérifier le pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml repo_create` → `test result: ok`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(repo-create): lore_repository_create command"
```

---

### Task 12 (GATED): UI « New repository… » (RepoSwitcher + RepoPicker) + mock

**Files:**
- Modify: `src/lib/types.ts` (LoreApi), `src/lib/tauri.ts`, `src/lib/mock.ts`, `src/lib/mock.test.ts`, `src/lib/repoActions.ts`, `src/lib/RepoSwitcher.svelte`, `src/lib/RepoPicker.svelte`

- [ ] **Step 1: Test mock qui échoue**

Ajouter à la fin de `src/lib/mock.test.ts` :

```ts
describe('mock createRepo', () => {
  it('creates a listed repo and returns its id', async () => {
    const id = await mock.createRepo('lore://demo:41337', 'p6-new-repo')
    expect(id).toBeTruthy()
    const repos = await mock.listRepos('lore://demo:41337')
    expect(repos.some((r) => r.id === id && r.name === 'p6-new-repo')).toBe(true)
  })
})
```

Run: `npx vitest run src/lib/mock.test.ts` → FAIL (`createRepo` inexistant).

- [ ] **Step 2: API + mock**

`src/lib/types.ts` — dans `LoreApi`, après `listRepos(…)` :

```ts
  /** Create a repository on the server; returns the new repository id. */
  createRepo(serverUrl: string, name: string): Promise<string>
```

`src/lib/tauri.ts` — après `listRepos:` :

```ts
  createRepo: (serverUrl, name) => invoke<string>('lore_repository_create', { serverUrl, name }),
```

`src/lib/mock.ts` — après `listRepos` :

```ts
  async createRepo(_serverUrl: string, name: string) {
    await delay(400)
    const id = '019f2e18' + Math.random().toString(16).slice(2).padEnd(24, '0').slice(0, 24)
    FAKE_REPOS.push({ id, name })
    return id
  },
```

Run: `npx vitest run src/lib/mock.test.ts` → PASS.

- [ ] **Step 3: Action `createServerRepo`**

Dans `src/lib/repoActions.ts`, ajouter après `cloneServerRepo` :

```ts
/**
 * Create a repository on the server, then clone it into a picked parent folder
 * and switch to it. Same global anti-double-clone guard as cloneServerRepo
 * (the slot is occupied from before the folder pick). Server errors surface
 * as a toastError; the caller validated the name beforehand (repoName.ts).
 */
export async function createServerRepo(name: string, useSharedStore = false): Promise<boolean> {
  if (cloneInFlight(opProgress.clone)) return false
  opProgress.clone = { done: 0 }
  try {
    const parent = await api.pickFolder()
    if (!parent) return false // cancelled
    const id = await api.createRepo(session.config.serverUrl!, name)
    const path = await api.cloneRepo(
      session.config.serverUrl!, id, name, parent,
      (p) => { opProgress.clone = p },
      useSharedStore,
    )
    await selectRepo(path)
    return true
  } catch (e) {
    toastError("Couldn't create repository", e)
    return false
  } finally {
    opProgress.clone = null
  }
}
```

(⚠ Si la Task 9 a constaté que `repository create` initialise DIRECTEMENT une working copy locale quand on lui passe un chemin, remplacer le couple create+clone par cet appel unique et le documenter ; le `selectRepo(path)` final ne change pas. Si la variante B de l'item 1 a été retenue, retirer le paramètre `useSharedStore` et l'argument passé à `cloneRepo`.)

- [ ] **Step 4: RepoSwitcher — sous-menu Add + mode create**

Dans `src/lib/RepoSwitcher.svelte` — imports :

```ts
import { addExistingRepo, cloneServerRepo, createServerRepo } from './repoActions'
import { validateRepoName } from './repoName'
```

états — remplacer `let mode = $state<'list' | 'clone'>('list')` par :

```ts
  // 'list' = known repos; 'clone' = pick a server repo; 'create' = new server repo.
  let mode = $state<'list' | 'clone' | 'create'>('list')
  let newName = $state('')
  const nameError = $derived(validateRepoName(newName))
```

logique (après `onClone`) :

```ts
  async function onCreate() {
    if (busy || nameError) return
    busy = 'create'
    try {
      if (await createServerRepo(newName.trim())) onclose()
    } finally {
      busy = ''
    }
  }
```

markup — dans le `.addmenu`, ajouter un troisième bouton après « Add existing repository… » :

```svelte
            <button class="action" onclick={() => { addOpen = false; newName = ''; mode = 'create' }}>
              <Icon name="plus" size={15} /> New repository…
            </button>
```

et transformer le `{:else}` (mode clone) en `{:else if mode === 'clone'}`, puis ajouter avant le `{/if}` final :

```svelte
  {:else if mode === 'create'}
    <div class="head">
      <button class="add" onclick={() => (mode = 'list')}><Icon name="chevronLeft" size={12} /> Back</button>
    </div>
    <div class="sec">New repository on {session.config.serverUrl}</div>
    <div class="createform">
      <input class="search" bind:value={newName} placeholder="repository-name"
             onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); onCreate() } }} />
      {#if newName.trim() && nameError}<p class="empty err">{nameError}</p>{/if}
      <button class="accent" onclick={onCreate} disabled={!!busy || !!nameError}>
        {busy === 'create' ? cloneProgressLabel(opProgress.clone) : 'Create & clone…'}
      </button>
      <p class="empty">Creates the repository on the server, then clones it into a folder you pick.</p>
    </div>
  {/if}
```

CSS (à la fin du `<style>`) :

```css
  .createform { display: flex; flex-direction: column; align-items: flex-start; gap: 8px; padding: 2px 12px 10px; }
  .createform .search { width: 100%; }
  .err { color: var(--deleted); margin: 0; }
```

- [ ] **Step 5: RepoPicker — bouton + mini-formulaire**

Dans `src/lib/RepoPicker.svelte` — imports :

```ts
import { addExistingRepo, cloneServerRepo, createServerRepo } from './repoActions'
import { validateRepoName } from './repoName'
```

états :

```ts
let createOpen = $state(false)
let newName = $state('')
const nameError = $derived(validateRepoName(newName))

async function createRepo() {
  if (busy || nameError) return
  busy = 'create'
  try {
    if (await createServerRepo(newName.trim())) { createOpen = false; newName = '' }
  } finally {
    busy = ''
  }
}
```

markup — remplacer `<h3>On {session.config.serverUrl}</h3>` par :

```svelte
  <div class="serverhead">
    <h3>On {session.config.serverUrl}</h3>
    <span class="spacer"></span>
    <button class="newrepo" onclick={() => (createOpen = !createOpen)}>New repository…</button>
  </div>
  {#if createOpen}
    <div class="card createcard">
      <input bind:value={newName} placeholder="repository-name"
             onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); createRepo() } }} />
      <button class="accent" onclick={createRepo} disabled={!!busy || !!nameError}>
        {busy === 'create' ? cloneProgressLabel(opProgress.clone) : 'Create & clone…'}
      </button>
      {#if newName.trim() && nameError}<p class="muted small err">{nameError}</p>{/if}
    </div>
  {/if}
```

CSS :

```css
  .serverhead { display: flex; align-items: baseline; gap: 8px; }
  .newrepo { font-size: 12px; padding: 4px 9px; }
  .createcard { margin: 6px 0; gap: 8px; }
  .createcard input { flex: 1; min-width: 0; padding: 7px 9px; background: var(--bg); border: 1px solid var(--border); border-radius: 6px; color: var(--text); font-size: 12px; }
  .err { color: var(--deleted); flex-basis: 100%; margin: 0; }
```

- [ ] **Step 6: Typecheck + suites + navigateur**

Run: `npm run check` → 0 erreur 0 warning. Run: `npx vitest run` → tout passe.
Navigateur (`npm run dev`) : RepoSwitcher → Add → New repository… → nom invalide (`Bad Name`) → message d'erreur, bouton désactivé ; nom valide `p6-demo` → Create & clone… → progression → l'app bascule sur `C:/SoonerOrLater/picked-repo/p6-demo` ; rouvrir le mode clone → `p6-demo` apparaît dans la liste serveur (mock). Même vérification côté RepoPicker (se déconnecter d'un repo courant via le picker si besoin).

- [ ] **Step 7: Commit**

```bash
git add src/lib/types.ts src/lib/tauri.ts src/lib/mock.ts src/lib/mock.test.ts src/lib/repoActions.ts src/lib/RepoSwitcher.svelte src/lib/RepoPicker.svelte
git commit -m "feat(repo-create): New repository flow in picker and switcher"
```

---

### Task 13: Vérification finale (suites + mock + réel)

**Files:** aucun nouveau — vérification uniquement. (Item 3 : ne vérifier que si les Tasks 9–12 ont été arbitrées et exécutées.)

- [ ] **Step 1: Suites complètes**

```powershell
npx vitest run
cargo test --manifest-path src-tauri/Cargo.toml
npm run check
```

Expected : vitest tous fichiers passed (16 existants + `pushErrors` + `sharedStore` [+ `repoName` si arbitré]), cargo `test result: ok` (0 failed), svelte-check 0 erreur 0 warning.

- [ ] **Step 2: Parcours navigateur mock**

`npm run dev` → dérouler et cocher :
- Levier `loredesktop.mock.pushNonFF` → Push → toast « Remote has new changes » → **Sync & push** → sync puis push OK, état propre. Sans levier : push normal, toast « locked files pushed » intact.
- Flux clone : case/toggle shared store (selon variante) — état par défaut, hint, persistance après un clone coché (variante A) ou via AvatarMenu (variante B).
- (Si arbitré) New repository… : validation du nom, création + clone + bascule, apparition dans la liste serveur mock.

- [ ] **Step 3: RÉEL — push refusé résolu par le toast**

Refabriquer la course de la Task 1 (Steps 2–3 du script : second clone temp → commit+push distant → commit local dans `C:\Users\jimmy\lore-test-repo`), puis **dans l'app Tauri** (`npm run tauri dev`) : ouvrir le repo de test, cliquer **Push** → attendu : toast « Remote has new changes » (PAS le toast rouge), cliquer **Sync & push** → progression sync puis push, titre revenu propre (`lore status --scan --repository C:\Users\jimmy\lore-test-repo --json` : plus de ahead). Cleanup : `Remove-Item -Recurse -Force C:\Users\jimmy\lore-tmp-p6-clone`.

- [ ] **Step 4: RÉEL — clone avec shared store**

Dans l'app : flux clone de `desktoptest1` vers un dossier temporaire avec l'option shared store active (case cochée en variante A / toggle on en variante B) → attendu : clone OK, l'app bascule, et `lore shared-store info --json` confirme le store utilisé (+ trace dans `.lore/` de la working copy, selon le constat Task 5). Cleanup : retirer le repo de la liste (bouton ×) et supprimer le dossier cloné.

- [ ] **Step 5 (GATED — seulement si arbitré): RÉEL — création bout-en-bout**

Dans l'app : New repository… → nom `desktoptest-created-<date>-app` → Create & clone… → attendu : repo créé sur le serveur (`lore repository list …` le montre), cloné, sélectionné. Cleanup selon le constat Task 9 (`repository delete` si disponible, sinon signaler le repo permanent).

- [ ] **Step 6: Commit final (si des retouches de vérification ont eu lieu)**

```bash
git add -A
git commit -m "chore(p6): final verification fixes"
```

---

## Hors périmètre (rappel spec)

`repository delete` depuis l'app, gestion fine du store (purge, taille), auto-sync périodique, retry automatique du push.

## Self-review (complétée par le contrôleur — le rédacteur a été coupé par la limite de dépense juste avant cette section ; structure et tâches vérifiées intactes)

- **Couverture spec** : item 2 → Tasks 1–4 (capture du push non-fast-forward réel avec second clone temp + cleanup ; matcher pur pinné sur le message capturé, jamais un match large ; toast action « Remote has new changes » enchaînant sync puis push, arrêt propre si le sync ouvre un merge — le chip existant prend le relais ; levier mock + vérif navigateur). Item 1 → Tasks 5–8 (discovery sémantique shared-store ; commandes Rust status/enable/disable ; API front + helper de case ; UI en DEUX variantes complètes A case-au-clone / B toggle-global, choisies au constat de la Task 5). Item 3 GATED → Tasks 9–12 (discovery create + delete éventuel ; validation de nom ; commande ; UI New repository — chaque tâche marquée GATED, à n'exécuter que sur arbitrage explicite de Jimmy). Task 13 = suites + navigateur mock + vérifs réelles (item 3 seulement si arbitré). Ordre spec 2 → 1 → 3 respecté.
- **Placeholders** : aucun TBD — les seules adaptations sont les pins de capture/discovery (Tasks 1, 5, 9), bornés au champ/message près, avec les deux variantes de la Task 8 écrites en code complet.
- **Cohérence de types** :  défini Task 2, consommé Task 3 (26 occurrences cohérentes) ;  (Rust, Task 6) ↔  (front, Task 7) ↔ / (helpers, Tasks 7–8) ;  défini Task 10, consommé Tasks 11–12 ;  Task 11 ↔ invoke Task 12.

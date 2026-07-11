# Lore Desktop — Lot P4 « visibilité du repo » Implementation Plan

> **STATUT : EXÉCUTÉ ET VÉRIFIÉ le 2026-07-11** (subagent-driven, double revue par tâche, revue finale item 5 : READY).
>
> **Suites** : vitest **139 passed / 21 fichiers**, 0 failed ; cargo `--lib` **107 passed**, 0 failed ; `npm run check` **0 errors, 0 warnings** (886 fichiers).
>
> **Parcours navigateur mock validé** : (1) en-tête Changes « 10 files +3~6−1 » avec couleurs réelles `--added` vert (63,185,80), `--modified` ambre (210,153,34), `--deleted` rouge (248,81,73), tiret U+2212 ; (2) BranchMenu — filtre `maya` → section « Remote » seule avec `user/maya/lighting-wip` estompé (opacity .65, classe `remote`), switch d'une remote-only → l'en-tête passe à `user/maya/lighting-wip` (checkout mock, devient locale), virtualisation préservée sur 2004 branches ; (3) **item 3 livré en variante B** — en-tête « BRANCHES · 2 007 · MAIN ↓1 » (ahead/behind de la branche courante, source status) ; (4) **item 4 (protected) ANNULÉ** — capture Task 9 : `protected` absent de `branch list` ET `branch info`, un badge codé en dur main/master serait mensonger (repli prévu par la spec) ; (5) About repository — 9 lignes ordonnées (Name, Repository id + Copy, Description, Local path + Reveal, Server, Default branch, Current branch, Revision #5, Created **2026-07-05**), Escape ferme, entrée absente sans repo ouvert. Console sans erreur.
>
> **Captures réelles pinnées** : `branch_list.ndjson` (`branchListEntry.location`), `branch_info.ndjson` (constat : pas d'ahead/behind par branche → variante B ; `protected` absent → item 4 annulé), `repo_info.ndjson` (tag réel **`repositoryData`** ≠ hypothèse `repositoryInfo` du plan ; champs `remoteUrl`/`id`/`name`/`description`/`defaultBranchName`/`created` epoch **secondes** ; **pas de `size`** → ligne Size supprimée). README des fixtures à jour.
>
> **Déviations conscientes vs draft du plan** : item 5 réécrit sur la capture (tag `repositoryData`, champs réels, Size supprimée, Description + Default branch + Created ajoutées, date formatée UTC `YYYY-MM-DD` pour un test stable) ; item 3 en variante B (en-tête) faute d'ahead/behind par branche côté CLI ; item 4 supprimé. Nitpicks non bloquants notés par la revue finale : `api.revealPath` en promesse flottante dans `AboutRepo.svelte` (cohérent avec le design « no toast »), pas de reset de `info` au changement de repo modal ouvert (fenêtre inatteignable), pas de focus-trap (panneau read-only).

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Livrer les 5 items read-only du lot P4 (spec `docs/superpowers/specs/2026-07-11-lore-desktop-p4-visibility-design.md`) : compteurs +/~/− dans l'en-tête de Changes, section « Remote » du BranchMenu, badge « protected » lecture seule, ahead/behind lazy par branche, panneau « About repository ».

**Architecture:** Aucun nouveau flux d'écriture. Backend Rust : enrichissement de `status_from` (summary), de `branches_from` (location, protected) et deux nouvelles commandes read-only (`lore_branch_info`, `lore_repository_info`) — chaque champ wire non pinné a sa tâche de CAPTURE en tête d'item, avec adaptation au constat et annulation propre en cas de constat négatif. Frontend Svelte 5 : la logique nouvelle vit dans des modules purs testés vitest (`statusSummary.ts`, `branchGrouping.ts`, `branchProtection.ts`, `branchInfoCache.ts`, `aboutFields.ts`), le markup dans les composants existants + un nouveau `AboutRepo.svelte`. Le mock fait vivre chaque nouveauté en dev navigateur.

**Tech Stack:** Svelte 5 runes, TypeScript, vitest (fake timers pour le débounce), Rust (Tauri v2), PowerShell pour les captures réelles.

---

## Contexte & conventions (à lire avant toute tâche)

- **Repo de test réel** : working copy `C:\Users\jimmy\lore-test-repo` (repo `desktoptest1`) sur `lore://lore.example.com:41337`. Binaire CLI : `C:\Users\jimmy\bin\lore.exe`. Les captures (Tasks 5, 9, 16) exigent le serveur joignable — vérifier avec `& $lore status --repository $repo --json` (`remoteAvailable:1`) avant de capturer.
- **Contrainte vitest** : `vitest.config.ts` n'a PAS le plugin Svelte — les tests ne peuvent importer NI un `.svelte` NI un `.svelte.ts` à runes. Toute logique testée vit dans des modules purs (`statusSummary.ts`, `branchGrouping.ts`, `branchProtection.ts`, `branchInfoCache.ts`, `aboutFields.ts`) ; le wiring composants/stores est vérifié navigateur (Task 20).
- **Commandes de test** :
  - Vitest ciblé : `npx vitest run src/lib/<fichier>.test.ts` — attendu `Test Files  1 passed`.
  - Vitest complet : `npx vitest run` — attendu `Test Files  N passed`, 0 failed.
  - Rust ciblé : `cargo test --manifest-path src-tauri/Cargo.toml <filtre>` — attendu `test result: ok`.
  - Typecheck : `npm run check` — attendu `svelte-check found 0 errors and 0 warnings`.
  - Dev navigateur (mock) : `npm run dev` → http://localhost:5173.
- **Défauts sûrs partout** : tout champ wire absent (CLI plus ancien, serveur down) ⇒ feature masquée, jamais de crash. Concrètement : `summary` = `Option` (header sans compteurs), `location` absente = `"local"` (pas de section Remote fantôme), `protected` absent = `false` (pas de badge), champs `branch info` / `repository info` absents = lignes/annotations masquées.
- **Constat négatif = résultat valide** : la spec prévoit l'annulation propre de l'item 4 si le flag protected n'existe nulle part en lecture, et le repli de l'item 3 si `branch info` n'expose pas d'ahead/behind. Les tâches concernées (10–11, 12–15) portent leurs instructions d'annulation/repli.
- **Stress-test mock** : `mock.ts` génère 2004 branches (`buildBranches(2000)`). Le fetch ahead/behind est donc STRICTEMENT lazy (survol/focus d'une ligne, débounce 150 ms, cache Map par nom) — jamais un appel par branche au montage ni au scroll.
- Ordre de livraison imposé par la spec : Item 1 (Tasks 1–4) → Item 2 (Tasks 5–8) → Item 4 (Tasks 9–11) → Item 3 (Tasks 12–15) → Item 5 (Tasks 16–19) → vérification finale (Task 20).

## Carte des fichiers

**Créés :**
- `src-tauri/tests/fixtures/branch_list.ndjson` — capture `branch list --json` (Task 5).
- `src-tauri/tests/fixtures/branch_info.ndjson` — capture `branch info <name> --json` (Task 9).
- `src-tauri/tests/fixtures/repo_info.ndjson` — capture `repository info --json` (Task 16).
- `src/lib/statusSummary.ts` (+ `statusSummary.test.ts`) — formatage des compteurs +/~/−.
- `src/lib/branchGrouping.ts` (+ `branchGrouping.test.ts`) — groupement local/remote filtrable en lignes virtualisables.
- `src/lib/branchProtection.ts` (+ `branchProtection.test.ts`) — tooltip protected + titre du bouton Push (conditionnel Task 9).
- `src/lib/branchInfoCache.ts` (+ `branchInfoCache.test.ts`) — débounce + cache Map du branch info lazy, `formatAheadBehind`.
- `src/lib/branchInfo.svelte.ts` — singleton réactif du cache (survit aux réouvertures du menu, purgé au changement de repo).
- `src/lib/AboutRepo.svelte` — panneau modal « About repository ».

**Modifiés :**
- `src-tauri/src/commands.rs` — `StatusSummaryDto` + `summary` dans `status_from` ; `location`/`protected` dans `BranchDto`/`branches_from` ; `lore_branch_info` ; `lore_repository_info` ; tests.
- `src-tauri/src/lib.rs` — enregistrement des 2 nouvelles commandes.
- `src-tauri/tests/fixtures/README.md` — pin des encodages constatés (Tasks 1, 5, 9, 16).
- `src/lib/types.ts` — `StatusResult.summary`, `Branch.location`/`Branch.protected`, `BranchInfo`, `RepositoryInfo`, méthodes `LoreApi`.
- `src/lib/mock.ts` — summary cohérent avec les fichiers, branches remote-only, `main` protected, `getBranchInfo`, `getRepositoryInfo`.
- `src/lib/tauri.ts` — `getBranchInfo`, `getRepositoryInfo`.
- `src/lib/Changes.svelte` — compteurs dans le colhead.
- `src/lib/BranchMenu.svelte` — liste groupée (toujours virtualisée), badge cadenas, ↑/↓ lazy.
- `src/lib/TitleBar.svelte` — tooltip Push enrichi, wiring du panneau About.
- `src/lib/RepoSwitcher.svelte` — entrée « About repository ».

---

## Item 1 — Compteurs +/~/− dans l'en-tête de Changes

### Task 1: Constat — le `repositoryStatusSummary` est DÉJÀ capturé (pas de recapture)

**Files:**
- Modify: `src-tauri/tests/fixtures/README.md`

- [ ] **Step 1: Vérifier la fixture existante**

La fixture P1 `src-tauri/tests/fixtures/status.ndjson` contient déjà l'événement (ligne 4) :

```json
{"tagName":"repositoryStatusSummary","data":{"adds":1,"deletes":0,"modifies":1,"moves":0,"copies":0}}
```

Confirmer avec :

```powershell
Select-String -Path src-tauri\tests\fixtures\status.ndjson -Pattern repositoryStatusSummary
```

Expected: la ligne ci-dessus. **Aucune recapture nécessaire** — l'encodage est pinné : cinq compteurs u64 `adds` / `deletes` / `modifies` / `moves` / `copies`, un seul événement par status, émis après les `repositoryStatusFile`. (Si contre toute attente la ligne manquait — fixture régénérée entre-temps — recapturer `& $lore status --scan --repository C:\Users\jimmy\lore-test-repo --json` vers `status.ndjson` et vérifier que l'événement y est.)

- [ ] **Step 2: Documenter dans le README des fixtures**

Ajouter à la fin de `src-tauri/tests/fixtures/README.md` :

```markdown

**`repositoryStatusSummary`** (déjà présent dans status.ndjson, constaté au lot P4) :
un seul événement par status, après les `repositoryStatusFile` — cinq compteurs u64
`adds` / `deletes` / `modifies` / `moves` / `copies`. Le DTO replie
`modifies + moves + copies` dans `mods` (l'UI colore R/C comme des « modified »).
Événement absent (CLI plus ancien) ⇒ `summary: None` ⇒ compteurs masqués côté UI.
```

- [ ] **Step 3: Commit**

```bash
git add src-tauri/tests/fixtures/README.md
git commit -m "docs(fixtures): pin the repositoryStatusSummary encoding (already captured in P1)"
```

### Task 2: Backend — `summary` dans `StatusResultDto` (TDD)

**Files:**
- Modify: `src-tauri/src/commands.rs` (struct ~ligne 53, `status_from` ~ligne 165, module `status_tests` ~ligne 1867)

- [ ] **Step 1: Écrire les tests qui échouent**

Ajouter dans le module `status_tests` de `src-tauri/src/commands.rs` :

```rust
    #[test]
    fn parses_summary_from_fixture() {
        let events = parse_events(include_str!("../tests/fixtures/status.ndjson")).unwrap();
        let s = status_from(&events, std::path::Path::new(""));
        let sum = s.summary.expect("the captured fixture carries a repositoryStatusSummary");
        assert_eq!(sum, StatusSummaryDto { adds: 1, mods: 1, dels: 0 });
    }

    #[test]
    fn missing_summary_event_is_none() {
        let sample = concat!(
            r#"{"tagName":"repositoryStatusRevision","data":{"branchName":"main","revisionLocalNumber":1,"revisionRemoteNumber":1,"isLocalAhead":false,"isRemoteAhead":false}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let s = status_from(&parse_events(sample).unwrap(), std::path::Path::new(""));
        assert!(s.summary.is_none());
    }

    #[test]
    fn moves_and_copies_fold_into_mods() {
        let sample = concat!(
            r#"{"tagName":"repositoryStatusRevision","data":{"branchName":"main","revisionLocalNumber":1,"revisionRemoteNumber":1,"isLocalAhead":false,"isRemoteAhead":false}}"#, "\n",
            r#"{"tagName":"repositoryStatusSummary","data":{"adds":2,"deletes":1,"modifies":3,"moves":1,"copies":1}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let s = status_from(&parse_events(sample).unwrap(), std::path::Path::new(""));
        assert_eq!(s.summary, Some(StatusSummaryDto { adds: 2, mods: 5, dels: 1 }));
    }
```

- [ ] **Step 2: Vérifier que ça échoue**

Run: `cargo test --manifest-path src-tauri/Cargo.toml status_tests`
Expected: erreur de compilation `no field summary on type StatusResultDto` / `cannot find StatusSummaryDto`.

- [ ] **Step 3: Implémenter**

Dans `src-tauri/src/commands.rs`, juste au-dessus de `StatusResultDto` :

```rust
/// Compteurs du `repositoryStatusSummary` (voir fixtures/README.md — encodage
/// pinné au lot P4). `mods` replie modifies + moves + copies : l'UI colore les
/// glyphes R/C dans la famille « modified ».
#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StatusSummaryDto {
    pub adds: u64,
    pub mods: u64,
    pub dels: u64,
}
```

Dans `StatusResultDto`, ajouter avant `pub files` :

```rust
    /// Compteurs adds/mods/dels du wire ; absent quand le CLI ne les émet pas.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<StatusSummaryDto>,
```

Dans `status_from`, avant `let files = …` :

```rust
    // Événement absent (CLI plus ancien) => None => compteurs masqués, pas de faux zéros.
    let summary = events_with_tag(events, "repositoryStatusSummary")
        .into_iter()
        .next()
        .map(|d| {
            let n = |k: &str| d.get(k).and_then(|v| v.as_u64()).unwrap_or(0);
            StatusSummaryDto { adds: n("adds"), mods: n("modifies") + n("moves") + n("copies"), dels: n("deletes") }
        });
```

Et remplacer la construction finale par :

```rust
    StatusResultDto { branch, local_ahead, remote_ahead, revision_number, remote_available, remote_authorized, merge_in_progress, staged_pending, summary, files }
```

- [ ] **Step 4: Vérifier que ça passe**

Run: `cargo test --manifest-path src-tauri/Cargo.toml status_tests`
Expected: `test result: ok` (tous les tests du module, y compris les 3 nouveaux).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(status): expose the repositoryStatusSummary counters in StatusResult"
```

### Task 3: Frontend — type `summary` + module pur `statusSummary.ts` + seed mock (TDD)

**Files:**
- Create: `src/lib/statusSummary.ts`
- Test: `src/lib/statusSummary.test.ts`
- Modify: `src/lib/types.ts` (interface `StatusResult`), `src/lib/mock.ts` (`getStatus`)

- [ ] **Step 1: Écrire les tests qui échouent**

Créer `src/lib/statusSummary.test.ts` (le `−` est U+2212, comme le glyphe delete de Changes.svelte) :

```ts
import { describe, it, expect } from 'vitest'
import { summaryParts } from './statusSummary'

describe('summaryParts', () => {
  it('is empty when the summary is absent (older CLI) or all-zero', () => {
    expect(summaryParts(undefined)).toEqual([])
    expect(summaryParts(null)).toEqual([])
    expect(summaryParts({ adds: 0, mods: 0, dels: 0 })).toEqual([])
  })
  it('renders only the non-zero counters, in +/~/− order', () => {
    expect(summaryParts({ adds: 3, mods: 2, dels: 1 })).toEqual([
      { text: '+3', cls: 'added' },
      { text: '~2', cls: 'modified' },
      { text: '−1', cls: 'deleted' },
    ])
    expect(summaryParts({ adds: 0, mods: 4, dels: 0 })).toEqual([{ text: '~4', cls: 'modified' }])
  })
})
```

- [ ] **Step 2: Vérifier que ça échoue**

Run: `npx vitest run src/lib/statusSummary.test.ts`
Expected: FAIL — `Cannot find module './statusSummary'`.

- [ ] **Step 3: Implémenter**

Créer `src/lib/statusSummary.ts` :

```ts
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
```

Dans `src/lib/types.ts`, ajouter à l'interface `StatusResult` (après `stagedPending`) :

```ts
  /** Compteurs wire repositoryStatusSummary ; absent sur un CLI plus ancien. */
  summary?: { adds: number; mods: number; dels: number }
```

Dans `src/lib/mock.ts`, méthode `getStatus`, ajouter au retour (après `stagedPending: …`) — cohérent avec `s.files`, donc les compteurs bougent après commit/discard/undo :

```ts
      // Seedé depuis les fichiers courants pour rester cohérent (spec P4 item 1).
      summary: {
        adds: s.files.filter((f) => f.action === 'add').length,
        mods: s.files.filter((f) => f.action === 'modify' || f.action === 'move' || f.action === 'copy').length,
        dels: s.files.filter((f) => f.action === 'delete').length,
      },
```

- [ ] **Step 4: Vérifier que ça passe**

Run: `npx vitest run src/lib/statusSummary.test.ts`
Expected: `Test Files  1 passed`.
Run: `npm run check`
Expected: `svelte-check found 0 errors and 0 warnings`.

- [ ] **Step 5: Commit**

```bash
git add src/lib/statusSummary.ts src/lib/statusSummary.test.ts src/lib/types.ts src/lib/mock.ts
git commit -m "feat(changes): status summary type, pure formatter and mock seed"
```

### Task 4: UI — compteurs dans le colhead de Changes

**Files:**
- Modify: `src/lib/Changes.svelte` (imports ~ligne 7, dérivés ~ligne 36, colhead ligne 113, styles)

- [ ] **Step 1: Brancher le composant**

Dans `src/lib/Changes.svelte`, ajouter l'import :

```ts
  import { summaryParts } from './statusSummary'
```

Ajouter le dérivé (près de `const branch = …`) :

```ts
  const summary = $derived(summaryParts(repo.status?.summary))
```

Remplacer la ligne du colhead :

```svelte
  <div class="colhead">Changes <span class="n">{filter.trim() ? `${shownCount} of ${files.length} files` : `${files.length} ${files.length === 1 ? 'file' : 'files'}`}</span></div>
```

par :

```svelte
  <div class="colhead">Changes
    <span class="n">{filter.trim() ? `${shownCount} of ${files.length} files` : `${files.length} ${files.length === 1 ? 'file' : 'files'}`}</span>
    {#if summary.length > 0}
      <span class="sum" aria-label="Change counters">
        {#each summary as p (p.cls)}<span class="p {p.cls}">{p.text}</span>{/each}
      </span>
    {/if}
  </div>
```

Ajouter dans le `<style>` (après la règle `.colhead .n`) :

```css
  .sum { margin-left: 6px; font-size: 11px; font-family: var(--font-mono); }
  .sum .p { margin-right: 5px; }
  .sum .added { color: var(--added); }
  .sum .modified { color: var(--modified); }
  .sum .deleted { color: var(--deleted); }
```

- [ ] **Step 2: Typecheck + vérification navigateur**

Run: `npm run check` — Expected: 0 errors, 0 warnings.
Run: `npm run dev` → http://localhost:5173, ouvrir un repo mock. Attendu dans l'en-tête Changes : « 10 files  +3 ~6 −1 » (seed mock : 3 adds, 6 modify, 1 delete), en couleurs vert/jaune/rouge. Committer tous les fichiers → « No local changes » et compteurs disparus (tout à zéro ⇒ masqués).

- [ ] **Step 3: Commit**

```bash
git add src/lib/Changes.svelte
git commit -m "feat(changes): +/~/- counters in the Changes header"
```

---

## Item 2 — Section « Remote » dans le BranchMenu

### Task 5: CAPTURE réelle — `branch list --json` (fixture + constat `location` et `protected`)

**Files:**
- Create: `src-tauri/tests/fixtures/branch_list.ndjson`
- Modify: `src-tauri/tests/fixtures/README.md`

- [ ] **Step 1: Capturer**

Dans PowerShell, à la racine du repo (`C:\Users\jimmy\Documents\SoonerOrLater\lore-desktop`), serveur joignable :

```powershell
$lore = "C:\Users\jimmy\bin\lore.exe"
$repo = "C:\Users\jimmy\lore-test-repo"
& $lore branch list --repository $repo --json | Out-File -Encoding utf8NoBOM src-tauri\tests\fixtures\branch_list.ndjson
Get-Content src-tauri\tests\fixtures\branch_list.ndjson
```

Expected: des blocs `branchListBegin` / `branchListEntry` / `branchListEnd` pour `location:"local"` PUIS `location:"remote"`, terminés par `{"tagName":"complete","data":{"status":0}}`. Si le bloc remote manque, vérifier la connectivité serveur et recapturer.

- [ ] **Step 2: Inspecter et pinner**

Noter dans la capture :
1. **`location`** : valeurs exactes (hypothèse : chaînes `"local"` / `"remote"` — déjà supposées par `branch_tips_from` dans commands.rs). Si l'encodage réel diffère (nombre, autre libellé), adapter `branches_from` en Task 6 et le README.
2. **Flag protected** : chercher `protect` (insensible à la casse) dans la capture :

```powershell
Select-String -Path src-tauri\tests\fixtures\branch_list.ndjson -Pattern protect
```

Noter le résultat (nom de champ exact ou « absent ») — c'est la moitié du constat de la Task 9.

- [ ] **Step 3: Documenter dans le README des fixtures**

Ajouter à la fin de `src-tauri/tests/fixtures/README.md` (adapter aux constats réels) :

```markdown

**`branchListEntry`** (branch_list.ndjson, capturé au lot P4) : une entrée par
branche PAR location — bloc `location:"local"` d'abord, puis `location:"remote"`
(une branche présente des deux côtés apparaît deux fois ; dédupe par nom,
« local » gagne). Champs : `name`, `latest` (hash tip), `isCurrent` (local
uniquement), `archived`, `location`. Flag protected : <présent sous le nom `…`
/ absent — voir Task 9>.
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/tests/fixtures/branch_list.ndjson src-tauri/tests/fixtures/README.md
git commit -m "test(fixtures): capture lore branch list output (location local/remote)"
```

### Task 6: Backend — `location` dans `BranchDto` (TDD)

**Files:**
- Modify: `src-tauri/src/commands.rs` (`BranchDto` ~ligne 974, `branches_from` ~ligne 985, module `branches_tests` ~ligne 1772)

- [ ] **Step 1: Écrire les tests qui échouent**

Dans le module `branches_tests`, remplacer le test `unions_dedupes_and_marks_current` par :

```rust
    #[test]
    fn unions_dedupes_and_marks_current() {
        let events = parse_events(SAMPLE).unwrap();
        let branches = branches_from(&events);
        // main (deduped local+remote) + feature/x (remote-only); archived old/thing dropped.
        assert_eq!(branches.len(), 2);
        assert_eq!(branches[0].name, "main"); // local order first
        assert!(branches[0].current);
        assert_eq!(branches[0].location, "local"); // present locally => local wins the dedupe
        assert_eq!(branches[1].name, "feature/x");
        assert!(!branches[1].current);
        assert_eq!(branches[1].location, "remote"); // remote-only
    }

    #[test]
    fn missing_location_defaults_to_local() {
        // Older CLI without the field: no phantom Remote section.
        let sample = concat!(
            r#"{"tagName":"branchListEntry","data":{"name":"main","latest":"a1","isCurrent":true,"archived":false}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let branches = branches_from(&parse_events(sample).unwrap());
        assert_eq!(branches[0].location, "local");
    }

    #[test]
    fn parses_branch_list_fixture() {
        let events = parse_events(include_str!("../tests/fixtures/branch_list.ndjson")).unwrap();
        let branches = branches_from(&events);
        assert!(!branches.is_empty(), "the captured fixture must list at least one branch");
        assert!(branches.iter().any(|b| b.current), "one branch is current");
        assert!(branches.iter().all(|b| b.location == "local" || b.location == "remote"));
    }
```

- [ ] **Step 2: Vérifier que ça échoue**

Run: `cargo test --manifest-path src-tauri/Cargo.toml branches_tests`
Expected: erreur de compilation `no field location on type BranchDto`.

- [ ] **Step 3: Implémenter**

Remplacer `BranchDto` :

```rust
#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BranchDto {
    pub name: String,
    pub current: bool,
    /// "local" (existe dans la working copy) ou "remote" (remote-only après
    /// dédup). Champ wire absent (CLI plus ancien) => "local" : défaut sûr,
    /// pas de section Remote fantôme.
    pub location: String,
}
```

Remplacer `branches_from` :

```rust
/// Union of `branchListEntry` events (which stream once per location, local then
/// remote) deduped by name. `current` folds `isCurrent` across every entry for a
/// name (only local entries carry it); a name with ANY local entry is "local",
/// otherwise "remote" (remote-only). Archived branches are dropped. First-seen
/// order is preserved, so local branches come first and remote-only ones append.
fn branches_from(events: &[LoreEvent]) -> Vec<BranchDto> {
    let mut order: Vec<String> = Vec::new();
    let mut current: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut local: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for d in events_with_tag(events, "branchListEntry") {
        if d.get("archived").map(json_truthy).unwrap_or(false) {
            continue;
        }
        let name = match d.get("name").and_then(|v| v.as_str()) {
            Some(n) if !n.is_empty() => n.to_string(),
            _ => continue,
        };
        if d.get("location").and_then(|v| v.as_str()).unwrap_or("local") == "local" {
            local.insert(name.clone());
        }
        if d.get("isCurrent").map(json_truthy).unwrap_or(false) {
            current.insert(name.clone());
        }
        if seen.insert(name.clone()) {
            order.push(name);
        }
    }
    order
        .into_iter()
        .map(|name| BranchDto {
            current: current.contains(&name),
            location: if local.contains(&name) { "local" } else { "remote" }.to_string(),
            name,
        })
        .collect()
}
```

- [ ] **Step 4: Vérifier que ça passe**

Run: `cargo test --manifest-path src-tauri/Cargo.toml branches_tests`
Expected: `test result: ok` (5 tests, dont les 3 nouveaux/étendus).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(branches): expose local/remote location in the Branch DTO"
```

### Task 7: Module pur `branchGrouping.ts` (TDD)

**Files:**
- Create: `src/lib/branchGrouping.ts`
- Test: `src/lib/branchGrouping.test.ts`
- Modify: `src/lib/types.ts` (interface `Branch`)

- [ ] **Step 1: Étendre le type `Branch`**

Dans `src/lib/types.ts`, remplacer l'interface `Branch` :

```ts
export interface Branch {
  name: string
  current: boolean
  /** 'remote' = existe seulement côté serveur (le switch reste permis — le CLI fait le checkout). Absent → local. */
  location?: 'local' | 'remote'
}
```

- [ ] **Step 2: Écrire les tests qui échouent**

Créer `src/lib/branchGrouping.test.ts` :

```ts
import { describe, it, expect } from 'vitest'
import { groupBranches } from './branchGrouping'
import type { Branch } from './types'

const b = (name: string, location?: 'local' | 'remote', current = false): Branch => ({ name, current, location })

describe('groupBranches', () => {
  const list = [b('main', 'local', true), b('feature/loot', 'local'), b('release/srv', 'remote'), b('hotfix/srv', 'remote')]

  it('locals first, then a Remote header, then remote-only branches', () => {
    const rows = groupBranches(list, '')
    expect(rows.map((r) => (r.kind === 'header' ? '§' + r.label : r.branch.name)))
      .toEqual(['main', 'feature/loot', '§Remote', 'release/srv', 'hotfix/srv'])
  })

  it('emits no header when there is no remote-only branch', () => {
    const rows = groupBranches([b('main', 'local', true), b('feature/loot', 'local')], '')
    expect(rows.every((r) => r.kind === 'branch')).toBe(true)
  })

  it('the filter applies to both groups and drops an emptied Remote section', () => {
    const onlyLocal = groupBranches(list, 'loot')
    expect(onlyLocal).toEqual([{ kind: 'branch', branch: b('feature/loot', 'local') }])
    const onlyRemote = groupBranches(list, 'srv')
    expect(onlyRemote[0]).toEqual({ kind: 'header', label: 'Remote' })
    expect(onlyRemote).toHaveLength(3)
  })

  it('a missing location counts as local (safe default: no phantom Remote section)', () => {
    const rows = groupBranches([b('legacy', undefined, true)], '')
    expect(rows).toEqual([{ kind: 'branch', branch: b('legacy', undefined, true) }])
  })
})
```

- [ ] **Step 3: Vérifier que ça échoue**

Run: `npx vitest run src/lib/branchGrouping.test.ts`
Expected: FAIL — `Cannot find module './branchGrouping'`.

- [ ] **Step 4: Implémenter**

Créer `src/lib/branchGrouping.ts` :

```ts
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
```

- [ ] **Step 5: Vérifier que ça passe**

Run: `npx vitest run src/lib/branchGrouping.test.ts`
Expected: `Test Files  1 passed` (4 tests).

- [ ] **Step 6: Commit**

```bash
git add src/lib/branchGrouping.ts src/lib/branchGrouping.test.ts src/lib/types.ts
git commit -m "feat(branches): pure local/remote grouping for the virtualized branch menu"
```

### Task 8: UI — BranchMenu groupé + mock remote-only

**Files:**
- Modify: `src/lib/BranchMenu.svelte`, `src/lib/mock.ts` (`buildBranches`, `switchBranch`)

- [ ] **Step 1: Mock — 3 branches remote-only**

Dans `src/lib/mock.ts`, remplacer le tableau `base` de `buildBranches` :

```ts
  const base: Branch[] = [
    { name: 'main', current: true, location: 'local' },
    { name: 'feature/loot', current: false, location: 'local' },
    { name: 'fix/lighting-bake', current: false, location: 'local' },
    { name: 'experimental/ai-nav', current: false, location: 'local' },
    // Remote-only: visible in the new Remote section; switching checks them out.
    { name: 'release/1.0-cut', current: false, location: 'remote' },
    { name: 'user/maya/lighting-wip', current: false, location: 'remote' },
    { name: 'hotfix/crash-on-load', current: false, location: 'remote' },
  ]
```

et la ligne du générateur :

```ts
    gen.push({ name: `${prefixes[i % prefixes.length]}/${topics[(i * 5) % topics.length]}-${i + 1}`, current: false, location: 'local' })
```

Dans `switchBranch`, refléter le checkout d'une branche remote-only (elle devient locale) :

```ts
  async switchBranch(repoPath: string, name: string) {
    await delay(300)
    branchList = branchList.map((b) => ({
      ...b,
      current: b.name === name,
      location: b.name === name ? 'local' : b.location,
    }))
    stateFor(repoPath).branch = name
  },
```

- [ ] **Step 2: BranchMenu — lignes groupées virtualisées**

Dans `src/lib/BranchMenu.svelte` :

Ajouter l'import :

```ts
  import { groupBranches } from './branchGrouping'
```

Remplacer la ligne `const shown = $derived(…)` par :

```ts
  const rows = $derived(groupBranches(branches.list, filter))
  const branchCount = $derived(rows.reduce((n, r) => (r.kind === 'branch' ? n + 1 : n), 0))
```

Remplacer les dérivés de virtualisation (les lignes `listHeight` / `winLast` / `windowBranches`) par :

```ts
  const listHeight = $derived(Math.min(rows.length * ROW_H, 238))
  const winFirst = $derived(Math.max(0, Math.floor(listScroll / ROW_H) - 4))
  const winLast = $derived(Math.min(rows.length, Math.ceil((listScroll + listHeight) / ROW_H) + 4))
  const windowRows = $derived(rows.slice(winFirst, winLast))
```

Remplacer la ligne `<div class="sec">Branches · {shown.length.toLocaleString()}</div>` par :

```svelte
  <div class="sec">Branches · {branchCount.toLocaleString()}</div>
```

Remplacer le bloc `<div class="list" …>…</div>` (la liste virtualisée) par :

```svelte
  <div class="list" bind:this={listEl} onscroll={onListScroll} style="height:{listHeight}px">
    <div class="listvp" style="height:{rows.length * ROW_H}px">
      {#each windowRows as r, k (r.kind === 'header' ? '§' + r.label : r.branch.name)}
        <div class="rowwrap" style="top:{(winFirst + k) * ROW_H}px; height:{ROW_H}px">
          {#if r.kind === 'header'}
            <div class="subsec">{r.label}</div>
          {:else}
            {@const b = r.branch}
            <button class="item" class:cur={b.current} class:remote={b.location === 'remote'}
                    onclick={() => (b.current ? onclose() : switchTo(b.name))} disabled={busy}>
              <span class="dot" style="background:{LANE[(winFirst + k) % LANE.length]}"></span>
              <span class="bn">{b.name}</span>
              {#if b.current}<Icon name="check" size={14} />{/if}
            </button>
            {#if !b.current && b.location !== 'remote'}
              <!-- Archive reste locale : pas d'extension de la surface d'écriture aux
                   branches remote-only dans ce lot read-only. -->
              <button class="arch" title="Archive (hides from lists; nothing is deleted)"
                      onclick={() => archive(b.name)} disabled={busy}>Archive</button>
            {/if}
          {/if}
        </div>
      {/each}
    </div>
  </div>
```

Ajouter dans le `<style>` (après `.sec`) :

```css
  .subsec { display: flex; align-items: flex-end; height: 100%; padding: 0 12px 5px; font-size: 10px; text-transform: uppercase; letter-spacing: .04em; color: var(--text-dim); border-top: 1px solid var(--border); }
  .item.remote { opacity: .65; }
```

- [ ] **Step 3: Typecheck + vérification navigateur**

Run: `npm run check` — Expected: 0 errors, 0 warnings.
Run: `npm run dev` → ouvrir le BranchMenu. Attendu : « Branches · 2 007 », les 2004 locales d'abord, un séparateur « Remote » puis les 3 remote-only estompées ; le scroll reste fluide ; filtrer `maya` → seule la section Remote reste (avec son en-tête) ; filtrer `loot` → locales seulement, pas d'en-tête Remote ; cliquer `release/1.0-cut` → switch OK, la branche remonte dans la section locale.

- [ ] **Step 4: Commit**

```bash
git add src/lib/BranchMenu.svelte src/lib/mock.ts
git commit -m "feat(branches): Remote section in the branch menu (grouped, still virtualized)"
```

---

## Item 4 — Badge « protected » (lecture seule)

### Task 9: CAPTURE réelle — `branch info` + double constat (protected ? ahead/behind ?)

**Files:**
- Create: `src-tauri/tests/fixtures/branch_info.ndjson`
- Modify: `src-tauri/tests/fixtures/README.md`

- [ ] **Step 1: Capturer `branch info` sur la branche courante**

```powershell
$lore = "C:\Users\jimmy\bin\lore.exe"
$repo = "C:\Users\jimmy\lore-test-repo"
& $lore branch info main --repository $repo --json | Out-File -Encoding utf8NoBOM src-tauri\tests\fixtures\branch_info.ndjson
Get-Content src-tauri\tests\fixtures\branch_info.ndjson
```

Si le CLI répond « unknown command » (le sous-commande exacte n'est pas garantie) :

```powershell
& $lore branch --help
```

et réessayer avec le nom réel (`show`, `describe`, …). Si AUCUN équivalent n'existe : créer quand même le fichier de constat négatif — écrire la sortie d'erreur dans le README (Step 3) SANS committer de fixture, l'item 3 se replie (variante B, Task 15) et l'item 4 ne dépend plus que de la capture Task 5.

- [ ] **Step 2: Double constat**

Dans la capture (et celle de Task 5), noter :

| Question | Où chercher | Conséquence |
|---|---|---|
| **(A) protected** : un champ `protect*` existe-t-il dans `branch list` OU `branch info` ? | `Select-String -Path src-tauri\tests\fixtures\branch_list.ndjson,src-tauri\tests\fixtures\branch_info.ndjson -Pattern protect` | OUI dans branch list → Tasks 10–11 telles quelles (adapter le nom de champ). OUI seulement dans branch info → le badge devient une donnée du fetch lazy : déplacer le champ dans `BranchInfoDto` (Task 12) et le badge dans la partie lazy de la ligne (Task 15) ; Task 10 annulée, la partie tooltip Push de Task 11 lit alors le cache lazy de la branche courante. NON nulle part → **item 4 annulé proprement** : Tasks 10–11 non exécutées, documenter (Step 3), cocher leurs cases avec la mention « annulé — constat négatif Task 9 ». |
| **(B) ahead/behind** : des compteurs d'avance/retard par branche existent-ils dans `branch info` ? (noms candidats : `ahead`/`behind`, `localAhead`/`remoteAhead`, `revisionLocalNumber`/`revisionRemoteNumber` + flags…) | lecture de branch_info.ndjson | OUI → Tasks 12–15 variante A (adapter tag + noms de champs). NON (ou pas de commande) → Tasks 12 et 14 annulées, Task 13 réduite à `formatAheadBehind`, Task 15 variante B (ahead/behind de la SEULE branche courante dans l'en-tête du menu) + documenter l'écart. |

Noter aussi les champs annexes constatés (créateur ? date ?) — utiles au README, pas consommés dans ce lot.

- [ ] **Step 3: Documenter dans le README des fixtures**

Ajouter à la fin de `src-tauri/tests/fixtures/README.md` (adapter aux constats) :

```markdown

**`branch info <name> --json`** (branch_info.ndjson, capturé au lot P4) :
<tag exact ; champs constatés — ahead/behind : présents sous les noms `…` /
absents ; protected : présent sous le nom `…` / absent ; annexes : …>.
Constat P4 : item 4 (badge protected) <réalisé via branch list / réalisé via
branch info / ANNULÉ — flag non exposé en lecture par le CLI 0.8.x> ; item 3
(ahead/behind lazy) <variante A / VARIANTE B — repli sur la branche courante>.
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/tests/fixtures/branch_info.ndjson src-tauri/tests/fixtures/README.md
git commit -m "test(fixtures): capture lore branch info output (protected + ahead/behind survey)"
```

(Sans fixture — commande inexistante — ne committer que le README.)

### Task 10: Backend — flag `protected` dans `BranchDto` (CONDITIONNEL — constat A)

> **Si le constat A de la Task 9 est négatif (flag nulle part)** : cocher cette tâche et la Task 11 avec la mention « annulé — constat négatif Task 9 », ne rien implémenter, passer à la Task 12. **Si le flag n'existe que dans `branch info`** : sauter cette tâche, ajouter `protected: Option<bool>` à `BranchInfoDto` en Task 12 à la place.

**Files:**
- Modify: `src-tauri/src/commands.rs` (`BranchDto`, `branches_from`, `branches_tests`)

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `branches_tests` (adapter le nom de champ wire — le plan suppose `protected` ; remplacer partout par le nom constaté en Task 9) :

```rust
    #[test]
    fn protected_flag_folds_across_entries_and_defaults_false() {
        let sample = concat!(
            r#"{"tagName":"branchListEntry","data":{"location":"local","name":"main","latest":"a1","isCurrent":true,"archived":false,"protected":true}}"#, "\n",
            r#"{"tagName":"branchListEntry","data":{"location":"local","name":"feature/x","latest":"a2","isCurrent":false,"archived":false}}"#, "\n",
            r#"{"tagName":"complete","data":{"status":0}}"#, "\n",
        );
        let branches = branches_from(&parse_events(sample).unwrap());
        assert!(branches.iter().find(|b| b.name == "main").unwrap().is_protected);
        // Absent field (older CLI) => false, never a crash.
        assert!(!branches.iter().find(|b| b.name == "feature/x").unwrap().is_protected);
    }
```

Étendre aussi `parses_branch_list_fixture` avec une assertion sur la valeur constatée dans la capture (ex. `assert!(branches.iter().all(|b| !b.is_protected));` si aucune branche du repo de test n'est protégée — adapter).

- [ ] **Step 2: Vérifier que ça échoue**

Run: `cargo test --manifest-path src-tauri/Cargo.toml branches_tests`
Expected: erreur de compilation `no field is_protected`.

- [ ] **Step 3: Implémenter**

Dans `BranchDto`, ajouter :

```rust
    /// Lecture seule : les poussées directes sont rejetées côté serveur.
    /// Champ wire absent => false. (protect/unprotect restent au CLI — décision d'audit.)
    #[serde(rename = "protected")]
    pub is_protected: bool,
```

Dans `branches_from`, ajouter un `HashSet` `protected` alimenté comme `current` :

```rust
    let mut protected: std::collections::HashSet<String> = std::collections::HashSet::new();
```

dans la boucle (après le bloc `isCurrent`) :

```rust
        if d.get("protected").map(json_truthy).unwrap_or(false) {
            protected.insert(name.clone());
        }
```

et dans la construction finale :

```rust
            is_protected: protected.contains(&name),
```

- [ ] **Step 4: Vérifier que ça passe**

Run: `cargo test --manifest-path src-tauri/Cargo.toml branches_tests`
Expected: `test result: ok`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands.rs
git commit -m "feat(branches): read-only protected flag in the Branch DTO"
```

### Task 11: UI — badge cadenas + tooltip Push (`branchProtection.ts`) (CONDITIONNEL — constat A)

> **Constat A négatif** : tâche annulée (voir Task 10). **Flag seulement dans `branch info`** : garder `branchProtection.ts` et le tooltip Push tels quels, mais alimenter le badge depuis le cache lazy (Task 15) au lieu de `b.protected`.

**Files:**
- Create: `src/lib/branchProtection.ts`
- Test: `src/lib/branchProtection.test.ts`
- Modify: `src/lib/types.ts` (`Branch`), `src/lib/mock.ts` (`buildBranches`), `src/lib/BranchMenu.svelte`, `src/lib/TitleBar.svelte`

- [ ] **Step 1: Écrire les tests qui échouent**

Créer `src/lib/branchProtection.test.ts` :

```ts
import { describe, it, expect } from 'vitest'
import { pushTitle, PROTECTED_TIP } from './branchProtection'

describe('pushTitle', () => {
  it('offline wins over everything', () => {
    expect(pushTitle(true, true)).toBe('Server unreachable — push is unavailable')
  })
  it('a protected current branch gets the enriched tooltip (the push stays attempted)', () => {
    expect(pushTitle(false, true)).toBe(PROTECTED_TIP)
  })
  it('plain Push otherwise', () => {
    expect(pushTitle(false, false)).toBe('Push')
  })
})
```

Run: `npx vitest run src/lib/branchProtection.test.ts` — Expected: FAIL (`Cannot find module`).

- [ ] **Step 2: Implémenter le module pur**

Créer `src/lib/branchProtection.ts` :

```ts
/**
 * Lecture seule : le serveur fait autorité — l'app ne bloque jamais le push
 * elle-même, elle enrichit seulement le tooltip. protect/unprotect restent au CLI.
 */
export const PROTECTED_TIP = 'Protected — direct pushes are rejected by the server'

export function pushTitle(noRemote: boolean, protectedBranch: boolean): string {
  if (noRemote) return 'Server unreachable — push is unavailable'
  if (protectedBranch) return PROTECTED_TIP
  return 'Push'
}
```

Run: `npx vitest run src/lib/branchProtection.test.ts` — Expected: `Test Files  1 passed`.

- [ ] **Step 3: Type + mock**

Dans `src/lib/types.ts`, ajouter à l'interface `Branch` :

```ts
  /** Lecture seule : poussées directes rejetées par le serveur. Absent → non protégée. */
  protected?: boolean
```

Dans `src/lib/mock.ts` (`buildBranches`), marquer `main` :

```ts
    { name: 'main', current: true, location: 'local', protected: true },
```

- [ ] **Step 4: Badge dans le BranchMenu**

Dans `src/lib/BranchMenu.svelte`, ajouter l'import :

```ts
  import { PROTECTED_TIP } from './branchProtection'
```

Dans la ligne branche (Task 8), après `<span class="bn">{b.name}</span>` :

```svelte
              {#if b.protected}<span class="prot" title={PROTECTED_TIP}><Icon name="lock" size={11} /></span>{/if}
```

CSS (après `.bn`) :

```css
  .prot { display: inline-flex; align-items: center; color: var(--text-muted); flex: none; }
```

- [ ] **Step 5: Tooltip Push dans la TitleBar**

Dans `src/lib/TitleBar.svelte` :

```ts
  import { repo, sync, push, branches } from './repo.svelte'
  import { pushTitle } from './branchProtection'
```

Ajouter le dérivé (après `noRemote`) :

```ts
  const currentProtected = $derived(branches.list.find((b) => b.current)?.protected ?? false)
```

Sur le bouton Push, remplacer `title={noRemote ? 'Server unreachable — push is unavailable' : 'Push'}` par :

```svelte
title={pushTitle(noRemote, currentProtected)}
```

Le `disabled` du bouton ne change PAS (le push reste tenté — le serveur fait autorité).

- [ ] **Step 6: Typecheck + vérification navigateur**

Run: `npm run check` — Expected: 0 errors, 0 warnings.
Run: `npm run dev` : cadenas discret sur la ligne `main` du BranchMenu (tooltip « Protected — direct pushes are rejected by the server ») ; survoler le bouton Push avec `main` courante → même tooltip ; switcher sur `feature/loot` → tooltip Push redevient « Push ».

- [ ] **Step 7: Commit**

```bash
git add src/lib/branchProtection.ts src/lib/branchProtection.test.ts src/lib/types.ts src/lib/mock.ts src/lib/BranchMenu.svelte src/lib/TitleBar.svelte
git commit -m "feat(branches): read-only protected badge and enriched push tooltip"
```

---

## Item 3 — Ahead/behind lazy par branche

### Task 12: Backend — commande `lore_branch_info` (CONDITIONNEL — constat B, TDD)

> **Constat B négatif (pas d'ahead/behind, ou pas de commande)** : cocher cette tâche et la Task 14 « annulé — repli variante B (Task 15) », exécuter la Task 13 réduite (seulement `formatAheadBehind` + ses tests), puis la Task 15 variante B.

**Files:**
- Modify: `src-tauri/src/commands.rs` (après le bloc `lore_branches`/`branch_tips_from`, ~ligne 1041), `src-tauri/src/lib.rs`

- [ ] **Step 1: Écrire les tests qui échouent**

Tag `branchInfo` et champs `ahead`/`behind` = hypothèses du plan — **adapter aux noms constatés en Task 9** (y compris si l'ahead/behind doit se calculer depuis des numéros de révision local/remote, comme dans `status_from`). Ajouter dans `commands.rs` :

```rust
#[cfg(test)]
mod branch_info_tests {
    use super::*;
    use crate::lore::parse_events;

    #[test]
    fn parses_branch_info_fixture() {
        let events = parse_events(include_str!("../tests/fixtures/branch_info.ndjson")).unwrap();
        let info = branch_info_from(&events);
        // Adapter l'assertion aux valeurs réelles de la capture Task 9.
        assert!(info.ahead.is_some() || info.behind.is_some(), "the capture must yield at least one counter");
    }

    #[test]
    fn absent_fields_stay_none() {
        let events = parse_events(r#"{"tagName":"complete","data":{"status":0}}"#).unwrap();
        assert_eq!(branch_info_from(&events), BranchInfoDto { ahead: None, behind: None });
    }
}
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml branch_info_tests` — Expected: erreur de compilation (`branch_info_from` inconnu).

- [ ] **Step 2: Implémenter**

Ajouter dans `commands.rs` (après `branch_tips_from`) :

```rust
#[derive(Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BranchInfoDto {
    /// Révisions locales non poussées ; None quand le CLI n'expose pas le compteur.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ahead: Option<u64>,
    /// Révisions distantes non tirées ; None quand le CLI n'expose pas le compteur.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behind: Option<u64>,
}

/// Premier événement `branchInfo` (tag et champs pinnés par la capture Task 9 —
/// adapter si le constat diffère). Champs absents => None : la ligne du menu
/// reste simplement sans annotation, jamais de faux 0.
fn branch_info_from(events: &[LoreEvent]) -> BranchInfoDto {
    let d = events_with_tag(events, "branchInfo").into_iter().next();
    BranchInfoDto {
        ahead: d.and_then(|d| d.get("ahead").and_then(|v| v.as_u64())),
        behind: d.and_then(|d| d.get("behind").and_then(|v| v.as_u64())),
    }
}

/// Ahead/behind d'UNE branche, à la demande (fetch lazy au survol d'une ligne
/// du BranchMenu — jamais un appel par branche au montage : 2000+ branches).
#[tauri::command]
pub async fn lore_branch_info(repo_path: String, name: String) -> Result<BranchInfoDto, String> {
    blocking(move || {
        let events = run_lore(&["branch", "info", &name, "--repository", &repo_path])?;
        Ok(branch_info_from(&events))
    })
    .await
}
```

Dans `src-tauri/src/lib.rs`, ajouter à l'`invoke_handler` (après `commands::lore_archive_branch,`) :

```rust
        commands::lore_branch_info,
```

- [ ] **Step 3: Vérifier que ça passe**

Run: `cargo test --manifest-path src-tauri/Cargo.toml branch_info_tests` — Expected: `test result: ok` (2 tests).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(branches): lore_branch_info command (per-branch ahead/behind)"
```

### Task 13: Module pur `branchInfoCache.ts` — débounce + cache Map (TDD)

> **Constat B négatif** : n'implémenter que `formatAheadBehind` et son `describe` (le cache et ses tests sont inutiles sans fetch par branche).

**Files:**
- Create: `src/lib/branchInfoCache.ts`
- Test: `src/lib/branchInfoCache.test.ts`
- Modify: `src/lib/types.ts` (interface `BranchInfo`)

- [ ] **Step 1: Type partagé**

Dans `src/lib/types.ts`, ajouter (près de `Branch`) :

```ts
/** Champs lazy de `lore branch info` — absents quand le CLI ne les expose pas. */
export interface BranchInfo {
  ahead?: number
  behind?: number
}
```

- [ ] **Step 2: Écrire les tests qui échouent**

Créer `src/lib/branchInfoCache.test.ts` :

```ts
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { createBranchInfoCache, formatAheadBehind } from './branchInfoCache'

describe('createBranchInfoCache', () => {
  beforeEach(() => vi.useFakeTimers())
  afterEach(() => vi.useRealTimers())

  it('debounces: no fetch before the delay, one fetch after', async () => {
    const fetch = vi.fn(async () => ({ ahead: 1, behind: 2 }))
    const cache = createBranchInfoCache(fetch, () => {}, 150)
    cache.request('main')
    await vi.advanceTimersByTimeAsync(149)
    expect(fetch).not.toHaveBeenCalled()
    await vi.advanceTimersByTimeAsync(1)
    expect(fetch).toHaveBeenCalledOnce()
    expect(cache.get('main')).toEqual({ ahead: 1, behind: 2 })
  })

  it('cancel before the delay prevents the fetch (pointer left the row)', async () => {
    const fetch = vi.fn(async () => ({}))
    const cache = createBranchInfoCache(fetch, () => {}, 150)
    cache.request('main')
    cache.cancel('main')
    await vi.advanceTimersByTimeAsync(500)
    expect(fetch).not.toHaveBeenCalled()
  })

  it('a cached name is never refetched (session TTL)', async () => {
    const fetch = vi.fn(async () => ({ ahead: 0, behind: 0 }))
    const cache = createBranchInfoCache(fetch, () => {}, 150)
    cache.request('main')
    await vi.advanceTimersByTimeAsync(150)
    cache.request('main')
    await vi.advanceTimersByTimeAsync(500)
    expect(fetch).toHaveBeenCalledOnce()
  })

  it('a failed fetch is silent (no cached entry) and retryable', async () => {
    const fetch = vi.fn()
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValueOnce({ ahead: 3 })
    const cache = createBranchInfoCache(fetch, () => {}, 150)
    cache.request('main')
    await vi.advanceTimersByTimeAsync(150)
    expect(cache.get('main')).toBeUndefined()
    cache.request('main')
    await vi.advanceTimersByTimeAsync(150)
    expect(cache.get('main')).toEqual({ ahead: 3 })
  })

  it('onUpdate fires when a fetch lands (drives Svelte reactivity)', async () => {
    const onUpdate = vi.fn()
    const cache = createBranchInfoCache(async () => ({ behind: 1 }), onUpdate, 150)
    cache.request('x')
    await vi.advanceTimersByTimeAsync(150)
    expect(onUpdate).toHaveBeenCalledOnce()
  })

  it('clear drops the cache AND the pending timers (repo switch)', async () => {
    const fetch = vi.fn(async () => ({ ahead: 1 }))
    const cache = createBranchInfoCache(fetch, () => {}, 150)
    cache.request('a')
    await vi.advanceTimersByTimeAsync(150)
    cache.request('b')
    cache.clear()
    await vi.advanceTimersByTimeAsync(500)
    expect(fetch).toHaveBeenCalledOnce() // only 'a'; 'b' was cancelled by clear
    expect(cache.get('a')).toBeUndefined()
  })
})

describe('formatAheadBehind', () => {
  it('formats present, non-zero parts only', () => {
    expect(formatAheadBehind({ ahead: 2, behind: 5 })).toBe('↑2 ↓5')
    expect(formatAheadBehind({ ahead: 2, behind: 0 })).toBe('↑2')
    expect(formatAheadBehind({ behind: 3 })).toBe('↓3')
    expect(formatAheadBehind({ ahead: 0, behind: 0 })).toBeNull()
    expect(formatAheadBehind(undefined)).toBeNull()
  })
})
```

Run: `npx vitest run src/lib/branchInfoCache.test.ts` — Expected: FAIL (`Cannot find module`).

- [ ] **Step 3: Implémenter**

Créer `src/lib/branchInfoCache.ts` :

```ts
import type { BranchInfo } from './types'

type Fetcher = (name: string) => Promise<BranchInfo>

export interface BranchInfoCache {
  /** Planifie un fetch débouncé pour `name` ; no-op si déjà caché, en vol ou déjà planifié. */
  request(name: string): void
  /** Annule le débounce en attente (le pointeur a quitté la ligne avant le délai). */
  cancel(name: string): void
  /** Résultat caché, si le fetch a abouti. */
  get(name: string): BranchInfo | undefined
  /** Purge tout — cache, timers, vols (changement de repo). */
  clear(): void
}

/**
 * Cache du `branch info` lazy : débounce 150 ms au survol/focus, Map par nom de
 * branche, TTL session (jamais réinterrogé tant que le repo ne change pas).
 * L'échec d'un fetch est SILENCIEUX (enrichissement best-effort — la ligne
 * reste simplement sans annotation) et laisse le nom réinterrogeable.
 * `onUpdate` est appelé à chaque fetch abouti — le composant y branche un
 * compteur `$state` pour re-rendre les lignes.
 */
export function createBranchInfoCache(
  fetch: Fetcher,
  onUpdate: () => void,
  debounceMs = 150,
): BranchInfoCache {
  const cache = new Map<string, BranchInfo>()
  const inflight = new Set<string>()
  const timers = new Map<string, ReturnType<typeof setTimeout>>()
  return {
    request(name) {
      if (cache.has(name) || inflight.has(name) || timers.has(name)) return
      timers.set(name, setTimeout(async () => {
        timers.delete(name)
        inflight.add(name)
        try {
          const info = await fetch(name)
          cache.set(name, info)
          onUpdate()
        } catch {
          // best-effort — pas de toast, la ligne reste nue, retry possible
        } finally {
          inflight.delete(name)
        }
      }, debounceMs))
    },
    cancel(name) {
      const t = timers.get(name)
      if (t !== undefined) {
        clearTimeout(t)
        timers.delete(name)
      }
    },
    get: (name) => cache.get(name),
    clear() {
      for (const t of timers.values()) clearTimeout(t)
      timers.clear()
      cache.clear()
      inflight.clear()
    },
  }
}

/** « ↑2 ↓5 » discret en fin de ligne ; null quand il n'y a rien à montrer. */
export function formatAheadBehind(info?: BranchInfo): string | null {
  if (!info) return null
  const parts: string[] = []
  if (info.ahead) parts.push(`↑${info.ahead}`)
  if (info.behind) parts.push(`↓${info.behind}`)
  return parts.length > 0 ? parts.join(' ') : null
}
```

- [ ] **Step 4: Vérifier que ça passe**

Run: `npx vitest run src/lib/branchInfoCache.test.ts`
Expected: `Test Files  1 passed` (7 tests).

- [ ] **Step 5: Commit**

```bash
git add src/lib/branchInfoCache.ts src/lib/branchInfoCache.test.ts src/lib/types.ts
git commit -m "feat(branches): debounced per-branch info cache (pure, session TTL)"
```

### Task 14: API — `getBranchInfo` (types, tauri, mock) + singleton `branchInfo.svelte.ts` (CONDITIONNEL — constat B)

**Files:**
- Modify: `src/lib/types.ts` (`LoreApi`), `src/lib/tauri.ts`, `src/lib/mock.ts`
- Create: `src/lib/branchInfo.svelte.ts`

- [ ] **Step 1: Surface d'API**

Dans `src/lib/types.ts`, ajouter à `LoreApi` (après `getBranches`) :

```ts
  /** Ahead/behind d'UNE branche — fetch lazy au survol d'une ligne du menu. */
  getBranchInfo(repoPath: string, name: string): Promise<BranchInfo>
```

Dans `src/lib/tauri.ts` : ajouter `BranchInfo` à l'import de types ligne 5, puis (après `getBranches`) :

```ts
  getBranchInfo: (repoPath, name) => invoke<BranchInfo>('lore_branch_info', { repoPath, name }),
```

Dans `src/lib/mock.ts` : ajouter `BranchInfo` à l'import de types ligne 2, puis (après `getBranches`) :

```ts
  async getBranchInfo(_repoPath: string, name: string) {
    await delay(180)
    // Valeurs déterministes par nom : le survol montre de la variété sans churn.
    let h = 0
    for (const c of name) h = (h * 31 + c.charCodeAt(0)) >>> 0
    return { ahead: h % 4, behind: (h >> 2) % 6 } as BranchInfo
  },
```

- [ ] **Step 2: Singleton réactif**

Créer `src/lib/branchInfo.svelte.ts` (wiring non testé vitest — la logique est dans `branchInfoCache.ts`) :

```ts
import { api } from './api'
import { session } from './session.svelte'
import { createBranchInfoCache } from './branchInfoCache'
import type { BranchInfo } from './types'

// Bumpé à chaque fetch abouti : les lignes du menu relisent le cache.
export const branchInfoState = $state({ tick: 0 })

// Module-scope : le cache survit aux ouvertures/fermetures du menu (TTL session).
let cacheRepo: string | null = null
const cache = createBranchInfoCache(
  (name) => api.getBranchInfo(cacheRepo!, name),
  () => { branchInfoState.tick++ },
)

/** Fetch lazy débouncé au survol/focus d'une ligne. Purge au changement de repo. */
export function requestBranchInfo(name: string) {
  const repo = session.config.currentRepo
  if (!repo) return
  if (repo !== cacheRepo) {
    cache.clear()
    cacheRepo = repo
  }
  cache.request(name)
}

export function cancelBranchInfo(name: string) {
  cache.cancel(name)
}

export function branchInfoFor(name: string): BranchInfo | undefined {
  void branchInfoState.tick // dépendance réactive : re-lit après chaque fetch
  return cache.get(name)
}
```

- [ ] **Step 3: Typecheck**

Run: `npm run check` — Expected: 0 errors, 0 warnings.

- [ ] **Step 4: Commit**

```bash
git add src/lib/types.ts src/lib/tauri.ts src/lib/mock.ts src/lib/branchInfo.svelte.ts
git commit -m "feat(branches): getBranchInfo API surface and reactive lazy-info singleton"
```

### Task 15: UI — « ↑2 ↓5 » lazy au survol dans le BranchMenu

**Files:**
- Modify: `src/lib/BranchMenu.svelte`

- [ ] **Step 1 (variante A — constat B positif): Brancher le survol/focus**

Dans `src/lib/BranchMenu.svelte`, ajouter les imports :

```ts
  import { requestBranchInfo, cancelBranchInfo, branchInfoFor } from './branchInfo.svelte'
  import { formatAheadBehind } from './branchInfoCache'
```

Dans la branche `{:else}` de la ligne (Task 8), sous `{@const b = r.branch}`, ajouter :

```svelte
            {@const ab = formatAheadBehind(branchInfoFor(b.name))}
```

et remplacer le `<button class="item" …>` par (ajout des trois handlers ; le débounce 150 ms + le cache Map font qu'un simple passage de pointeur en scroll ne déclenche RIEN — jamais de rafale sur les 2000 branches du stress-test) :

```svelte
            <button class="item" class:cur={b.current} class:remote={b.location === 'remote'}
                    onpointerenter={() => requestBranchInfo(b.name)}
                    onpointerleave={() => cancelBranchInfo(b.name)}
                    onfocus={() => requestBranchInfo(b.name)}
                    onclick={() => (b.current ? onclose() : switchTo(b.name))} disabled={busy}>
              <span class="dot" style="background:{LANE[(winFirst + k) % LANE.length]}"></span>
              <span class="bn">{b.name}</span>
              {#if b.protected}<span class="prot" title={PROTECTED_TIP}><Icon name="lock" size={11} /></span>{/if}
              {#if ab}<span class="ab">{ab}</span>{/if}
              {#if b.current}<Icon name="check" size={14} />{/if}
            </button>
```

(Si la Task 11 a été annulée, omettre la ligne `.prot`.)

CSS (après `.bn`) :

```css
  .ab { flex: none; font-size: 10.5px; font-family: var(--font-mono); color: var(--text-dim); }
```

- [ ] **Step 1-bis (VARIANTE B — constat B négatif): ahead/behind de la seule branche courante dans l'en-tête**

À la place du Step 1 : ne PAS importer `branchInfo.svelte` (annulé). Importer :

```ts
  import { repo, refreshStatus, refreshBranches, branches } from './repo.svelte'
  import { formatAheadBehind } from './branchInfoCache'
```

Ajouter le dérivé (le status porte déjà les compteurs de la branche courante) :

```ts
  const curAB = $derived(formatAheadBehind({ ahead: repo.status?.localAhead ?? 0, behind: repo.status?.remoteAhead ?? 0 }))
```

et remplacer l'en-tête de section (Task 8) par :

```svelte
  <div class="sec">Branches · {branchCount.toLocaleString()}{#if curAB} · {currentName} {curAB}{/if}</div>
```

Documenter l'écart dans le README des fixtures (section Task 9) : « item 3 replié — `branch info` sans compteurs par branche ; l'UI montre l'ahead/behind de la branche courante (déjà connu via status) dans l'en-tête du menu ».

- [ ] **Step 2: Typecheck + vérification navigateur**

Run: `npm run check` — Expected: 0 errors, 0 warnings.
Run: `npm run dev` (variante A) : survoler une ligne du BranchMenu → ~330 ms plus tard (150 débounce + 180 mock) « ↑2 ↓5 » apparaît en fin de ligne ; re-survoler la même ligne → instantané (cache, pas de re-fetch — vérifier dans l'onglet réseau/console qu'aucun appel ne repart) ; traverser 20 lignes rapidement à la molette → aucune rafale ; changer de repo → annotations reparties de zéro.

- [ ] **Step 3: Commit**

```bash
git add src/lib/BranchMenu.svelte
git commit -m "feat(branches): lazy per-branch ahead/behind on hover in the branch menu"
```

---

## Item 5 — Panneau « About repository »

### Task 16: CAPTURE réelle — `repository info --json` (fixture + constat)

**Files:**
- Create: `src-tauri/tests/fixtures/repo_info.ndjson`
- Modify: `src-tauri/tests/fixtures/README.md`

- [ ] **Step 1: Capturer**

```powershell
$lore = "C:\Users\jimmy\bin\lore.exe"
$repo = "C:\Users\jimmy\lore-test-repo"
& $lore repository info --repository $repo --json | Out-File -Encoding utf8NoBOM src-tauri\tests\fixtures\repo_info.ndjson
Get-Content src-tauri\tests\fixtures\repo_info.ndjson
```

Si « unknown command », consulter `& $lore repository --help` et réessayer (la commande peut vouloir l'URL serveur en argument, comme `repository list`). Si AUCUNE forme n'aboutit : documenter dans le README, le panneau About se construit alors uniquement sur le contexte local (nom du dossier, chemin, serveur, branche, révision — déjà connus) et les Tasks 17 et la partie `info` de la Task 18 sont annulées proprement (cocher avec mention).

- [ ] **Step 2: Constat**

Noter : le tag exact de l'événement (hypothèse : `repositoryInfo`), et les champs réels — id ? nom ? taille (octets ?) ? serveur ? autres (dates, compte de révisions…) ? Les champs utiles découverts s'ajoutent en Task 17 (DTO) et Task 18 (`aboutRows`) sur le même modèle Option/ligne masquée.

- [ ] **Step 3: Documenter dans le README des fixtures**

Ajouter à la fin de `src-tauri/tests/fixtures/README.md` (adapter) :

```markdown

**`repository info --json`** (repo_info.ndjson, capturé au lot P4) : <tag exact> —
champs constatés : <id (string) ; name (string) ; size (u64, octets) ; …>.
Tout champ absent => `None` dans `RepositoryInfoDto` => ligne masquée dans le
panneau About (défaut sûr).
```

- [ ] **Step 4: Commit**

```bash
git add src-tauri/tests/fixtures/repo_info.ndjson src-tauri/tests/fixtures/README.md
git commit -m "test(fixtures): capture lore repository info output"
```

### Task 17: Backend — commande `lore_repository_info` (TDD)

**Files:**
- Modify: `src-tauri/src/commands.rs` (après le bloc `repositories_from`/`lore_repositories`, ~ligne 258), `src-tauri/src/lib.rs`

- [ ] **Step 1: Écrire les tests qui échouent**

Ajouter dans `commands.rs` (adapter tag/champs/valeurs à la capture Task 16) :

```rust
#[cfg(test)]
mod repository_info_tests {
    use super::*;
    use crate::lore::parse_events;

    #[test]
    fn parses_repository_info_fixture() {
        let events = parse_events(include_str!("../tests/fixtures/repo_info.ndjson")).unwrap();
        let info = repository_info_from(&events);
        assert!(info.id.is_some() || info.name.is_some(), "the captured fixture must carry an id or a name");
    }

    #[test]
    fn empty_stream_yields_all_none() {
        let events = parse_events(r#"{"tagName":"complete","data":{"status":0}}"#).unwrap();
        assert_eq!(repository_info_from(&events), RepositoryInfoDto::default());
    }
}
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml repository_info_tests` — Expected: erreur de compilation.

- [ ] **Step 2: Implémenter**

Ajouter (après `lore_repositories` ; adapter le tag `repositoryInfo` et les noms de champs à la capture, ajouter les champs utiles constatés sur le même modèle `Option`) :

```rust
#[derive(Serialize, PartialEq, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryInfoDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Taille du dépôt en octets, si le CLI l'expose.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

/// Premier événement `repositoryInfo` (tag pinné par la capture Task 16).
/// Tout champ absent reste None — la ligne correspondante est masquée côté UI.
fn repository_info_from(events: &[LoreEvent]) -> RepositoryInfoDto {
    let d = events_with_tag(events, "repositoryInfo").into_iter().next();
    RepositoryInfoDto {
        id: d.and_then(|d| d.get("id").and_then(|v| v.as_str()).map(String::from)),
        name: d.and_then(|d| d.get("name").and_then(|v| v.as_str()).map(String::from)),
        size: d.and_then(|d| d.get("size").and_then(|v| v.as_u64())),
    }
}

/// Métadonnées du dépôt courant, pour le panneau « About repository » (read-only).
#[tauri::command]
pub async fn lore_repository_info(repo_path: String) -> Result<RepositoryInfoDto, String> {
    blocking(move || {
        let events = run_lore(&["repository", "info", "--repository", &repo_path])?;
        Ok(repository_info_from(&events))
    })
    .await
}
```

Dans `src-tauri/src/lib.rs`, ajouter à l'`invoke_handler` (après `commands::lore_repositories,` ligne 34) :

```rust
        commands::lore_repository_info,
```

- [ ] **Step 3: Vérifier que ça passe**

Run: `cargo test --manifest-path src-tauri/Cargo.toml repository_info_tests` — Expected: `test result: ok` (2 tests).

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(repo): lore_repository_info command for the About panel"
```

### Task 18: Module pur `aboutFields.ts` + surface d'API (TDD)

**Files:**
- Create: `src/lib/aboutFields.ts`
- Test: `src/lib/aboutFields.test.ts`
- Modify: `src/lib/types.ts` (`RepositoryInfo`, `LoreApi`), `src/lib/tauri.ts`, `src/lib/mock.ts`

- [ ] **Step 1: Type + API**

Dans `src/lib/types.ts`, ajouter (près de `RepoEntry`) :

```ts
/** Champs de `lore repository info` — tous optionnels : un champ absent masque sa ligne. */
export interface RepositoryInfo {
  id?: string
  name?: string
  /** Taille du dépôt en octets. */
  size?: number
}
```

et à `LoreApi` (après `listRepos`) :

```ts
  /** Métadonnées du dépôt ouvert, pour le panneau About (best-effort : peut rejeter hors-ligne). */
  getRepositoryInfo(repoPath: string): Promise<RepositoryInfo>
```

Dans `src/lib/tauri.ts` : ajouter `RepositoryInfo` à l'import de types, puis :

```ts
  getRepositoryInfo: (repoPath) => invoke<RepositoryInfo>('lore_repository_info', { repoPath }),
```

Dans `src/lib/mock.ts` : ajouter `RepositoryInfo` à l'import de types, puis (après `listRepos`) :

```ts
  async getRepositoryInfo(_repoPath: string) {
    await delay(180)
    return { id: '019f2e14006f7870a7b27df367c78b72', name: 'game-main', size: 1288490188 } as RepositoryInfo
  },
```

- [ ] **Step 2: Écrire les tests qui échouent**

Créer `src/lib/aboutFields.test.ts` :

```ts
import { describe, it, expect } from 'vitest'
import { aboutRows } from './aboutFields'

const ctx = {
  repoPath: 'C:/game/main-repo',
  serverUrl: 'lore://lore.example.com:41337',
  branch: 'main',
  revisionNumber: 42,
}

describe('aboutRows', () => {
  it('renders the full set when everything is known', () => {
    const rows = aboutRows({ id: 'abc123', name: 'game-main', size: 4 * 1024 * 1024 }, ctx)
    expect(rows.map((r) => r.label)).toEqual(
      ['Name', 'Repository id', 'Local path', 'Server', 'Current branch', 'Revision', 'Size'])
    expect(rows.find((r) => r.label === 'Repository id')).toMatchObject({ value: 'abc123', copyable: true })
    expect(rows.find((r) => r.label === 'Local path')).toMatchObject({ value: 'C:/game/main-repo', revealPath: 'C:/game/main-repo' })
    expect(rows.find((r) => r.label === 'Revision')?.value).toBe('#42')
    expect(rows.find((r) => r.label === 'Size')?.value).toBe('4.0 MB')
  })

  it('hides absent fields instead of showing blanks (safe defaults)', () => {
    const rows = aboutRows(null, { ...ctx, serverUrl: null, revisionNumber: null })
    expect(rows.map((r) => r.label)).toEqual(['Name', 'Local path', 'Current branch'])
    // Server info entirely missing: the name falls back to the folder name.
    expect(rows[0].value).toBe('main-repo')
  })

  it('renders nothing without a repo', () => {
    expect(aboutRows(null, { repoPath: null, serverUrl: null, branch: null, revisionNumber: null })).toEqual([])
  })
})
```

Run: `npx vitest run src/lib/aboutFields.test.ts` — Expected: FAIL (`Cannot find module`).

- [ ] **Step 3: Implémenter**

Créer `src/lib/aboutFields.ts` (compléter avec les champs utiles constatés en Task 16, même modèle) :

```ts
import { fmtSize } from './sizeFormat'
import type { RepositoryInfo } from './types'

export interface AboutRow {
  label: string
  value: string
  /** Bouton Copy (ex. l'id du dépôt). */
  copyable?: boolean
  /** Bouton Reveal ouvrant le gestionnaire de fichiers sur ce chemin absolu. */
  revealPath?: string
}

export interface AboutContext {
  repoPath: string | null
  serverUrl: string | null
  branch: string | null
  revisionNumber: number | null
}

/**
 * Lignes du panneau About. Donnée absente = ligne masquée, jamais un blanc ni
 * un faux zéro (défauts sûrs — `info` peut être null si le serveur est down).
 */
export function aboutRows(info: RepositoryInfo | null, ctx: AboutContext): AboutRow[] {
  const rows: AboutRow[] = []
  const name = info?.name ?? ctx.repoPath?.split(/[\\/]/).pop() ?? null
  if (name) rows.push({ label: 'Name', value: name })
  if (info?.id) rows.push({ label: 'Repository id', value: info.id, copyable: true })
  if (ctx.repoPath) rows.push({ label: 'Local path', value: ctx.repoPath, revealPath: ctx.repoPath })
  if (ctx.serverUrl) rows.push({ label: 'Server', value: ctx.serverUrl })
  if (ctx.branch) rows.push({ label: 'Current branch', value: ctx.branch })
  if (ctx.revisionNumber != null && ctx.revisionNumber > 0) rows.push({ label: 'Revision', value: `#${ctx.revisionNumber}` })
  if (info?.size != null && info.size > 0) rows.push({ label: 'Size', value: fmtSize(info.size) })
  return rows
}
```

- [ ] **Step 4: Vérifier que ça passe**

Run: `npx vitest run src/lib/aboutFields.test.ts` — Expected: `Test Files  1 passed` (3 tests).
Run: `npm run check` — Expected: 0 errors, 0 warnings.

- [ ] **Step 5: Commit**

```bash
git add src/lib/aboutFields.ts src/lib/aboutFields.test.ts src/lib/types.ts src/lib/tauri.ts src/lib/mock.ts
git commit -m "feat(about): repository-info API surface and pure About rows"
```

### Task 19: UI — `AboutRepo.svelte` + entrée dans le RepoSwitcher + wiring TitleBar

**Décision de design (posée au plan)** : l'entrée vit dans le **RepoSwitcher** (le menu « repository » — c'est là qu'on pense dépôt ; l'AvatarMenu est le menu « compte »). Le panneau est un modal centré à scrim, fermé par Escape et clic dehors (pattern des listeners de ContextMenu.svelte), monté par la TitleBar pour survivre à la fermeture du menu.

**Files:**
- Create: `src/lib/AboutRepo.svelte`
- Modify: `src/lib/RepoSwitcher.svelte`, `src/lib/TitleBar.svelte`

- [ ] **Step 1: Créer le composant `AboutRepo.svelte`**

```svelte
<script lang="ts">
  import { api } from './api'
  import { session } from './session.svelte'
  import { repo } from './repo.svelte'
  import { aboutRows } from './aboutFields'
  import type { RepositoryInfo } from './types'
  import Icon from './Icon.svelte'

  let { onclose }: { onclose: () => void } = $props()

  let info = $state<RepositoryInfo | null>(null)
  let copied = $state('')

  // Best-effort : les lignes locales (chemin, branche, révision) rendent tout de
  // suite ; les champs serveur se remplissent quand l'appel aboutit. Un échec
  // (hors-ligne) laisse simplement le panneau local-only — pas de toast.
  $effect(() => {
    const p = session.config.currentRepo
    if (!p) return
    api.getRepositoryInfo(p).then((r) => { info = r }).catch(() => { /* rows stay local-only */ })
  })

  const rows = $derived(aboutRows(info, {
    repoPath: session.config.currentRepo,
    serverUrl: session.config.serverUrl,
    branch: repo.status?.branch ?? null,
    revisionNumber: repo.status?.revisionNumber ?? null,
  }))

  // Fermeture Escape / pointerdown hors du panneau (pattern ContextMenu.svelte).
  let panelEl = $state<HTMLDivElement>()
  $effect(() => {
    function onDoc(e: PointerEvent) {
      if (panelEl && !panelEl.contains(e.target as Node)) onclose()
    }
    function onKey(e: KeyboardEvent) {
      if (e.key === 'Escape') { e.stopPropagation(); onclose() }
    }
    document.addEventListener('pointerdown', onDoc)
    document.addEventListener('keydown', onKey)
    return () => {
      document.removeEventListener('pointerdown', onDoc)
      document.removeEventListener('keydown', onKey)
    }
  })

  async function copy(value: string, label: string) {
    try {
      await navigator.clipboard.writeText(value)
      copied = label
      setTimeout(() => (copied = ''), 1500)
    } catch { /* clipboard denied — silent */ }
  }
</script>

<div class="scrim">
  <div class="panel" bind:this={panelEl} role="dialog" aria-modal="true" aria-label="About repository">
    <div class="title"><Icon name="info" size={16} /> About repository</div>
    {#each rows as r (r.label)}
      <div class="row">
        <span class="lbl">{r.label}</span>
        <span class="val" title={r.value}>{r.value}</span>
        {#if r.copyable}
          <button class="mini" onclick={() => copy(r.value, r.label)}>{copied === r.label ? 'Copied' : 'Copy'}</button>
        {/if}
        {#if r.revealPath}
          <button class="mini" onclick={() => api.revealPath(r.revealPath!)}>Reveal</button>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .scrim { position: fixed; inset: 0; background: rgba(0, 0, 0, .35); z-index: 90; display: grid; place-items: center; }
  .panel { width: 420px; max-width: calc(100vw - 40px); background: var(--panel); border: 1px solid var(--border-strong); border-radius: 10px; box-shadow: 0 12px 30px rgba(0, 0, 0, .45); padding: 14px 16px 12px; }
  .title { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 500; margin-bottom: 10px; }
  .title :global(svg) { color: var(--text-muted); }
  .row { display: flex; align-items: center; gap: 8px; padding: 5px 0; font-size: 12.5px; }
  .lbl { width: 110px; flex: none; color: var(--text-muted); font-size: 11.5px; }
  .val { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-family: var(--font-mono); font-size: 12px; }
  .mini { flex: none; padding: 2px 8px; font-size: 10.5px; color: var(--text-muted); background: var(--bg); border: 1px solid var(--border); border-radius: 5px; }
  .mini:hover { color: var(--text); background: var(--panel-hover); }
</style>
```

- [ ] **Step 2: Entrée dans le RepoSwitcher**

Dans `src/lib/RepoSwitcher.svelte`, remplacer la déclaration des props :

```ts
  let { onclose, onabout }: { onclose: () => void; onabout?: () => void } = $props()
```

et, dans le mode `list`, juste après la fermeture du `<div class="list">…</div>` (avant le `{:else}`) :

```svelte
    {#if session.config.currentRepo}
      <div class="div"></div>
      <button class="action" onclick={() => onabout?.()}>
        <Icon name="info" size={15} /> About repository
      </button>
    {/if}
```

Ajouter au `<style>` :

```css
  .div { height: 1px; background: var(--border); margin: 6px 0; }
```

- [ ] **Step 3: Wiring TitleBar**

Dans `src/lib/TitleBar.svelte` :

```ts
  import AboutRepo from './AboutRepo.svelte'
```

```ts
  let aboutOpen = $state(false)
```

Remplacer la ligne du RepoSwitcher :

```svelte
    {#if repoOpen}<RepoSwitcher onclose={() => (repoOpen = false)} onabout={() => { repoOpen = false; aboutOpen = true }} />{/if}
```

et ajouter à la fin du `<header>` (après la `avatarzone`) :

```svelte
  {#if aboutOpen}<AboutRepo onclose={() => (aboutOpen = false)} />{/if}
```

- [ ] **Step 4: Typecheck + vérification navigateur**

Run: `npm run check` — Expected: 0 errors, 0 warnings.
Run: `npm run dev` : menu repo → « About repository » → panneau modal ; lignes attendues (mock) : Name `game-main`, Repository id `019f2e14…` avec bouton Copy (clic → « Copied » 1,5 s, presse-papiers rempli), Local path avec bouton Reveal (no-op navigateur), Server, Current branch `main`, Revision `#5`, Size `1.2 GB` ; Escape ferme ; clic sur le scrim ferme ; sans repo ouvert, l'entrée n'apparaît pas dans le RepoSwitcher.

- [ ] **Step 5: Commit**

```bash
git add src/lib/AboutRepo.svelte src/lib/RepoSwitcher.svelte src/lib/TitleBar.svelte
git commit -m "feat(about): About repository panel from the repo switcher"
```

---

## Task 20: Vérification finale (suites + parcours navigateur + captures)

**Files:** aucun nouveau (mise à jour du statut de ce plan seulement).

- [ ] **Step 1: Suites complètes**

```powershell
npx vitest run
cargo test --manifest-path src-tauri/Cargo.toml
npm run check
```

Expected : vitest `Test Files  21 passed` (16 existants + statusSummary, branchGrouping, branchProtection, branchInfoCache, aboutFields — ajuster si des tâches conditionnelles ont été annulées), 0 failed ; cargo `test result: ok` (≈98 existants + les nouveaux status/branches/branch_info/repository_info), 0 failed ; svelte-check 0 errors 0 warnings.

- [ ] **Step 2: Parcours navigateur mock complet**

`npm run dev` → http://localhost:5173, dérouler et noter chaque point :

1. **Compteurs** : en-tête Changes « 10 files +3 ~6 −1 » colorés ; après un commit de tout, compteurs disparus.
2. **Remote** : BranchMenu → locales puis « Remote » (3 branches estompées) ; filtre `maya` → section Remote seule avec en-tête ; filtre `loot` → pas d'en-tête ; switch d'une remote-only OK (elle devient locale) ; scroll fluide sur ~2000 lignes ; bouton Archive absent des lignes remote.
3. **Protected** (si item réalisé) : cadenas sur `main` + tooltip ; tooltip du bouton Push enrichi quand `main` est courante, redevenu « Push » sur une autre branche ; bouton Push jamais désactivé par la protection.
4. **Ahead/behind lazy** (variante A) : survol → « ↑x ↓y » après ~330 ms ; re-survol instantané sans nouvel appel ; molette rapide sur 20 lignes → pas de rafale ; changement de repo → cache repart de zéro. (Variante B : en-tête « Branches · N · main ↑x ↓y ».)
5. **About** : panneau depuis le RepoSwitcher, champs mock complets, Copy id → « Copied », Escape et clic-dehors ferment, entrée absente sans repo ouvert.

- [ ] **Step 3: Checklist des captures réelles**

Vérifier que les fixtures sont committées et référencées :

```powershell
git log --oneline -- src-tauri/tests/fixtures/
Get-ChildItem src-tauri\tests\fixtures\
```

Expected : `branch_list.ndjson`, `branch_info.ndjson` (sauf commande inexistante — alors le constat est dans le README), `repo_info.ndjson` présents ; README des fixtures documentant `repositoryStatusSummary`, `branchListEntry.location`, le constat protected (positif OU négatif), les champs `branch info` (ou le repli variante B) et `repository info`.

- [ ] **Step 4 (optionnel mais recommandé): Fumée réelle Tauri**

`cargo tauri dev` (ou `npm run tauri dev`) sur `C:\Users\jimmy\lore-test-repo` : compteurs de l'en-tête cohérents avec un fichier modifié + un ajouté ; BranchMenu montre les branches réelles (la section Remote n'apparaît que s'il existe une branche remote-only) ; About affiche l'id réel du dépôt et le chemin local avec Reveal fonctionnel.

- [ ] **Step 5: Marquer le plan exécuté**

Ajouter sous le header de ce plan un bloc « STATUT : EXÉCUTÉ ET VÉRIFIÉ le <date> » listant : compte de tests par suite, points du parcours navigateur validés, constats des captures (dont toute annulation/repli d'item — protected et/ou variante B), et toute déviation consciente. Commit :

```bash
git add docs/superpowers/plans/2026-07-11-lore-desktop-p4-visibility.md
git commit -m "docs: mark P4 visibility plan executed and verified"
```

---

## Self-review (fait à l'écriture du plan)

- **Couverture spec** : item 1 → Tasks 1–4 (capture: déjà faite en P1, constaté ; backend `StatusResult.summary` optionnel ; UI colhead couleurs `--added/--modified/--deleted`, masqué si absent/zéro ; mock cohérent). Item 2 → Tasks 5–8 (capture location ; DTO ; groupes local puis « Remote » estompé, switch permis, filtre sur les deux groupes ; mock 3 remote-only ; virtualisation préservée pour les 2000 branches). Item 4 → Tasks 9–11 (capture double constat, badge + tooltip Push enrichi, push jamais bloqué, annulation propre prévue). Item 3 → Tasks 12–15 (capture partagée Task 9 ; fetch lazy survol/focus débouncé 150 ms + Map par nom TTL session en module pur testé ; « ↑2 ↓5 » discret ; repli variante B documenté). Item 5 → Tasks 16–19 (capture ; commande ; panneau modal read-only : nom, id copiable, chemin + Reveal, serveur, branche, révision, taille ; Escape/clic dehors ; choix RepoSwitcher tranché). Ordre 1→2→4→3→5 respecté ; Task 20 = suites + navigateur + captures.
- **Hors périmètre respecté** : aucune écriture nouvelle (protect/unprotect, metadata, `repository config`), pas de recherche, pas de tri avancé, pas de pagination du branch info ; l'Archive n'est pas étendue aux branches remote-only.
- **Placeholders** : aucun TBD/TODO ; chaque étape code porte son code ; les seuls « adapter » sont les pins de capture, explicitement bornés (tag/nom de champ) avec le comportement par défaut écrit.
- **Cohérence de types** : `StatusSummaryDto{adds,mods,dels}` ↔ `StatusResult.summary` ↔ `summaryParts` ; `BranchDto.location: String` ↔ `Branch.location?: 'local'|'remote'` (serde émet la chaîne) ; `is_protected` renommé wire `protected` ↔ `Branch.protected?` ; `BranchInfoDto{ahead,behind}: Option<u64>` ↔ `BranchInfo{ahead?,behind?}` (skip_serializing_if ⇒ clés absentes) ; `RepositoryInfoDto` ↔ `RepositoryInfo` idem ; `formatAheadBehind` défini Task 13, consommé Tasks 15 (A et B).

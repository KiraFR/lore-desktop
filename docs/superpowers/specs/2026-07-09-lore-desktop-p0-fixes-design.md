# Lore Desktop — Passe P0 : bugs + identité + offline + binaires

**Date :** 2026-07-09
**Statut :** Design validé — à implémenter
**Source :** audit `2026-07-09-lore-desktop-feature-audit.md` (§1 bugs, §5 P0), choix utilisateur du 2026-07-09.

## Périmètre

Huit corrections/features P0, indépendantes entre elles, sur l'app existante :

1. Sign out réel (B1)
2. Description de commit prise en compte (B2)
3. « + Lock a file… » fonctionnel + toggle lock dans FilePreview (B3)
4. Fin de pagination History (B4)
5. Retrait des compteurs +/~/− des lignes History (B5)
6. Identité réelle + menu avatar + display name
7. Indicateur offline / session expirée dans la StatusBar
8. Détection binaire : liste étendue + sniff de contenu

**Hors périmètre :** intégration/extension Unreal Engine pour les locks (passe future, notée par l'utilisateur) ; tout le reste des P1/P2 de l'audit.

---

## 1. Sign out réel (B1)

- **Rust** : nouvelle commande `lore_sign_out` → `run_lore(&["auth", "logout"])`.
- **`tauri.ts`** : override `signOut: () => invoke('lore_sign_out')` (aujourd'hui hérité du mock — c'est le bug).
- **UX** : le clic sur l'avatar n'appelle plus `signOut` directement ; il ouvre le menu avatar (§6). « Sign out » est une entrée du menu.
- `session.signOut()` inchangé côté séquence (api.signOut → signedIn=false).

## 2. Description de commit (B2)

- `Changes.svelte` : `let description = $state('')`, `bind:value` sur le textarea.
- Message envoyé : `message.trim()` seul si description vide, sinon `message.trim() + "\n\n" + description.trim()`.
- Reset des deux champs après commit.
- Le mock reste inchangé (il accepte n'importe quel message) ; pas de changement d'API.

## 3. Lock préventif (B3)

- **API** : nouvelle méthode `pickRepoFile(repoPath): Promise<string | null>` dans `LoreApi` — dialog natif en mode fichier (`open({ directory: false, defaultPath: repoPath })`), retourne le chemin absolu ou null.
  - Mock : retourne un chemin factice dans le repo.
- **Validation** : le chemin choisi doit être **dans** le repo (préfixe `repoPath` après normalisation des séparateurs) ; sinon toast « Ce fichier n'est pas dans le dépôt ». Convertir en chemin relatif (séparateurs `/`) avant `setLock`.
- **Locks.svelte** : le bouton « + Lock a file… » appelle pickRepoFile → validation → `setLock(rel, true)` ; état busy pendant l'opération.
- **FilePreview.svelte** : dans l'en-tête (à côté de Discard), un bouton Lock/Unlock pour le fichier affiché : « Lock » si non verrouillé, « Unlock » si `lockedBy === 'you'`, badge non cliquable « Locked by X » sinon. Réutilise `setLock` existant.

## 4. Fin de pagination History (B4)

- `commands.rs::lore_history` : mémoriser `requested = length` ; après `history_from`, si le nombre d'entrées brutes de la page `< requested`, forcer `page.next_cursor = None`.
- Test unitaire Rust sur le cas « page courte ⇒ cursor None » et « page pleine ⇒ cursor Some(dernier) ».
- (Le mock a déjà un `nextCursor: null` en fin de liste — comportement aligné.)

## 5. Compteurs de lignes History (B5)

- `History.svelte` : retirer le `<span class="counts">` des lignes (et le CSS orphelin). Le panneau de détail garde ses compteurs (calculés depuis `detailFiles`, exacts).
- `types.ts` / DTO Rust / mock : supprimer `adds/mods/dels` de `Commit` (champ mort ensuite) — et retirer leur génération du mock pour garder la parité.

## 6. Identité + menu avatar

### Données

- **Rust** : nouvelle commande `lore_identity(repo_path) -> { id, email }` → `run_lore(&["auth", "info", "--repository", &repo_path])`, événement `authUserInfo { id, name }` (name = email). Erreur ⇒ `Err` (le front la traite en silence).
  - Contrainte vérifiée : `auth info` ne répond **que** dans un working copy — pas d'identité sans repo ouvert.
- **`types.ts`** : `Identity { id: string; email: string }` ; `LoreApi.getIdentity(repoPath)`. Mock : retourne `{ id: 'mock-user', email: 'jane.doe@studio.dev' }`.
- **`session.svelte.ts`** : `session.identity: Identity | null` ; `AppConfig.displayName?: string | null` (persisté dans config.json ; le Rust `AppConfigDto` gagne `display_name: Option<String>` avec `#[serde(default)]`).
- Chargement : un `loadIdentity()` dédié, appelé quand `currentRepo` change (effet dans App.svelte à côté de `refreshStatus`), silencieux et best-effort. Re-fetch à chaque changement de repo (même serveur ⇒ même identité, mais l'appel est peu coûteux et reste toujours juste).

### Initiales / nom affiché

Helper pur `initialsFor(displayName, email)` dans un module testable (`identity.ts`) :

- `displayName` non vide → premières lettres des 2 premiers mots (« Jimmy D. » → « JD ») ; un seul mot → 2 premières lettres.
- Sinon email : partie locale, découpée sur `. _ -` ; 2 mots+ → initiales ; sinon 2 premières lettres (« jimmydelannoy » → « JI »).
- Rien → « ? ».

`displayNameFor(displayName, email)` : displayName sinon partie locale de l'email sinon « Not signed in ».

### Menu avatar (TitleBar)

Popover au pattern BranchMenu/RepoSwitcher (zone + clic extérieur pour fermer), ancré à droite :

- En-tête : avatar (initiales), display name, email en dessous (muted).
- Champ « Display name » : input pré-rempli, enregistré sur blur/Enter → `config.displayName` + `saveConfig`.
- Séparateur, puis « Sign out » (rouge) → `signOut()`.
- Sans repo ouvert (pas d'identité) : en-tête « Not signed in to a repository » + email masqué ; Sign out reste disponible.

### Propagation « you »

- `History.svelte` : `avatar()` et `shortName()` utilisent l'identité — un auteur égal à `session.identity.email` s'affiche « you » avec les initiales de l'utilisateur ; plus aucun « JD » en dur.
- TitleBar : initiales dynamiques.
- (Le backend locks mappe déjà « you » via l'id — inchangé.)

## 7. Indicateur offline / session

- **Rust** `StatusResultDto` : + `remote_available: bool`, + `remote_authorized: bool` (depuis `repositoryStatusRevision.remoteAvailable/remoteAuthorized`, `json_truthy`, défaut `true` si absent), + `revision_number: u64`.
- **`types.ts`** `StatusResult` : + `remoteAvailable: boolean`, `remoteAuthorized: boolean`, `revisionNumber: number`. Mock : `true/true` + un numéro.
- **StatusBar** (3 états, maquette validée) :
  - normal : « ✓ Synced · rev N » ;
  - `!remoteAvailable` : point ambre + « Offline — changes stay local » ;
  - `remoteAvailable && !remoteAuthorized` : point rouge + « Session expired » + bouton « Sign in again » → `signOut()` puis retour à l'écran SignIn.
- **TitleBar** : Sync et Push `disabled` quand `!remoteAvailable || !remoteAuthorized`, avec `title` explicatif.

## 8. Détection binaire

- **Rust** `is_binary_path` → `is_binary(repo_path, rel_path, action)` :
  1. **Liste étendue** (fast path) : uasset, umap, png, fbx, wav, tga, psd, dds, exr, hdr, tif, tiff, jpg, jpeg, webp, blend, ma, mb, max, sbs, sbsar, spp, ztl, obj, abc, gltf, glb, ogg, mp3, flac, bank, anim, zip, pak, bin, dll, exe, so, dylib.
  2. Extension inconnue (ou absente) : lire les premiers 8 Ko du fichier local (`repo_path/rel_path`) ; **octet NUL ⇒ binaire**. Fichier illisible (supprimé, action delete) ⇒ retomber sur la liste seule (donc texte par défaut).
  - Tests unitaires : extension connue, sniff NUL positif, fichier texte, fichier absent.
- Utilisé par `status_from` et `merge_conflicts_from` (les deux appelants actuels) — signature adaptée pour passer `repo_path`.
- **FilePreview `TYPES`** : compléter les libellés (dds/exr/tga « Texture », blend « Blender scene », ma/mb « Maya scene », sbs/sbsar « Substance », ogg/mp3/flac/bank « Audio », abc/gltf/glb/obj « Mesh », pak « Package »…).

---

## Erreurs & cas limites

- `lore auth info` en échec (offline au premier chargement) : identité absente, avatar « ? », pas de toast — l'indicateur offline (§7) explique déjà la situation.
- Lock d'un fichier hors repo ou picker annulé : toast / no-op, pas d'état bloqué.
- Sniff binaire sur gros fichier : lecture bornée à 8 Ko, jamais le fichier entier.
- Sign out pendant une opération : les boutons d'action sont déjà gérés par `repo.busy` ; le menu avatar désactive Sign out si `repo.busy`.

## Tests

- Rust : pagination (cursor None/Some), `is_binary` (4 cas), parsing `remoteAvailable/remoteAuthorized/revisionNumber`, `lore_identity` parsing `authUserInfo`.
- TS (vitest, modules purs uniquement — pas de runes) : `initialsFor`/`displayNameFor` (6 cas), message de commit composé (extraire un helper pur `composeCommitMessage(summary, description)`).
- Vérification manuelle : passe complète dans l'app réelle (identité, offline en coupant le réseau si praticable, lock préventif, commit avec description visible dans `lore history`).

## Parité mock

Le mock reflète chaque changement d'API : `getIdentity`, `pickRepoFile`, `StatusResult` enrichi, `Commit` sans adds/mods/dels — pour que l'UI en dev navigateur reste représentative.

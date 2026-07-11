# Lore Desktop — Lot P3 « récupération » (design)

Date : 2026-07-11. Backlog de l'audit n°2, priorisé avec Jimmy (« fait tous les
plans maintenant ») : specs et plans rédigés sur Fable, exécution prévue sur Opus,
subagent-driven avec double revue par tâche, sur `main`, conventions des lots P1/P2
(captures wire OBLIGATOIRES en tête d'item pour tout champ NDJSON non encore pinné ;
repo de test C:\Users\jimmy\lore-test-repo, CLI C:\Users\jimmy\bin\lore.exe ;
fixtures dans src-tauri/tests/fixtures/ + README).

## Trois items

### 1. « Locate moved project » (`repository update-path`)

**Problème (audit P1-A5)** : si l'artiste déplace/renomme le dossier d'un repo
connu, l'app échoue sans issue — `addExistingRepo` valide via `getStatus` et
échoue, la liste des repos garde un chemin mort.

**Design.**
- Détection : au chargement de la config et au switch de repo, un chemin de
  `config.repos` dont le dossier n'existe plus sur disque (check frontend via une
  nouvelle commande Tauri `os_path_exists(path) -> bool`, triviale) est marqué
  `missing` dans le RepoSwitcher (ligne estompée, badge « Missing », icône warn).
  Si le repo COURANT est manquant au démarrage → retour à l'écran RepoPicker avec
  un toast « <nom> is missing — locate it from the repository list ».
- Action : sur une ligne `missing`, le bouton « Locate… » remplace l'entrée
  normale → `pickFolder` → nouvelle commande Tauri `lore_update_path(old_path,
  new_path)` qui exécute `lore repository update-path` (CAPTURE OBLIGATOIRE en
  tête d'item : syntaxe exacte — `lore repository update-path <new> --repository
  <new?>` ? le help CLI tranche ; si la commande échoue ou n'a pas la sémantique
  attendue, fallback = valider le nouveau dossier par `getStatus` seul) puis
  vérifie `getStatus(new_path)`. Succès → remplace le chemin dans `config.repos`
  (+ `currentRepo` si c'était lui), toast info. Échec → toastError, l'entrée
  reste marquée missing.
- La validation « le dossier choisi est bien CE repo » : comparer l'id du repo
  (`repository info` ou le champ id du status si disponible — la capture le dira ;
  à défaut, accepter et laisser getStatus faire foi, avec un avertissement si
  le nom de branche/id ne matche pas ce qu'on connaissait).

### 2. Sync-to-revision (« time travel ») depuis History

**Décision de design (Fable)** : le restore PAR FICHIER reste bloqué par
l'absence de `lore file cat <rev>` (dette CLI connue). Ce qui est directement
supporté par le CLI est le voyage du working copy entier : `lore revision sync
<revision>` (alias de `lore sync <revision>`). C'est l'item P2 « time travel »
de l'audit, et c'est LA voie honnête pour « récupérer une ancienne version » en
v1 : on synchronise le repo à la révision, l'artiste copie ce dont il a besoin,
puis revient au latest.

**Design.**
- Dans le détail d'un commit de History : bouton « Sync to this revision… »
  (sous le hash, à côté d'Undo/Edit quand ils existent), avec confirmation
  explicite : « Your working copy will match revision N. You'll be behind the
  latest — sync back when you're done. » Exigence : arbre PROPRE (même règle que
  Undo commit — bouton désactivé avec tooltip si des changements locaux existent).
- Backend : `lore_sync_to(repo_path, revision)` → runner streaming existant
  (`run_lore_op`, kind "sync", progression déjà câblée) avec `["sync", <rev>,
  "--repository", ...]`. CAPTURE OBLIGATOIRE en tête d'item : forme exacte
  (`lore sync <rev>` vs `lore revision sync <rev>`), comportement sur arbre sale,
  et ce que `status` rapporte ensuite (isRemoteAhead ? revisionLocalNumber ?) —
  fixture status_detached.ndjson.
- État visible : la StatusBar sait déjà afficher « Synced · rev N » ; quand
  `revisionLocalNumber < revisionRemoteNumber` ET pas de merge/staged, le chip
  (pattern statusChip existant, nouveau kind 'behind') devient « At rev N —
  back to latest », cliquable → `lore sync` normal (retour au tip, streaming).
  Précédence : merge > staged > behind.
- Cas limite : sync-to-revision pendant un merge/staged → boutons désactivés
  (mêmes gardes que le reste).

### 3. Garde anti-race de `refreshHistory` (follow-up revue finale P2)

`repo.svelte.ts` : `refreshHistory` assigne `history.commits` sans vérifier que
`session.config.currentRepo` est toujours le path capturé — un switch rapide
A→B peut afficher les commits de A sous B. Appliquer le pattern jeton existant
(`sizesToken` de refreshFileSizes) : `historyToken` incrémenté à chaque appel,
assignation gated sur `token === historyToken && currentRepo === path`. Même
garde pour `loadMoreHistory`. Tests vitest sur la logique pure si extractible,
sinon test du mock avec deux appels entrelacés.

## Hors périmètre

Restore par fichier (bloqué `file cat` — à re-planifier quand le CLI l'aura),
cherry-pick/revert d'un commit pushé, `repository create` (lot P6), toute
écriture pendant l'état « behind » autre que le retour au latest (commit en
détaché : comportement CLI inconnu — le bouton Commit est désactivé avec
tooltip quand le chip behind est actif, décision conservatrice v1).

## Tests

Vitest : détection missing (fonction pure sur config + existences), chip behind
(précédence à 3), garde anti-race history (jetons). Rust : fixtures update-path
et status détaché (captures), commande sync_to (args). Mock : levier « repo
manquant » (chemin spécial), sync-to-revision simulé (remoteAhead après).
Vérification réelle finale : déplacer réellement un clone de test et le
relocaliser ; sync-to-revision sur le repo de test + retour latest ; suites.

## Ordre de livraison

1. Item 3 (garde anti-race — petit, indépendant).
2. Item 1 (Locate — capture update-path d'abord).
3. Item 2 (time travel — capture sync <rev> d'abord).

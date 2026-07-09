# Audit Lore Desktop — couverture des features Lore & usage studio

**Date :** 2026-07-09
**Statut :** Audit — constats + recommandations, rien d'implémenté
**Contexte :** l'app cible les équipes d'un studio de jeu vidéo (devs, artistes, LD, audio…). Elle doit être alignée sur Lore : exploiter toutes les features utiles du CLI et afficher/utiliser toutes les infos qu'il fournit.

**Méthode :** comparaison systématique entre (a) la surface complète de `lore 0.8.3+201` (`--help` de chaque sous-commande + capture des événements JSON réels sur `lore-test-repo` / `desktoptest1`), (b) ce que `src-tauri/src/commands.rs` invoque et parse, et (c) ce que l'UI Svelte affiche.

---

## 1. Bugs concrets découverts pendant l'audit

À corriger avant toute nouvelle feature — quatre sont visibles par l'utilisateur final.

| # | Défaut | Où | Détail |
|---|--------|-----|--------|
| B1 | **Sign out ne déconnecte pas réellement** | `tauri.ts` | `tauriApi` hérite de `...mock` et **n'override pas `signOut`** : le bouton avatar efface une clé localStorage du mock, mais `lore auth logout` n'est jamais appelé. Au redémarrage on est toujours connecté. |
| B2 | **Le champ Description du commit est jeté** | `Changes.svelte:73` | Le `<textarea>` Description n'est bindé à rien ; `doCommit` n'envoie que `message`. L'utilisateur qui rédige une description la perd silencieusement. Fix : `bind:value` + concaténer `summary + "\n\n" + description` dans le message. |
| B3 | **Bouton « + Lock a file… » mort** | `Locks.svelte:16` | Aucun `onclick`. Un artiste ne peut verrouiller un fichier **que** s'il apparaît déjà dans Changes — impossible de verrouiller *avant* d'éditer, ce qui est pourtant le cœur du workflow de lock. |
| B4 | **Pagination History sans fin de liste** | `commands.rs:265` | `next_cursor` = dernier commit de la page, **jamais `null`**. En bas d'un historique réel, chaque événement scroll relance `lore history` pour une page vide (le curseur re-renvoie le commit-curseur, retiré ensuite). Fix : `next_cursor = None` quand la page renvoie moins de `length` entrées. |
| B5 | **Compteurs +/~/− toujours vides dans History** | `commands.rs:236` | `adds/mods/dels` sont codés en dur à 0 (le mock, lui, les remplit — la feature a l'air de marcher en dev et disparaît en prod). Soit les remplir (coûteux : un diff par commit), soit retirer les chips des lignes et ne garder que le détail à la sélection (déjà exact). |

Défauts secondaires : `relative_time` plafonne à « N days ago » (pas de semaines/mois, pas de date absolue en tooltip) ; le libellé StatusBar « Synced » est affiché même quand le remote est en avance ou injoignable.

---

## 2. Couverture des commandes Lore

### Utilisé aujourd'hui (correct)

`auth list` (session), `login`, `repository list`, `clone`, `status --scan`, `stage --scan` + `unstage` (commit sélectif), `commit`, `push`, `sync`, `reset` (discard), `diff` (fichier courant, plage pour push-locks et fichiers d'un commit), `history --revision` (pagination), `branch list/switch/create/merge (start/resolve/theirs|mine/abort)/diff` (préview merge), `branch reset` (undo), `lock acquire/release/query`.

### Features Lore absentes de l'app

| Feature CLI | Valeur pour un studio | Reco |
|---|---|---|
| `auth info` / événement `authUserInfo` | **Identité réelle de l'utilisateur** — l'avatar est un « JD » codé en dur (`TitleBar.svelte:8`, `History.svelte:131`). | P0 — récupérer nom/email au bootstrap, initiales dérivées, menu avatar (identité + Sign out). |
| `notification subscribe` | **Événements temps réel** (push d'un coéquipier, locks). Aujourd'hui le refresh n'a lieu qu'au focus fenêtre. Pour une équipe, c'est LA différence entre « le badge Sync dit 3 » et « je découvre au push que je suis en retard ». Testé : le flux JSON marche. | P0 — process `lore notification subscribe --json` par repo ouvert (sidecar), events Tauri → rafraîchir status/locks/history + toast discret. |
| `file history` | **Timeline par fichier** : qui a modifié cet asset, quand, dans quel commit (métadonnées complètes : message, timestamp, created-by/committed-by, taille à chaque révision). C'est la question n°1 d'un artiste. | P1 — panneau « History » dans FilePreview + entrée contextuelle dans Changes/commit detail. |
| `file info` | `size` (révision) vs `localSize` (disque) → le « 2.0 MB → 2.3 MB » du mock, aujourd'hui perdu en réel (le DTO status n'a pas `oldSize`) ; flags conflit. | P1 — remplir `oldSize` ; afficher le delta de taille (les artistes surveillent le poids des assets). |
| `revision amend` | Corriger le message du dernier commit local — bien plus simple que l'undo maison pour la faute de frappe. | P1 — action « Edit message » sur le dernier commit local. |
| `sync <revision>` / `revision info` | **Time travel** : synchroniser le working copy sur une révision passée (repro d'un bug de build, retrouver l'état d'une démo). | P1 — action « Sync to this revision » dans History (avec garde-fou tree propre). |
| `revision revert` / `cherry-pick` | Annuler un commit déjà pushé / rapatrier un fix d'une branche. L'undo actuel ne couvre que le dernier commit **local**. | P2 — « Revert » sur commit pushé ; cherry-pick au menu contextuel. |
| `branch archive` | La liste de branches ne fait que croître (le menu virtualise 2000 branches — signe que le besoin est réel). Archiver = ranger. | P1 — action « Archive » dans BranchMenu (les archivées sont déjà filtrées à l'affichage). |
| `branch protect/unprotect` + état | Éviter le push direct sur `main` par un junior. L'app ne montre même pas qu'une branche est protégée : le push échouera avec une erreur brute. | P2 — badge « protected » + message clair au push refusé. |
| `branch info` (`latest` vs `latestRemote`, `parent`, `branchPoint`, `creator`, `created`) | Ahead/behind **par branche** dans le menu, « créée par X il y a N jours ». | P2 — enrichir BranchMenu. |
| `push --fast-forward-merge` | Quand la branche a bougé côté serveur, proposer « Sync & push » au lieu d'échouer. | P2. |
| `repository info` | Nom serveur, description, branche par défaut, créateur/date — jamais affichés (le switcher n'affiche que le basename du dossier). | P2 — sous-titre du switcher + panneau About repo. |
| `file dependency` + `sync --root-file/--dependency-*` | **Sync sélectif par dépendances** : sur un repo de jeu de plusieurs centaines de Go, un sound designer ne veut que sa zone. Gros différenciateur Lore. | P2/P3 — d'abord vérifier que le studio renseigne des dépendances ; sinon non pertinent pour l'instant. |
| `link` / `layer` (+ `commit --link/--layer`) | Mounts de repos/couches (packs d'assets partagés). Invisible dans l'app ; si un repo du studio en utilise, l'app affichera un tree sans expliquer sa structure. | P2 — au minimum les **afficher** (badge sur les chemins montés, liste dans About repo). |
| `revision bisect` / `restore` / `find`, `file obliterate`, `repository verify/gc/dump`, `shared-store`, `service` | Outils pointus/ops : bisect d'asset cassé, purge d'un binaire, dédup de store. | P3 — laisser au CLI ; documenter. `shared-store` mérite un jour une case à cocher au clone (plusieurs working copies = disque économisé). |
| `repository create` | **Exclu volontairement** par la spec du repo-switcher (2026-07-08). Ne pas réintroduire sans décision. | — |

---

## 3. Infos que Lore fournit déjà et que l'app jette

Champs présents dans les événements JSON **déjà parcourus** par l'app, mais ignorés :

| Événement.champ | Ce que ça dit | Où l'afficher |
|---|---|---|
| `repositoryStatusRevision.remoteAvailable` / `remoteAuthorized` | **Mode hors-ligne / non autorisé.** La StatusBar affiche « Synced » même sans réseau. | StatusBar : « Offline — travail local » + désactiver Sync/Push proprement. P0. |
| `repositoryStatusRevision.revisionStaged` | Un état stagé existe (commit interrompu, merge en cours). L'app le découvre en échouant (« Cannot merge with staged state »). | Bandeau « staged state en attente » + action de reprise. P1. |
| `repositoryStatusRevision.revisionMerged*` | Merge en cours au niveau status — permettrait à **toutes** les vues de le savoir, pas seulement Merge.svelte quand on l'ouvre. | Badge global « merge in progress ». P1. |
| `repositoryStatusSummary` (adds/deletes/modifies/moves/copies) | Compteurs par type de changement, gratuits. | En-tête de Changes (« 7 files · +2 ~4 −1 »). P2. |
| `repositoryStatusRevision.revisionNumber` | Numéro de révision courant. | StatusBar (« rev 5 »), utile au support (« t'es à quelle rev ? »). P2. |
| `metadata timestamp` (history) | Timestamp absolu, converti en « N days ago » avec perte. | Tooltip date complète + granularité semaines/mois. P1 (trivial). |
| `metadata committed-by` (history) | Committer ≠ auteur (commits déposés par un build farm, merges). | Détail commit. P3. |
| `fileHistory.size` par révision | Évolution du poids d'un asset. | Vue file history (cf. §2). P1. |
| `branchListEntry.location` (local/remote) | Branches remote-only (pas encore récupérées) vs locales — fusionnées silencieusement dans le menu. | Section « Remote » dans BranchMenu. P2. |

---

## 4. Alignement UX studio (devs, artistes…)

Constats indépendants du CLI, à l'aune de « app quotidienne d'une équipe de jeu » :

1. **Détection binaire par extension : 7 extensions** (`commands.rs:61` — uasset, umap, png, fbx, wav, tga, psd). Manquent au minimum : `dds, exr, hdr, tif, blend, ma/mb, max, sbs/sbsar, spp, ztl, ogg, mp3, flac, bank, anim, abc, gltf/glb, bin, zip`. Un `.blend` modifié tentera un diff texte (échec ou bruit). Même problème pour la table `TYPES` de FilePreview (9 entrées). Reco P0 : liste étendue + fallback « contenu non-UTF-8 ⇒ binaire » côté Rust.
2. **Aucune vraie vignette** : FilePreview affiche des placeholders même pour un `.png` du working copy, lisible localement. Reco P1 : afficher l'image locale (after) via `convertFileSrc` ; le « before » nécessite l'accès au contenu d'une révision — **manque côté CLI** (pas de `lore file cat <rev>`), à remonter à l'équipe Lore.
3. **Pas de « Reveal in Explorer » / « Open file »** — réflexe permanent des artistes (GitHub Desktop, P4V l'ont). Reco P1 : menu contextuel sur les fichiers (Changes, Locks, détail commit) : Reveal, Open, Copy path, Lock/Unlock, Discard.
4. **Pas de recherche** : ni filtre dans Changes (une refonte de map = 300 fichiers), ni recherche History (message/auteur — `revision find` existe côté CLI), ni filtre Locks. Reco P1 Changes, P2 History.
5. **Locks passifs** : pas de lock préventif (cf. B3), pas d'avertissement à l'édition d'un fichier verrouillé par un autre (l'info `lockedBy` est déjà sur le fichier — un bandeau rouge dans FilePreview suffirait), pas de « force release » (lead, avec `--force`). Reco P0/P1 — c'est le workflow central des binaires en studio.
6. **Un seul repo à la fois, zéro parallélisme d'état** : le switcher (2026-07-08) a réglé la navigation ; les badges (remote ahead, locks) ne vivent que pour le repo ouvert. Avec `notification subscribe` multi-repo, le switcher pourrait afficher un point « activité » par repo. P3.
7. **Onboarding artiste** : la première erreur « Not a Lore repository » est le seul feedback si on ouvre le mauvais dossier ; pas d'explication de ce qu'est un working copy. Micro-copy à soigner. P3.
8. **Parité mock/réel** : le mock expose `oldSize`, des stats de commit, des auteurs multiples — le réel non (cf. B5). Toute correction de §3 doit être répercutée dans `mock.ts` pour que le dev UI reste représentatif.

---

## 5. Priorisation proposée

**P0 — corriger/aligner (petit, fort impact) :**
B1 sign-out réel (`lore auth logout`), B2 description de commit, B3 lock préventif (picker de fichier du repo ou lock depuis Changes), B4 fin de pagination, identité réelle (`authUserInfo` → avatar/initiales), indicateur offline (`remoteAvailable`), liste d'extensions binaires étendue.

**P1 — la valeur quotidienne :**
Notifications temps réel (`notification subscribe`), file history par asset, `oldSize` + delta de poids, vignettes images du working copy, Reveal/Open/menu contextuel, amend du dernier message, dates absolues en tooltip, filtre Changes, archive de branche, avertissement « verrouillé par X » à la sélection, badge merge-in-progress/staged-state.

**P2 — confort & équipe :**
Sync vers une révision, revert de commit pushé, branches protégées (badge + erreur claire), ahead/behind par branche, section Remote du BranchMenu, `repositoryStatusSummary` dans Changes, repository info/About, recherche History, push fast-forward, affichage links/layers.

**P3 — plus tard / à valider avec l'usage :**
Sync sélectif par dépendances, cherry-pick/bisect/restore, shared-store au clone, activité multi-repo dans le switcher, committed-by, obliterate/ops (rester CLI).

**Manques côté CLI à remonter à l'équipe Lore :** lecture du contenu d'un fichier à une révision donnée (pour les vignettes « before » et un vrai visualiseur d'historique d'asset) ; idéalement un `history --stats` (compteurs par commit sans un diff par ligne).

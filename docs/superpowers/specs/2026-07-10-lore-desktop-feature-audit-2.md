# Lore Desktop — Audit de features n°2 (2026-07-10)

Second audit complet, un jour après le premier (`2026-07-09-lore-desktop-feature-audit.md`).
Méthode : matrice de couverture commande par commande (CLI `lore` réel, 90 sous-commandes
énumérées via `--help`) croisée avec les invocations effectives dans `src-tauri/src` et la
surface UI Svelte, plus inventaire des specs/plans et des stubs. Chaque « absent » important
a été contre-vérifié par sondage grep — aucun faux négatif détecté.

## Verdict global

**La parité avec Lore est atteinte pour le périmètre « app artiste ».** Sur ~90 sous-commandes
du CLI, l'app en invoque ~30 ; tout le reste se classe soit en « équivalent fonctionnel déjà
couvert autrement », soit en « admin/CI/debug hors sujet ». Les 39 méthodes du contrat
`LoreApi` ont toutes une surface UI (aucune méthode morte). Les 20 specs et 14 plans sont
shippés ; tout ce qui était marqué « verified » l'est dans le git log.

Ce qui reste se répartit en : 6 manques réels à prioriser, une liste P2, des dettes internes
à l'app, et 2 blocages CLI à remonter à l'équipe Lore.

## A. Manques réels à prioriser (valeur artiste directe)

1. **`file info` → delta de poids d'asset** — `oldSize` existe dans le DTO et l'UI
   (`FilePreview.svelte:115`) mais n'est jamais rempli en réel. Afficher « 2.0 MB → 2.3 MB ».
   Déjà P1 de l'audit n°1, toujours ouvert. Petit effort, forte valeur.
2. **Bandeau « verrouillé par X » + force release** — à la sélection d'un fichier locké par un
   autre, aucun avertissement dans FilePreview ; pas de release `--force` pour un lead.
   P1 de l'audit n°1, toujours ouvert.
3. **Badge « merge in progress » / « staged state en attente »** — les champs
   `revisionMerged*`/`revisionStaged` sont déjà parcourus mais rien n'est affiché globalement.
   P1 de l'audit n°1, toujours ouvert.
4. **Restaurer une ancienne version d'un asset** (`revision restore` / sync-to-revision) —
   l'historique par fichier est affiché (FilePreview) mais sans action « restaurer cette
   version ». LE cas d'usage Anchorpoint. Couplé au blocage CLI `file cat <rev>` (voir E)
   pour la variante « récupérer sans reset ».
5. **« Locate moved project »** (`repository update-path`) — **nouveau, repéré par cet audit** :
   si l'artiste déplace/renomme le dossier du projet, `addExistingRepo` (`repoActions.ts:15`)
   échoue sans proposer de re-lier le repo. Flux GitHub Desktop classique.
6. **`repository create`** — exclu volontairement lors du switcher (07-08). À réarbitrer :
   GitHub Desktop et Anchorpoint savent créer un projet depuis l'UI ; aujourd'hui il faut le
   CLI ou un admin.

## B. Utile mais secondaire (P2)

- **Sync-to-revision** (time travel) depuis History / file history.
- **`revision revert` d'un commit déjà pushé** — l'Undo actuel ne couvre que le dernier commit
  local non poussé (mécanisme backup + `branch reset` + réapplication).
- **Lecture de `branch protect`** — griser push/merge et badge « protected » (l'écriture
  protect/unprotect reste admin).
- **`branch info`** — ahead/behind par branche, créateur/date dans le BranchMenu.
- **Recherche History** (`revision find` : message/auteur) ; **filtre Locks**.
- **`repository info`** — panneau « About repo » + sous-titre du switcher.
- **`shared-store`** — toggle « utiliser le cache partagé » au clone ou activation silencieuse
  (gros gain disque multi-working-copies).
- **`push --fast-forward-merge`** — proposer « Sync & push » quand le push est refusé.
- **Compteurs +/~/− dans l'en-tête de Changes** (`repositoryStatusSummary`) ; **« rev N » dans
  la StatusBar** ; **section « Remote » dans le BranchMenu**.
- **`service start/stop`** géré en interne (démarrage auto du service à l'ouverture du repo) —
  comportement, pas UI. **`logfile info`** → item Aide > « Ouvrir les logs » pour le support.
- **`file dependency`** — sync sélectif par dépendances : différenciateur Lore sur les gros
  repos, à valider que le studio renseigne des dépendances (P3 de l'audit n°1, inchangé).

## C. Hors sujet pour l'app (admin/CI/debug) — ne rien faire

`dirty`, `completions`, `repository delete/verify/dump/store/metadata/instance`,
`branch latest/metadata/protect(écriture)/unprotect`, `revision bisect/cherry-pick/metadata`,
`file metadata/dirty/obliterate/write/hash`, `auth clear`, `layer *`, `link *`, `service run`.

Les « partiels » de la matrice sont tous **fonctionnellement sains** : stage/unstage (design
stage-less assumé, modèle checkbox GitHub Desktop), `repository status/clone` et `branch push`
(variantes top-level équivalentes), `branch reset` et `revision info/revert` (recomposés côté
app), `auth list` (booléen suffisant en mono-serveur), `lock status` (redondant avec
`lock query`).

## D. Dettes et stubs internes à l'app (hors matrice CLI)

- **Preview « Before » = placeholder** (`FilePreview.svelte:150-167`) — bloqué CLI (voir E).
  Idem A/B audio.
- **Cartes Mine/Theirs du merge aveugles** (`Merge.svelte:203-218`) — icônes génériques au lieu
  du contenu réel des deux versions ; la décision de résolution se fait à l'aveugle. Le
  « After » (mine, copie de travail) pourrait déjà utiliser le pipeline de previews existant ;
  le « Theirs » retombe sur le blocage `file cat`.
- **`resolvedSide` purement UI** (`Merge.svelte:17-19`) — perdu si on quitte la vue en cours de
  résolution.
- **Graphe History** : lanes linéaires par page (`commands.rs:249`), labels de head best-effort
  (`commands.rs:354`) — follow-ups notés en commentaire.
- **`notifyRouting.ts`** : le filtrage « ne pas se notifier soi-même » dépend de la cohérence
  entre le `userId` des events et `identity.id` — à vérifier en conditions réelles.
- **Dette de vérification Substance** : `.sbsar`/`.spp` jamais testés sur fichiers réels
  (aucun Substance sur la machine). À valider dès qu'un artiste fournit un échantillon.
- **Encodage wire du champ `action`** à confirmer contre fixture (`commands.rs:145-158`).
- **Diff texte** : pas de side-by-side, ni coloration syntaxique, ni diff intra-ligne, ni diff
  entre révisions arbitraires (out of scope assumé du design 07-06).
- **Barre de progression de clone** (streaming `repositoryCloneProgress`) — différée depuis la
  slice B.
- **Thème clair + picker** — tokens posés depuis 07-05, jamais livrés. **Écran de
  préférences** absent. **Versionnement du schéma de config** absent.
- **CI + releases signées** — notées dès le design 07-04, toujours pas en place.
- **README obsolète** — décrit encore « Slice 1 — UI with mock data » alors que le backend réel
  est câblé depuis le 07-05.

## E. Blocages CLI à remonter à l'équipe Lore (confirmés au 2026-07-10)

1. **`lore file cat <revision>`** — n'existe toujours pas (vérifié sur le CLI installé ce jour).
   Bloque : preview « Before », A/B audio, cartes Theirs du merge, « ouvrir cette révision »
   et diff inter-révisions dans le file history. Blocage transverse n°1.
2. **`lore history --stats`** — compteurs +/~/− par commit sans payer un diff par commit.

## F. Features desktop pures (sans équivalent CLI) — l'actif du produit

Pipeline de previews complet (images dont DDS/EXR/HDR/PSD, vignettes embarquées
`.blend`/`.uasset`/`.umap`/`.sbsar`/`.spp`, cache par hash + negative cache), lecteur audio
waveform, viewer 3D turntable, vignettes de listes, notifications temps réel avec sidecar
auto-restart, toast « release locks » post-push, Undo commit composite, menus contextuels
Reveal/Open/Copy/Lock/Discard, switcher multi-repos. C'est la valeur « façon Anchorpoint »
— rien de tout ça ne se lit dans une matrice CLI.

## Recommandation d'ordre

1. Lot « artiste au quotidien » : A1 (delta de poids) + A2 (bandeau lock + force release) +
   A3 (badge merge/staged) — les trois P1 hérités, petits et indépendants.
2. A5 « Locate moved project » — petit, nouveau, vrai irritant.
3. A4 restore de version (la partie faisable sans `file cat` : `revision sync` scoped ou
   restore via reset temporaire) — et remonter E1 en parallèle.
4. Réarbitrer A6 `repository create`.
5. Puis piocher dans B selon retours d'usage ; l'extension Unreal Engine (locks + vignettes
   manquantes) reste la passe suivante de la roadmap.

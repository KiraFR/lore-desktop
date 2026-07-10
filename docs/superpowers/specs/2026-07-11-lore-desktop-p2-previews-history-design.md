# Lore Desktop — Lot P2 « previews & history » (design)

Date : 2026-07-11. Retours de Jimmy après livraison du lot P1 (+ les trois restes du
lot P1 consignés dans le header du plan précédent). Design validé en session ; la
revue de la spec et du plan par Jimmy aura lieu avant l'exécution (préparé de nuit,
exécution le lendemain).

## Six items

### 1. Preview de fichier dans la vue History

Dans le détail d'un commit, chaque ligne de fichier devient sélectionnable comme
dans Changes : clic ou Entrée → un panneau latéral s'ouvre à droite du détail
(même emplacement que FilePreview dans Changes). Re-clic sur la ligne sélectionnée
ou Escape → fermeture. Changer de commit réinitialise la sélection.

Le panneau est un **FilePreview allégé** : vignette/audio/3D via le pipeline
existant (`getPreview`), type/taille, historique du fichier (`getFileHistory`) —
**sans** Discard ni Lock (actions de copie de travail, hors contexte ici).
Mention « Preview of the current working copy » affichée dès que le commit
sélectionné n'est pas le tip local (sans `lore file cat <rev>`, on ne peut montrer
que l'état actuel du disque ; la mention devient inutile le jour où le CLI sert
une révision). Fichier supprimé du disque (ex. ligne « delete » d'un vieux
commit) → icône générique + « No longer in the working copy ».

**Découpage** : extraire de FilePreview la partie réutilisable (rendu
image/audio/3D + métadonnées) en composant partagé plutôt que dupliquer —
FilePreview (Changes) = partagé + diff + lock + discard ; HistoryFilePreview =
partagé + historique + mention working-copy.

### 2. Unification des previews dans la vue Merge

Les cartes Mine/Theirs abandonnent les icônes placeholder pour le pipeline réel :
- **Mine** = copie de travail → `requestThumb`/`listThumbs` direct (comme les
  lignes de Changes).
- **Theirs** = le fichier sidecar `<nom>~theirs` que le CLI matérialise sur le
  disque pendant un merge conflictuel (constaté lors des captures du lot P1).
  **Étape de vérification OBLIGATOIRE en tête de plan** : sur un merge réel,
  confirmer l'existence et le nommage exact du sidecar pour un binaire ET un
  texte ; si le sidecar manque pour certains types, fallback = l'icône actuelle
  côté Theirs uniquement (Mine garde sa vraie vignette).
- Fichiers **texte** : mini-diff (quelques lignes, réutiliser le rendu de diff
  existant) dans la carte plutôt qu'une vignette.

### 3. Préchargement de l'History

`refreshHistory(true)` déclenché dès la sélection/l'ouverture d'un repo (au même
endroit que le `refreshStatus` du changement de repo dans App.svelte), au lieu
d'attendre l'entrée dans la vue History. L'`$effect` de History.svelte est
conservé (rafraîchit en revenant sur la vue) mais trouve le cache chaud →
affichage instantané. Coût : un appel CLI de plus à l'ouverture, en arrière-plan,
non bloquant, silencieux en cas d'échec (pattern des refreshs existants).

### 4. Corrections de la vue Merge (restes P1, préexistants)

- **Nom de branche source erroné en reprise** : quand le merge a été démarré hors
  app, Merge.svelte défaulte la source au premier branch non courant → bannière
  fausse (« Merging main into feature/test » vu en vérification). Fix : afficher
  un libellé neutre « Resolve merge » / « Merging into <cible> » quand la source
  est inconnue — ne JAMAIS afficher un nom deviné. (Résoudre le vrai nom via
  `revisionMerged` ↔ branch tips est une amélioration possible si triviale,
  sinon hors périmètre.)
- **Vue stale après abort externe** : si `branch merge abort` arrive au CLI
  pendant que la vue Merge est en phase résolution, elle y reste. Fix : un
  `$effect` qui surveille `repo.status.mergeInProgress` — passage à `false` sans
  action locale → retour en phase setup + toast informatif (« Merge was aborted
  outside the app »).

### 5. Finitions progression (restes P1)

- **Libellé clone « X / Y »** : la spec P1 le prévoyait, seul le pourcentage est
  affiché. L'unité est connue (octets) et `unit` est câblé bout en bout mais
  inutilisé. Libellé cible : « Cloning… 42% — 12.0 MB / 48.0 MB » (réutiliser
  `fmtSize`), appliqué dans RepoPicker et RepoSwitcher. Sync/push gardent leur
  barre sans texte (décision P1, commentée dans TitleBar).
- **Garde globale anti double-clone** : RepoPicker et RepoSwitcher ont chacun un
  `busy` local ; les deux surfaces peuvent coexister et lancer deux clones qui
  s'écrasent le slot `opProgress.clone`. Fix : la garde monte au niveau du store
  (clone en cours = `opProgress.clone !== null` ou un flag dédié) — les deux
  surfaces désactivent leurs boutons Clone quand un clone est en vol, où qu'il
  ait été lancé.

### 6. Cycle de vie du sidecar notifications (reste P1, préexistant)

Constat de la vérification P1 : après un kill dur de l'app, les process
`lore notification subscribe` survivent (2 orphelins constatés + 1 zombie d'une
session antérieure). Fix côté Rust (notifications.rs) : attacher les children à
un **Job object Windows avec JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE** (crate
`win32job` ou appel direct windows-rs — choisir ce qui est déjà dans l'arbre de
dépendances, sinon le plus léger) pour que l'OS tue les sidecars à la mort du
process app, quelle que soit la façon dont il meurt. Prévoir l'équivalent no-op
sur non-Windows (cfg). Test : difficile en unit — vérification manuelle scriptée
au plan (spawn app dev, kill dur, `Get-Process lore` vide).

## Hors périmètre

Contenu des fichiers à une révision passée (`file cat` — blocage CLI inchangé),
recherche dans History, compteurs +/~/− par commit (`history --stats`, blocage
CLI), toute action sur le panneau preview de History (Discard/Lock/Restore),
« Pushing 2/5 » par révision (noté comme piste au P1, pas repris ici).

## Tests

- Vitest : logique de sélection du fichier de commit (ouvrir/fermer/reset au
  changement de commit), libellé « X / Y » (nouveau helper à côté de
  `cloneLabel`), garde globale de clone, détection « tip local vs commit
  ancien » pour la mention working-copy.
- Rust : rien de neuf à tester unitairement pour le Job object (vérif manuelle
  scriptée) ; les items 1-5 sont frontend.
- Navigateur mock : les trois vues (History preview, Merge cards avec vignettes
  mock, préchargement — l'entrée dans History doit être instantanée).
- Réel : sidecar `~theirs` (étape de vérification du plan), kill dur → zéro
  process lore.

## Ordre de livraison suggéré

1. Item 3 (préchargement — le plus petit).
2. Item 5 (finitions progression — petit, indépendant).
3. Item 4 (corrections Merge — petit).
4. Item 1 (preview History — le plus gros, extraction du composant partagé).
5. Item 2 (unification Merge — dépend de l'extraction de l'item 1 et de la
   vérification ~theirs).
6. Item 6 (Job object — indépendant, à tout moment).

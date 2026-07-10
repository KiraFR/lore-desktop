# Lore Desktop — Lot P1 « artiste au quotidien » (design)

Date : 2026-07-10. Issu de l'audit n°2 (`2026-07-10-lore-desktop-feature-audit-2.md`),
validé avec Jimmy. Quatre items indépendants ; la progression streaming est le plus gros.

## Décisions produit prises pendant le brainstorm

- **Pas de force release** dans l'app. L'idée « communication » (contacter le détenteur
  d'un lock) est notée comme piste future, probablement hors de cette app.
- Delta de poids visible **dans la liste Changes ET dans le panneau** FilePreview.
- Fichiers verrouillés par un coéquipier : **section séparée stricte** dans Changes
  (variante B stricte) — pas de checkbox, pas d'« include anyway ». Le cas du lock
  périmé reste au CLI.
- Chip merge/staged dans la **StatusBar**, clic vers l'action pour le merge ; chip
  staged **informatif seulement** en v1.
- Progression streaming pour **clone + sync + push**, avec remplacement du timeout
  fixe de 45 s par une détection de blocage pour ces trois commandes.

---

## Item 1 — Delta de poids d'asset

**Problème.** `ChangedFile.oldSize` existe dans le contrat et l'UI (« old → new » dans
FilePreview, `FilePreview.svelte:115`) mais n'est jamais rempli par le backend réel.

**Backend.** Nouvelle commande Tauri `lore_file_sizes(repo_path, paths: Vec<String>)
-> HashMap<String, u64>` : un **seul appel batch** `lore file info <chemins absolus…>
--repository <repo>` (la commande accepte plusieurs chemins), qui extrait la taille au
dépôt (révision courante) de chaque fichier — c'est l'« ancien » poids ; le « nouveau »
est le `size` local déjà fourni par `status`. Chemins passés en absolu (gotcha cwd
habituel). Le nom exact de l'événement NDJSON (`fileInfo` ?) et le champ de taille
seront confirmés contre une fixture réelle en début d'implémentation.

**Frontend.** `LoreApi.fileSizes(repoPath, paths): Promise<Record<string, number>>`.
`repo.svelte.ts` l'appelle en fire-and-forget après `refreshStatus()`, uniquement pour
les fichiers `modify` et `delete`, et fusionne `oldSize` dans `repo.status.files` —
le pattern exact du join des locks (`repo.svelte.ts:73`).

**UI.**
- Liste Changes : delta signé compact en fin de ligne (« +0.3 MB », « −0.1 MB »),
  couleur secondaire neutre (grossir n'est pas une faute). Rien si delta nul ou
  inconnu. Les `add` affichent la taille simple comme aujourd'hui ; les `delete`
  affichent l'ancienne taille seule (« 2.0 MB », sans flèche ni signe).
- FilePreview : la ligne Size affiche « 2.0 MB → 2.3 MB » (déjà câblée par `oldSize`).

**Erreurs.** Échec ou timeout de `file info` → silence total, les deltas n'apparaissent
pas. Jamais de toast pour de l'enrichissement.

---

## Item 2 — Fichiers verrouillés par un coéquipier

**Problème.** Un fichier locké par un autre est committable comme les autres, et rien
d'alarmant n'apparaît à la sélection. Les données sont déjà là : `lockedBy` est joint
côté frontend depuis `getLocks()`.

**Changes.svelte — section stricte.** La liste est partitionnée :
1. Les changements committables (tout fichier non locké ou locké par `you`).
2. En dessous, la section **« Locked by teammates (n) — excluded from commit »** :
   en-tête ambre avec icône cadenas, lignes **sans checkbox**, légèrement estompées,
   avec le nom du détenteur. Ces fichiers sont **toujours** dans `exclude` au commit —
   exclusion par construction, aucun moyen de les committer depuis l'app.

Ces lignes restent sélectionnables (preview) et gardent le menu contextuel (Reveal,
Open, Copy path/full path, Discard — Discard reste légitime : c'est la copie locale).
Le filtre texte s'applique aux deux groupes. Le compteur du bouton Commit et le badge
du NavRail comptent les committables seulement ; l'en-tête de section porte le compte
des verrouillés. Les notifications temps réel rafraîchissent déjà les locks : la
section apparaît/disparaît en live quand un coéquipier acquiert/relâche.

**FilePreview — bandeau.** Quand `lockedBy` est renseigné et ≠ `you` : bandeau
d'avertissement en tête du panneau (fond `--bg-warning`, icône cadenas) :
« Locked by <nom> — excluded from commit while locked ». La ligne Lock discrète
existante reste en bas (elle couvre le cas `you` et le bouton Lock/Unlock).

**Cas limite.** Fichier locké par un autre ET en conflit de merge : la vue Merge n'est
pas modifiée par ce lot (la résolution reste possible) ; seule la vue Changes exclut.

---

## Item 3 — Chip StatusBar « merge in progress / staged state »

**Problème.** Un merge en cours n'est visible que si on ouvre la vue Merge ; un état
stagé résiduel (commit/merge interrompu) ne se découvre qu'en échouant
(« Cannot merge with staged state »).

**Backend.** `status_from` (`commands.rs:160`) lit déjà l'événement
`repositoryStatusRevision` ; il en extrait désormais deux booléens ajoutés au DTO :
`mergeInProgress` (champs `revisionMerged*`) et `stagedPending` (champ
`revisionStaged`). Noms wire exacts à confirmer contre une capture réelle de
`lore status --json` pendant un merge — étape prévue au plan. Champs absents
(CLI plus ancien, pas de merge) → `false`.

**Frontend.** `StatusResult` gagne `mergeInProgress: boolean` et
`stagedPending: boolean` (mock mis à jour). Dans la StatusBar, un chip avec
précédence :
1. `mergeInProgress` → chip actionnable « Merge in progress — resume » ; clic =
   `ui.view = 'merge'` (la vue Merge sait déjà reprendre une résolution en cours).
2. sinon `stagedPending` → chip informatif « Staged state pending », tooltip :
   un commit ou merge interrompu a laissé un état stagé ; il sera repris au prochain
   commit/merge. **Aucune action en v1** — la sémantique d'un abandon (`unstage`)
   sera validée contre le CLI réel avant d'exposer un bouton destructif.

Le chip staged est masqué pendant un merge (le merge implique un état stagé).

---

## Item 4 — Progression clone / sync / push (+ fix du timeout 45 s)

**Problème.** `run_lore` a un plafond dur de 45 s (`lore.rs:63`) qui s'applique aussi
au clone, au sync et au push : toute opération légitime plus longue **échoue**
aujourd'hui — inévitable avec des binaires de studio. Et aucune progression n'est
visible. Le design slice B avait noté des événements `repositoryCloneProgress` dans
le flux NDJSON.

**Backend.** Nouveau `run_lore_streaming(args, on_event) -> Result<Vec<LoreEvent>>`
dans `lore.rs`, sur le modèle du sidecar de notifications (`notifications.rs`) :
process enfant stdout pipé (`CREATE_NO_WINDOW` sur Windows), lecture ligne à ligne,
parse NDJSON incrémental. Chaque événement est (a) transmis à `on_event` au fil de
l'eau et (b) collecté pour le résultat final, validé par le même `check_ok`.
Le timeout devient une **détection de blocage** : aucune ligne reçue pendant 60 s →
kill du child + erreur. Une opération qui avance n'est jamais tuée.

`lore_clone`, `lore_sync` et `lore_push` basculent sur ce runner. Les événements de
progression sont relayés au webview via un événement Tauri `lore://op-progress`
portant `{ opId, kind: "clone"|"sync"|"push", done, total?, unit? }`. L'`opId` est
généré par le **frontend** et passé en argument de la commande Tauri ; le listener
`lore://op-progress` filtre dessus — ce qui distingue des opérations simultanées
(ex. sync pendant qu'un clone tourne) sans aller-retour supplémentaire.

**Étape obligatoire en début d'implémentation** : capturer le NDJSON réel d'un clone,
d'un sync et d'un push pour confirmer la forme de `repositoryCloneProgress` (champs
done/total, octets ou fichiers) et découvrir les équivalents sync/push. S'il n'existe
pas d'événement de progression pour une opération → progression indéterminée, prévue
par le design.

**Frontend.** `cloneRepo`, `sync` et `push` gagnent un paramètre optionnel
`onProgress?: (p: OpProgress) => void` dans le contrat `LoreApi`
(`OpProgress = { done: number, total?: number, unit?: 'bytes'|'files' }`).
`tauri.ts` écoute `lore://op-progress` filtré par `opId`.
- **Clone** : barre de progression dans le flux Clone (RepoPicker et RepoSwitcher) —
  pourcentage + « X / Y » quand un total existe, barre indéterminée sinon.
- **Sync / Push** : fine barre de progression intégrée aux boutons de la TitleBar
  pendant l'opération (déterminée ou indéterminée selon les événements reçus).
- Pas de bouton d'annulation en v1.
- Le mock simule des ticks de progression (browser dev).

**Erreurs.** Blocage détecté → même surface d'erreur qu'aujourd'hui (toast). Les
autres commandes (status, diff, lock…) gardent `run_lore` et son plafond de 45 s,
adapté aux opérations courtes.

---

## Tests

- **Rust** : fixtures NDJSON pour (a) le parse de `file info` batch, (b) les champs
  `revisionMerged*`/`revisionStaged` présents/absents dans `repositoryStatusRevision`,
  (c) le runner streaming — événements incrémentaux relayés dans l'ordre, complete
  requis, détection de blocage (fake child qui se tait), kill effectif.
- **Vitest** : partition committables/verrouillés (dont le compteur du bouton Commit
  et l'application du filtre aux deux groupes), formatage du delta signé, précédence
  merge > staged du chip, fusion `oldSize` (fichiers disparus entre status et file
  info ignorés), progression du mock.
- **Vérification réelle** (skill verify) : delta visible sur un asset modifié réel,
  section verrouillée avec un second utilisateur ou un lock posé au CLI, chip pendant
  un merge conflictuel réel, clone d'un repo réel avec barre qui avance et absence de
  timeout au-delà de 45 s.

## Hors périmètre (réaffirmé)

Force release (écarté — piste « communication » future, sans doute hors app),
« include anyway » sur les fichiers verrouillés, action sur le chip staged,
annulation d'un clone/sync/push en cours, progression sur les autres commandes,
delta de poids dans History/commit detail.

## Ordre de livraison suggéré

1. Item 1 (delta) — le plus autonome, valide le pattern `file info`.
2. Item 2 (section verrouillés + bandeau) — pur frontend.
3. Item 3 (chip) — petit, backend + frontend.
4. Item 4 (streaming) — le plus gros, touche l'infra `lore.rs`.

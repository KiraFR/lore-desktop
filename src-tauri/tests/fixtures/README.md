# Captured `lore 0.8.3 --json` fixtures

Real output from a 2-commit test repo (`desktoptest1`) on `lore.example.com`,
used as the test oracle for the Rust parsers. Regenerate with the commands in the
Slice A plan, Task 2.

## Pinned encodings

**Stream:** one `{"tagName","data"}` object per line, ending with
`{"tagName":"complete","data":{"status":N}}`. `history --json` emits **two**
`complete` events: the listing, then a trailing `authUserInfo` block.

**`repositoryStatusRevision`:** `branchName` (display name, e.g. "main"),
`revisionLocalNumber` / `revisionRemoteNumber` (u64), `isLocalAhead` /
`isRemoteAhead` (JSON `true`/`false`). Ahead counts = `localNumber - remoteNumber`
(and vice-versa), gated by the boolean.

**`repositoryStatusFile`:** `action` is a **string** — `"keep"` (content modify),
`"add"`, `"delete"`, `"move"`, `"copy"`. Map `keep → modify`. Fields: `path`,
`size` (u64), `type` (`"file"`/`"directory"`/`"link"`), `flagDirty` / `flagStaged`
/ … (JSON booleans), `fromPath`. **No binary field** → infer `isBinary` from the
extension. A clean repo emits zero file events (only the revision + `complete`).

**`revisionHistoryEntry`:** `revision` (hash string), `revisionNumber` (u64),
`parent` = `[hash, hash]` (parent[0] = direct parent, parent[1] = merge parent or
all-zeros). A zero hash is 64 `'0'` chars; filter those out.

**`metadata`** (interleaved after each `revisionHistoryEntry`, until the next
entry): `{"key":"…","value":{"tagName":"string|numeric|context","data":<v>}}` —
extract `value.data`. Relevant keys:
- `message` → commit message (string).
- `created-by` → author **user id** (uuid string). Resolve to a display name via
  the trailing `authUserInfo` events.
- `committed-by` → committer user id.
- `timestamp` → commit time, epoch **milliseconds** (numeric).
- `branch` → branch context id (NOT the display name).

**`authUserInfo`** (after the listing's `complete`): `{"id":"<uuid>","name":"<display>"}`
→ the id→name map used to resolve `created-by` / `committed-by`.

**`fileInfo`** (`file info <paths…> --json`, batch — one event per file, even
when the paths passed on the CLI are absolute): `path` (relative to the repo
root, **not** echoed absolute), `size` (u64, size at the repo's current
revision — this is the "old" side of the weight delta; the "new" side is the
local `size` of `repositoryStatusFile`). Also present but not currently
consumed: `context` (uuid), `hash` / `localHash` (content hash strings —
`localHash` is all-zeros when the file has local modifications, i.e. not
computed), `isFile` / `isDir`, `flagModified` / `flagDeleted` / `flagAdded` /
`flagConflict` (booleans), `mode` (number), `localSize` (u64, local disk
size), `filterSize` (number). No per-file error event was observed; batch
still ends with the usual `{"tagName":"complete","data":{"status":0}}`.

**Merge/staged dans `repositoryStatusRevision`** : `revisionMerged` (hash — non-zéro
= merge en cours) et `revisionStaged` (hash — non-zéro = état stagé résiduel).
Un hash all-zeros (64 × '0') ou un champ absent (CLI plus ancien) = false.
Pendant un merge, `revisionStaged` est **également** non-zéro (le merge implique
un stage) : vérifier `revisionMerged` en premier — `revisionStaged != 0` seul ne
distingue pas « stagé simple » de « merge en cours » (précédence du chip).
Captures : status_merge.ndjson (pendant `branch merge start` conflictuel),
status_staged.ndjson (après `stage .` sans commit).

**Progression clone/sync/push** (fixture clone_progress.ndjson pour le clone ;
sync/push observés en live, pas committés en fixture séparée — voir ci-dessous) :
les hypothèses slice B (`done`/`current` + `total?` en top-level) étaient
**fausses**. Trois tags distincts, trois encodages distincts, tous confirmés en
**octets** :

- `repositoryCloneProgress` : imbriqué sous `data.count` — `bytesTransferred` /
  `bytesTotal` (+ `fileComplete`/`fileCount` en alternative fichiers). Vérifié :
  `bytesTotal` (202) == somme exacte des tailles des 6 fichiers trackés du repo
  de test. Avant la fin de la découverte, `bytesTotal` vaut `0` (pas "déjà
  fini") — traité comme total inconnu.
- `branchPushFragmentProgress` (push) : top-level — `bytesTransferred` /
  `bytesTotal` (+ `complete`/`count` en alternative fragments). Un push
  multi-révisions émet **une rafale Begin/Progress…/End par révision poussée** —
  `done`/`total` repart légitimement à 0 plusieurs fois pendant un seul push,
  ce n'est pas un bug. `bytesTotal` grandit lui-même en cours de rafale (la
  taille totale du lot est elle aussi découverte progressivement, comme pour le
  clone) avant de se stabiliser.
- `revisionSyncProgress` (sync) : top-level, noms différents — `bytesUpdate` /
  `bytesUpdateTotal` (+ `fileUpdate`/`fileUpdateTotal`). Un sync déjà à jour
  (`isLatest:1`, aucun delta) n'émet **aucun** événement de progression —
  barre indéterminée, comportement prévu.

Les trois tags se terminent bien par `Progress`, mais leurs champs ne
partagent aucun nom commun sauf le **préfixe `bytes…`** — `op_progress_from`
(commands.rs) utilise une allow-list explicite des trois tags + une recherche
de champ à plusieurs candidats (`count.bytesTransferred`/`count.bytesTotal`,
`bytesTransferred`/`bytesTotal`, `bytesUpdate`/`bytesUpdateTotal`).

**Sidecar « theirs » d'un merge conflictuel** (vérifié le 2026-07-11 sur un merge réel
`lore-test-repo`, `feature/test` ← `p2-theirs-src`, binaire PNG + texte README.md) :
pendant `branch merge start`, le CLI matérialise **trois** sidecars par fichier texte
en conflit à côté de l'original — `<nom>~base` (version de base commune), `<nom>~mine`
(version locale résolue, sans marqueurs) et `<nom>~theirs` (version entrante résolue,
sans marqueurs), ex. `README.md~base`, `README.md~mine`, `README.md~theirs`. Pour un
**binaire** en conflit (`p2-theirs-test.png`), seuls `~base` et `~theirs` apparaissent —
**pas de `~mine`** (le fichier de travail original tient déjà lieu de "mine"). Les trois
sidecars sont exclus du scan (`status --scan --json` les rapporte en `filterExclude`,
`reason:0` — ils ne portent jamais leur propre `flagConflict`) ; seul le fichier de base
(`README.md`, `p2-theirs-test.png`) porte `flagConflict:true` / `flagConflictUnresolved:true`
dans `repositoryStatusFile`.

Le fichier de travail du texte en conflit contient directement des marqueurs de
conflit style git : `<<<<<<< ours` / `||||||| original` / `=======` / `>>>>>>> theirs`.
`lore diff <abs README.md> --json` pendant le merge renvoie un `fileDiff` dont le
`patch` est un diff unifié entre la révision de base committée (`README.md@20`) et
ce fichier de travail marqué — le patch contient donc littéralement les lignes
`<<<<<<< ours` / `||||||| original` / `=======` / `>>>>>>> theirs` en plus des lignes
ajoutées de chaque côté (pas un diff mine-vs-base propre). Pour le **binaire** en
conflit, `lore diff <abs p2-theirs-test.png> --json` ne renvoie **aucun** événement
`fileDiff` (juste `complete`, status 0) — pas de patch pour un binaire, cohérent avec
le fallback icône-seule déjà prévu pour la Task 17.

`branch merge abort` supprime les trois sidecars et restaure l'état pré-merge ;
`branch archive p2-theirs-src` retire la branche de test de `branch list` ; après
abort + archive, `status --scan --json` ne rapporte plus aucun `flagConflict` ni
aucune trace `~base`/`~mine`/`~theirs`.

## `repository update-path` (déplacement de clone, P3 Locate)

Vérifié le 2026-07-11 en déplaçant un vrai clone (`desktoptest1` / repo id
`019f333af5e073d28bb117ad1596784a`) de `…\p3-locate2\before` vers
`…\p3-locate2\after` (`Move-Item`), scratch nettoyé après coup.

**Syntaxe réelle :** `lore repository update-path --repository <path> --json`
— un seul flag `--repository`, **pas** d'argument positionnel pour
« ancien »/« nouveau » chemin. `<path>` doit être le chemin **actuel** (déjà
déplacé) du dossier de travail local ; le CLI lit les métadonnées `.lore`
locales à cet endroit pour retrouver l'`instanceId`, puis pousse le nouveau
chemin au registre distant pour cet id. Il n'y a pas de moyen de fournir
explicitement l'ancien chemin — inutile, il est déjà connu côté serveur.

**CONSTAT : (a)** — `status --repository <after>` fonctionne **déjà**
(exit 0, `repositoryStatusRevision` complet et identique) juste après le
`Move-Item`, **avant** tout appel à `update-path`. Les métadonnées locales du
clone se suffisent à elles-mêmes pour `status` : le CLI ne consulte pas le
registre d'instances pour valider le chemin donné en `--repository`, il lit
directement les données locales à ce chemin et compare au serveur par id de
révision/repo.

Ce que `update-path` corrige réellement, c'est un registre **serveur**
séparé — visible via `lore repository instance list --repository <path>
--json` — qui associe chaque `instanceId` (uuid interne, un par clone) à son
dernier chemin connu, avec un flag `stale`. Séquence observée :
1. Juste après le clone (chemin encore correct) : `instance list` renvoie
   l'entrée avec `"path":".../before"`, `"stale":0`.
2. Après `Move-Item` (chemin réel = `after`, registre encore `before`),
   `status --repository <after>` réussit quand même ; mais interroger le
   registre à ce moment (`instance list --repository <after>`) fait
   apparaître l'entrée avec `"path":".../before"` **et** `"stale":1` — le
   flag passe à stale automatiquement dès qu'une commande tourne depuis un
   chemin qui ne correspond plus à l'entrée enregistrée.
3. Après `repository update-path --repository <after> --json` (exit 0),
   `instance list` renvoie la même entrée avec `"path":".../after"`,
   `"stale":0` — corrigée.

**Impact flux Locate (Tasks 3-5) :** valider le nouveau chemin par
`getStatus` seul suffit fonctionnellement (le repo est utilisable
immédiatement, avant même d'appeler `update-path`) ; appeler `update-path`
ensuite est un **best-effort d'hygiène** pour que ce clone n'apparaisse plus
« stale » dans le registre d'instances (utile pour du diagnostic/collab
multi-machine, pas bloquant pour l'usage local). `--dry-run` sur
`update-path` ne produit aucune sortie différenciée (même unique événement
`complete`) — ne pas s'appuyer dessus pour prévisualiser un changement.

**Événements NDJSON observés (`update_path.ndjson`)** : `update-path`
n'émet **aucun** événement porteur de données, seulement la ligne de fin
`{"tagName":"complete","data":{"status":0}}` — pas de confirmation du
nouveau chemin dans le flux, il faut re-interroger `status` ou `instance
list` séparément pour vérifier l'effet.

## `sync <revision>` / status « behind » (P3 Recovery, Item C)

Vérifié le 2026-07-11 sur `lore-test-repo` (id `019f333af5e073d28bb117ad1596784a`,
branche `feature/test`) : sync arrière vers la révision N-1 (19, `cccaca367…`)
depuis la tête locale (20, `8ff4711d…`), capture du status pendant le time
travel, puis test arbre sale, puis retour propre à la tête.

**Syntaxe réelle :** `lore sync [revision] --repository <path> --json` — un
seul verbe `sync` (**pas** de variante namespaced `revision sync`), un unique
argument **positionnel optionnel** qui est un **hash de révision** (complet ou
partiel — pas un numéro de révision, `--help` ne mentionne que la signature de
hash). `lore sync --repository <path> --json` **sans** argument = retour à la
tête (dernière révision locale connue, `isLatest:1`). Un event
`revisionSyncTarget` ouvre chaque sync : `sourceRevision(Number)` /
`targetRevision(Number)` / `isLatest` (0/1) / `local` (1) — puis
`revisionSyncProgress`/`revisionSyncFile` (fichiers touchés par le delta
seulement, `action:"keep"` ici car contenu identique déjà présent
localement) et `revisionSyncRevision` en fin de sync (branch/revision/
revisionNumber/flagMerge/flagConflict).

**CONSTAT — le champ « behind » n'est PAS `isRemoteAhead`/`isLocalAhead`.**
Ces deux booléens dans `repositoryStatusRevision` comparent la **tête locale**
(`revisionLocal(Number)`) à la **tête remote connue** (`revisionRemote(Number)`)
— ils ne bougent **pas** quand on time-travel en interne au working copy et
restent identiques avant/pendant/après le sync arrière (`isLocalAhead:1`,
`isRemoteAhead:0` inchangés tout du long dans ce repo, qui a 2 révisions
locales non poussées au remote — état préexistant du repo de test, sans
rapport avec le sync). Le signal réel de « je suis synchronisé sur une
révision passée » est l'écart entre `revision`/`revisionNumber` (position
courante du working copy, ex. 19) et `revisionLocal`/`revisionLocalNumber`
(tête locale connue, ex. 20) : `revisionNumber < revisionLocalNumber` ⇒
le repo est « behind » sa propre tête locale. Voir `status_behind.ndjson`
(capturé juste après `sync cccaca367…`) : `revision":"cccaca367…"`,
`revisionNumber":19`, `revisionLocal":"8ff4711d…"`,
`revisionLocalNumber":20` — `revisionRemote(Number)` (`fe7897ea…`/18) et
`isLocalAhead`/`isRemoteAhead` sont **inchangés** par rapport au status pris
avant le sync. **Implication Task 8 :** le chip « behind » doit comparer
`revisionNumber` à `revisionLocalNumber` (ou tester `revision !=
revisionLocal`), pas se fier à `isRemoteAhead` qui répond à une question
différente (retard vis-à-vis du serveur, pas vis-à-vis de sa propre tête
locale après un sync arrière).

**Scan pendant le time travel :** `status --scan --json` en `status_behind.ndjson`
ne liste **aucun** `repositoryStatusFile` et `repositoryStatusSummary` reste à
zéro (`adds/deletes/modifies/moves/copies: 0`) — être positionné sur une
révision passée n'est **pas** en soi un changement committable, le working
copy correspond exactement au contenu de la révision 19. Confirme que le chip
« behind » doit se déclencher sur l'écart de révision (ci-dessus), pas sur la
présence de fichiers modifiés dans le scan.

**Comportement sur arbre sale : le sync n'est PAS bloqué.** Modifier un
fichier suivi (`notify-test.txt`, `flagDirty:true`, `modifies:1` au scan) puis
lancer `sync <rev antérieure>` **réussit silencieusement** (exit 0, pas
d'erreur, pas besoin de `--force`) : le fichier sale n'apparaît même pas dans
la liste `revisionSyncFile` du delta (il n'est pas concerné par le diff entre
les deux révisions) et **conserve son contenu local modifié** après le sync.
Effet de bord observé dans `repositoryStatusRevision` : `revisionStaged`
passe de tout-zéro à un hash réel — la modification locale est reportée comme
un état stagé résiduel sur la nouvelle révision cible (cf. section
« Merge/staged » ci-dessus sur `revisionStaged`). `lore reset <path
absolu> --repository <path> --json` restaure le contenu tracké et remet
`revisionStaged` à zéro. **Implication garde UI :** il n'y a pas de refus
CLI natif à afficher ; si l'UI veut empêcher un sync arrière sur arbre sale,
la garde doit être côté application (vérifier `repositoryStatusSummary` avant
d'autoriser l'action), pas déduite d'un code d'erreur du CLI.

**Retour à la tête + nettoyage :** `sync --repository <path> --json` sans
argument ramène `revisionNumber` à la tête locale (`isLatest:1`) ;
`repositoryStatusRevision` redevient alors identique à l'état de départ
(`revision == revisionLocal`, `revisionStaged` tout-zéro,
`repositoryStatusSummary` à zéro). Repo de test revérifié propre et à jour
après ce test (voir rapport de tâche).


**`repositoryStatusSummary`** (déjà présent dans status.ndjson, constaté au lot P4) :
un seul événement par status, après les `repositoryStatusFile` — cinq compteurs u64
`adds` / `deletes` / `modifies` / `moves` / `copies`. Le DTO replie
`modifies + moves + copies` dans `mods` (l'UI colore R/C comme des « modified »).
Événement absent (CLI plus ancien) ⇒ `summary: None` ⇒ compteurs masqués côté UI.

## `branch list --json` (P4 Item 2 — section Remote, + constat `protected` pour Task 9)

Capturé le 2026-07-11 sur `lore-test-repo` (branches `main` et `feature/test`,
cette dernière courante avec un commit local non encore poussé). Fixture :
`branch_list.ndjson`.

**CONSTAT — `location` : chaîne, valeurs `"local"` / `"remote"`** (hypothèse du
plan confirmée). Deux blocs `branchListBegin`/`branchListEntry`(×N)/`branchListEnd`
par appel, groupés par location : `location:"local"` d'abord (2 entrées : `main`,
`feature/test`), PUIS `location:"remote"` (2 entrées, mêmes 2 branches — le repo
de test a bien les deux poussées côté serveur). **Le bloc remote est présent**
dans cette capture (pas de cas « absent » à noter ici ; connectivité serveur OK).

**CONSTAT — flag `protected` : ABSENT.** `Select-String -Pattern protect`
(insensible à la casse) sur toute la fixture ne remonte **aucune** occurrence —
ni `protected`, ni `protect`, ni variante, dans `branchListEntry` ni ailleurs
dans le flux. **Task 9 (badge protected) ne peut pas s'appuyer sur un champ natif
du CLI** : soit la notion n'existe pas côté `lore`, soit elle passe par une
commande séparée non couverte par cette capture. Tout badge « protected » dans
l'UI devra venir d'une convention côté app (ex. nom de branche `main`/`master`
en dur), pas d'un champ `branch list`.

**Champs réels de `branchListEntry`** (plus riches que l'hypothèse du plan, qui
ne listait que `name`/`latest`/`isCurrent`/`archived`/`location`) :
- `location` (string, `"local"` / `"remote"`)
- `id` (uuid string — identifiant **stable** de la branche, pas le nom ; c'est
  ce qui permet de faire correspondre une même branche entre les deux blocs)
- `name` (string, ex. `"main"`, `"feature/test"`)
- `category` (string — vide `""` sur les deux branches de ce repo ; format réel
  non déterminé par cette capture)
- `latest` (hash string, tip de la branche à cette location — **diffère entre
  local et remote** pour une branche pas entièrement synchronisée, voir plus bas)
- `stack` (array de `{"branch":<id>,"revision":<hash>}` — **absent de
  l'hypothèse du plan** ; vide `[]` sur `main`, une entrée sur `feature/test`,
  vraisemblablement la chaîne de branches empilées/parentes)
- `creator` (uuid string — id utilisateur créateur ; à résoudre via
  `authUserInfo` comme pour `created-by`/`committed-by` ailleurs)
- `created` (timestamp — **CONSTAT : unité différente selon `location`**, voir
  ci-dessous)
- `isCurrent` (bool — `true` uniquement sur l'entrée **locale** de la branche
  active ; toujours `false` côté remote, la notion de « current » n'existe pas
  côté serveur)
- `archived` (bool)

`branchListEnd` porte aussi `count` (u64, nb d'entrées du bloc) en plus de
`location`.

**CONSTAT — `created` : résolution différente entre `location:"local"`
(millisecondes) et `location:"remote"` (secondes).** Sur `main` (déjà poussée,
`latest` identique des deux côtés) : `created` local = `1783270929978`
(13 chiffres, epoch ms) vs `created` remote = `1783270930` (10 chiffres, epoch
s) — `remote ≈ round(local / 1000)`, cohérent avec la **même** révision vue à
deux résolutions différentes. Sur `feature/test` (commit local non poussé,
`latest` différent entre les deux blocs), les deux `created` n'ont **aucun**
rapport arithmétique simple (`1783373340062` local vs `1783455139` remote) —
normal, ce ne sont pas le même évènement (le remote reflète le dernier push,
le local le dernier commit local). **Piège pour Tasks 6/9/11** : ne jamais
comparer/trier/afficher `created` entre les deux blocs sans normaliser l'unité
(diviser par 1000 côté remote, ou multiplier par 1000 côté local) — sinon
dates fantaisistes (1970 ou horizon lointain selon le sens de l'erreur).

**Dédup local/remote :** confirmé qu'une branche présente des deux côtés
(`main`, `feature/test`) apparaît deux fois avec le **même `id`** mais des
`latest`/`created` potentiellement différents — c'est `id` (pas `name`) qui
doit servir de clé de dédup/jointure entre les deux blocs pour la Task 6 (le
plan supposait une dédup par nom).

## `branch info <name> --json` (P4 Item 4/Task 9 — double constat protected + ahead/behind)

Capturé le 2026-07-11 sur `lore-test-repo`, branche `feature/test` (courante,
un commit local non poussé — même repo que `branch_list.ndjson`). Fixture :
`branch_info.ndjson`. La commande **existe** (pas de repli « unknown command »
nécessaire) sous le nom `branch info <name>`, tag `branchInfo` — un seul
événement de données par appel (pas de begin/end/entry comme `branch list`),
suivi de `complete`.

**Champs réels de `branchInfo`** : `id` (uuid, même id que dans
`branch_list.ndjson`), `name`, `category` (vide ici, comme dans `branch
list`), `latest` (hash tip local), `latestRemote` (hash tip remote — **pas**
un compteur, juste le hash ; absent de l'hypothèse du plan), `parent` (id de
branche parente), `branchPoint` (hash de révision au point de fork),
`creator` (uuid, résolvable via `authUserInfo` comme `branch list`),
`created` (epoch **millisecondes** — même résolution que `location:"local"`
dans `branch list`, cohérent), `stack` (même structure `{"branch":<id>,
"revision":<hash>}` que dans `branch list`), `archived` (bool).

**CONSTAT (A) protected — RECONFIRMÉ ABSENT.** `Select-String -Pattern
protect` sur `branch_info.ndjson` : zéro occurrence. Combiné au constat déjà
négatif de `branch_list.ndjson` (Task 5), **aucune** commande `lore` n'expose
de flag protected en lecture. Item 4 (badge protected) : **ANNULÉ** — Tasks
10-11 non exécutées (cocher avec la mention « annulé — constat négatif
Task 9 »), un badge protected dans l'UI ne peut venir que d'une convention
côté app (ex. nom `main`/`master` en dur), pas d'un champ CLI.

**CONSTAT (B) ahead/behind — ABSENT.** `Select-String -Pattern
"ahead|behind|revisionLocal|revisionRemote"` sur `branch_info.ndjson` : zéro
occurrence. Aucun compteur numérique d'avance/retard par branche (pas de
`ahead`/`behind`, pas de `revisionLocalNumber`/`revisionRemoteNumber`, pas de
flags `isLocalAhead`/`isRemoteAhead` équivalents). Les seuls champs relatifs
au remote sont `latest`/`latestRemote`, deux **hashes** de révision (pas des
nombres) — on peut au mieux en déduire une égalité/inégalité booléenne
(`latest == latestRemote` ⇒ branche synchronisée), jamais une magnitude
d'avance/retard, et ce uniquement pour la branche interrogée (pas de version
batch de `branch info` sur toutes les branches). Item 3 (ahead/behind lazy
par branche dans le menu) : **VARIANTE B** — pas de fetch lazy réel par
branche ; repli sur la branche **courante uniquement**, dont l'ahead/behind
exact (`revisionLocalNumber`/`revisionRemoteNumber` + `isLocalAhead`/
`isRemoteAhead`) est déjà disponible sans appel supplémentaire via
`repositoryStatusRevision` (`status --json`, cf. section plus haut) — Tasks
12 et 14 annulées, Task 13 réduite à `formatAheadBehind`, Task 15 affiche
l'ahead/behind de la seule branche courante dans l'en-tête du menu.

## `repository info --json` (P4 Item 5 — panneau « About repository »)

Capturé le 2026-07-11 sur `lore-test-repo` (`lore repository info --repository
<path> --json`, exit 0 dès le premier essai — pas de repli `--help` nécessaire).
Fixture : `repo_info.ndjson`.

**CONSTAT — tag `repositoryData`, PAS `repositoryInfo`** (hypothèse du plan
infirmée). Un seul événement de données par appel, suivi de `complete` —
même forme que `branchInfo` (pas de begin/end/entry).

**Champs réels** (aucun `size` ni compte de révisions — hypothèses du plan
infirmées, ces deux notions sont absentes de la réponse) :
- `remoteUrl` (string) — URL du serveur, ex. `"lore://lore.example.com:41337"`.
- `id` (string, hex 32) — id du repo, **identique** au `id` déjà vu dans
  `repositoryStatusRevision`/`repositoryStatusFile`/autres fixtures pour ce
  même repo de test (`019f333af5e073d28bb117ad1596784a`).
- `name` (string) — nom du repo, ex. `"desktoptest1"`.
- `description` (string) — vide (`""`) sur ce repo de test ; format non
  déterminé au-delà de « chaîne, potentiellement vide ».
- `defaultBranch` (string, hex 32) — **pas un hash de révision : c'est le
  `id` de branche**, vérifié identique au `id` de l'entrée `"main"` dans
  `branch_list.ndjson` (`e726318bbc3fd75ac8733a7e030cc35b`). Nécessite un
  lookup via `branch list`/`branch info` pour résoudre le nom affichable
  (déjà couvert par `defaultBranchName` ci-dessous, donc lookup inutile en
  pratique pour l'affichage seul).
- `defaultBranchName` (string) — nom affichable direct, ex. `"main"`.
- `creator` (uuid string) — id utilisateur créateur du repo, **même uuid**
  que le `creator` des branches de ce repo dans `branch_list.ndjson` /
  `branch_info.ndjson` ; résoluble en nom via `authUserInfo` comme ailleurs.
- `created` (number, epoch **secondes**, 10 chiffres) — confirmé par
  comparaison exacte avec le `created` de l'entrée remote `"main"` dans
  `branch_list.ndjson` (`1783270930` des deux côtés, même repo/branche) :
  même résolution que `location:"remote"` dans `branch list`, pas la
  résolution milliseconde de `location:"local"`.

**Impact Tasks 17-19 :** `RepositoryInfoDto` porte
`{ remote_url, id, name, description, default_branch_name, creator, created }`
(option `default_branch` id brut aussi disponible mais redondant avec
`default_branch_name` pour l'affichage). **Pas de champ taille/poids du
repo, pas de compte de révisions** — ces deux lignes envisagées pour le
panneau About sont annulées faute de source CLI ; le panneau se limite à
serveur/id/nom/description/branche par défaut/créateur/date de création.
Tout champ absent (ex. `description` vide traité comme présent-mais-vide,
pas absent) => `None` dans le DTO => ligne masquée (défaut sûr, comme les
autres DTOs de ce lot).

**Push non-fast-forward** (fixture push_nonff.ndjson, capturée le 2026-07-11 en
fabriquant la course : second clone temp sur `feature/test` → commit+push distant →
commit local dans le repo principal → push refusé). Forme constatée : un événement
`{"tagName":"error","data":{"errorType":4294967295,"errorInner":"Branch has diverged, sync to merge remote changes"}}`
suivi de `complete status 1` (il y a aussi un event `log` level=error du même message).
Ce que le frontend reçoit (via `check_ok`, lore.rs:35 → `err.data.to_string()`, clés
triées alpha par serde_json, PUIS suffixe ` (lore exited with …)` du runner streaming) :
`{"errorInner":"Branch has diverged, sync to merge remote changes","errorType":4294967295} (lore exited with …)`.
Sous-chaîne discriminante retenue pour pushErrors.ts : **`Branch has diverged`**
(jamais un match large — un status seul, un stall ou un launch-fail ne matchent pas).
`push --fast-forward-merge` : **existe** ("Allow the server to fast-forward merge if
the target branch head has moved") — MAIS le refus réel est une DIVERGENCE (local ET
remote ont avancé), pas un simple fast-forward ; le CLI dit lui-même « sync to merge »,
donc la voie honnête est la chaîne front sync→push (spec item 2 variante 3B), pas le
flag. Sync de rattrapage sans conflit : **auto-committé** — `sync` émet un
`revisionCommitRevision` (le merge, rev N+1) et laisse `isLocalAhead:1`, `revisionStaged:0`
→ prêt à pousser sans commit manuel. ⚠ Constat annexe : après ce merge propre,
`revisionMerged` reste **non-zéro** (2e parent du commit de merge, permanent tant que
HEAD est ce merge) — donc `merge_in_progress = revisionMerged != 0` (commands.rs:200)
est trop large. Le garde de `syncAndPush` s'appuie sur `stagedPending` (revisionStaged
!= 0), seul discriminant fiable d'un merge NON résolu (cf. status_merge.ndjson : merge
en conflit → revisionStaged non-zéro + fichiers flagConflictUnresolved).

**Shared store** (fixtures shared_store_info.ndjson / shared_store_info_none.ndjson,
capturées le 2026-07-11). `shared-store info --json` émet un event
`{"tagName":"sharedStoreInfo","data":{...}}` puis `complete status 0` dans les DEUX
cas (avec ou sans store — le cas « aucun store » n'est PAS une erreur). Champs (⚠ ce
sont des ARRAYS parallèles par remote, pas un `path` singulier) : `useAutomatically`
(0/1, réglage GLOBAL machine), `remoteUrls` (array de remotes, ex. ["lore.example.com:41337"]),
`paths` (array de chemins de store), `exists` (array de 0/1). Sans store : status 0,
`useAutomatically:0`, arrays vides. Création : `shared-store create <remote-url> [--path]`
— l'URL du remote est REQUISE ; émet `{"tagName":"sharedStoreCreate","data":{"path":"…"}}`.
Store créé dans `%LOCALAPPDATA%\Epic Games\lore\data\<remote>_<port>\shared_store` (RESTE
en place — dev machine). Activation : `shared-store set-use-automatically <true|false>` —
réglage GLOBAL une-fois (pas par-clone), et `clone --help` n'expose AUCUN flag store.
Décision UI (spec item 1) : **variante B — toggle global « Use shared store for clones »
dans l'AvatarMenu**. ⚠ Conséquence pour les commandes Rust : `enable` doit prendre la
server-url (create l'exige) — create le store du remote courant s'il manque, puis
set-use-automatically true ; `disable` = set-use-automatically false (sans arg). `status`
= exists si `exists`/`paths` non vides, path = `paths[0]`, autoUse = `useAutomatically`.

**File diff between two revisions** (fixture file_diff_revs.ndjson, capturée le
2026-07-12). `lore diff <path> --source <revSig> --target <revSig> --json` émet un
`{"tagName":"fileDiff","data":{"path","patch","action"}}` puis `complete status 0`.
⚠ `--source/--target` veulent une SIGNATURE de révision (hash — un NUMÉRO donne
« revision not found »), et le chemin est résolu contre le cwd du process (passer un
chemin ABSOLU, comme le fait déjà `lore_diff`). `action` : **`add`** (patch `--- /dev/null`
→ `+++ path`), **`keep`** = modifié (patch `--- path@<n>` → `+++ path@<m>`), **`delete`**
(patch `--- path@<n>` → `+++ /dev/null`). Le `patch` est un diff unifié standard (séparateur
`\n`, avec le `\r\n` réel du fichier embarqué dans les lignes ; lignes méta `\ No newline at
end of file` présentes). Parsé par le `parse_diff` existant (le même que `lore_diff` du
working-tree). Sert au diff TEXTE historique dans la preview de History (parent→révision du
commit). Le contenu BINAIRE à une révision reste hors de portée (pas de `file cat`).

**Scoped sync (restore one file to an old revision)** (fixture sync_root_file.ndjson,
capturée le 2026-07-12 pour A4). `lore sync <revSig> --root-file <path> --json` synchronise
UNIQUEMENT ce fichier (+ ses dépendances) à la révision cible. Events : `revisionSyncTarget`
{sourceRevision(+Number), targetRevision(+Number), isLatest, local}, `dependencyResolveBegin/End`
{rootCount, resolvedCount} (le filtre de dépendances — resolvedCount > rootCount si le fichier a
des deps), `revisionSyncProgress` {fileUpdate(Total), bytesUpdate(Total), discoveryComplete},
`revisionSyncFile` {path, size, action, flagFile}, `revisionSyncRevision` {revision, revisionNumber,
flagMerge, flagConflict}, puis `complete`. ⚠ La commande DÉPLACE la révision synchronisée vers la
cible (le repo passe « behind » sur ce fichier) — d'où le flux « restore-forward » A4 : sync scopé
→ lire les octets → re-sync au head → réécrire les octets (le fichier devient un changement en
attente `flagDirty:true` action add/modify au head, prêt à committer). Le round-trip peut laisser
un état « staged vide » non-zéro → `unstage` le nettoie. Restore = LOCAL (rien poussé) ; le lock
exclusif du fichier est géré côté UI (bloqué si locké par un tiers, Lock&restore si libre).

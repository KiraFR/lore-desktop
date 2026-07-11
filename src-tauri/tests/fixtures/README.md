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

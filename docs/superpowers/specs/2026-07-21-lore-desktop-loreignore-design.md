# Lore Desktop — .loreignore (ignore côté app)

> **⚠ ADDENDUM 2026-07-21 (post-implémentation) — le CLI supporte NATIVEMENT `.loreignore`.**
> Découvert lors de la vérification réelle : totalement absent de l'aide CLI, mais `lore status --scan` lit `.loreignore` à la racine, exclut les chemins matchés et émet des événements `{"tagName":"filterExclude","data":{"reason":0,"path":"…"}}` (avec DOUBLONS possibles par chemin ; les dossiers exclus sortent comme une seule entrée). Le résumé wire est déjà ajusté. La **négation `!pattern` fonctionne** (testé : `*.tmp` + `!scratch.tmp` → `scratch.tmp` réapparaît) — la syntaxe native est donc plus riche que le sous-ensemble v1 ci-dessous, et le double filtrage front pouvait masquer à tort un fichier ré-inclus par `!`.
>
> **Architecture révisée (décision Jimmy : « on fait le filtre là où est le check de modification »)** : l'app réelle s'appuie sur le natif — `status_from` (Rust) compte les chemins `filterExclude` DÉDUPLIQUÉS → champ `ignoredCount` du StatusResultDto ; le front ne refiltre plus rien (suppression du filtrage dans `refreshStatus`, du garde merge, de l'API `readIgnoreFile`/`lore_read_ignore`). Le module `loreIgnore.ts` reste UNIQUEMENT pour que le mock simule le comportement natif (getStatus mock filtre sa liste et pose `ignoredCount`). Le segment « N ignored » lit `status.ignoredCount`. À remonter à l'équipe Lore : documenter `.loreignore` et `filterExclude` (et la sémantique de `reason`).
>
> Le corps ci-dessous décrit la v1 telle que livrée (`38b6d2e`) AVANT cette découverte — conservé pour l'historique ; la sémantique de matching du module reste valable pour le mock.

Validé par Jimmy le 2026-07-21. Contexte : la CLI Lore n'a **aucun** support ignore *(faux — voir addendum ; l'aide CLI n'en mentionne rien : pas de commande, pas de flag `--exclude`, rien dans `config.toml`)*. L'app fait `lore status --scan`, donc tout fichier de l'arbre apparaît dans Changes — bruit UE (`Saved/`, `Intermediate/`, `DerivedDataCache/`), Blender (`*.blend1`), etc.

## Décisions (arbitrées avec Jimmy)

1. **Effet** : un fichier matché est **masqué de Changes** — absent de la liste, des compteurs et du staging. Mention discrète « N ignored » dans la ligne de résumé quand N > 0.
2. **Syntaxe** : gitignore courant **sans négation `!`** : commentaires `#`, lignes vides, globs `*` / `?`, `**`, suffixe `/` = dossier entier, `/` initial ou interne = ancré à la racine, sinon match à toute profondeur.
3. **Emplacement** : `.loreignore` à la **racine du repo, versionné** via Lore comme n'importe quel fichier — règles partagées par l'équipe.

## Architecture (option A retenue : filtrage front, module TS pur)

Une seule implémentation de la logique, en TS pur (testable vitest, parité mock gratuite). Le Rust ne fait que lire le fichier.

### `src/lib/loreIgnore.ts` (nouveau, pur)

- `parseIgnore(text: string): IgnoreRule[]` — trim, saute vides et `#…`. Pas d'échappement en v1.
- `isIgnored(path: string, rules: IgnoreRule[]): boolean` — `path` repo-relatif avec `/` (format de `ChangedFile.path`). **`.loreignore` n'est jamais ignoré**, quel que soit le contenu des règles.

Compilation d'une règle en RegExp sur le chemin complet :
- retirer un `/` final éventuel (marqueur « dossier » — sans effet supplémentaire en v1 puisque le status ne liste que des fichiers : le pattern couvre déjà tout ce qui est dessous) ;
- conversion glob : `**` → `.*` (traverse les `/`), `*` → `[^/]*`, `?` → `[^/]`, le reste échappé ;
- pattern **contenant un `/`** (hors final) : ancré racine → `^<glob>(/.*)?$` (un `/` initial est simplement retiré avant conversion) ;
- pattern **sans `/`** : toute profondeur → `(^|/)` + glob + `(/.*)?$` — le suffixe `(/.*)?` fait qu'un nom de dossier matché ignore tout son contenu, comme gitignore.

Exemples normatifs (à reprendre en tests) : `Saved/` ignore `Saved/x.txt` et `Sub/Saved/y.txt` ; `*.blend1` ignore à toute profondeur ; `/Config/*.ini` ignore `Config/a.ini` mais pas `Sub/Config/a.ini` ni `Config/deep/a.ini` ; `Content/**/Temp` ignore `Content/A/B/Temp/f.txt` ; `.loreignore` dans les règles n'ignore pas `.loreignore`.

### API (parité obligatoire types.ts + mock.ts + tauri.ts)

- `readIgnoreFile(repoPath: string): Promise<string | null>` — contenu brut du `.loreignore` racine, `null` si absent.
- Rust : commande `lore_read_ignore` — `fs::read_to_string(<repo>/.loreignore)` en `spawn_blocking`, `Ok(None)` si absent, erreur sinon. Tests cargo : présent / absent.
- Mock : renvoie un `.loreignore` d'exemple (ex. `Saved/` + `*.tmp`) et la liste de fichiers mock gagne 1–2 fichiers matchés (ex. `Saved/autosave.tmp`) pour rendre le filtre visible en preview. Adapter les tests mock existants si des comptes changent.

### Application — dans le store, en amont de tout

Dans `refreshStatus` (repo.svelte.ts) : après `getStatus`, `readIgnoreFile` (relu à CHAQUE refresh — fichier minuscule, toujours à jour, y compris quand on édite le `.loreignore` lui-même puisqu'il apparaît alors comme changement), puis filtrage de la liste avant stockage. Tout consommateur (Changes, compteurs, vignettes, FilePreview) voit la liste déjà filtrée. Le nombre de fichiers retirés est stocké (ex. `repo.ignoredCount`) pour l'UI. Échec de lecture du fichier → toast + comportement « pas de règles » (ne jamais casser le status).

### Garde-fous

- **Les conflits de merge ne sont JAMAIS filtrés** (un conflit doit rester visible et résoluble).
- Locks et History non touchés (données serveur ; un lock sur un fichier ignoré reste de l'info).
- Un `.loreignore` vide/absent = zéro filtrage, zéro coût.

### UI

`summaryParts` (statusSummary.ts) : segment muted « N ignored » quand N > 0. Rien d'autre en v1.

## Hors périmètre v1 (suites possibles, non engagées)

Négation `!pattern` ; template UE/Blender proposé quand le fichier n'existe pas ; toggle « Show ignored » ; ignore local par repo en plus du versionné.

## Tests & vérification

- vitest : `loreIgnore.test.ts` (exemples normatifs ci-dessus + commentaires/vides/casse) ; tests des helpers touchés (`statusSummary`) ; tests mock adaptés.
- cargo : `lore_read_ignore` présent/absent.
- Preview navigateur (mock) : fichiers matchés absents de la liste et des compteurs, « N ignored » visible, `.loreignore` lui-même listé comme changement quand modifié.
- Vérif réelle ensuite (session principale) : `.loreignore` + fichier poubelle dans `lore-test-repo`, app réelle.

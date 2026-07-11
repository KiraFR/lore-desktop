# Lore Desktop — Lot P4 « visibilité du repo » (design)

Date : 2026-07-11. Items P2 « lecture seule » de l'audit n°2 — enrichissements
d'affichage sans nouveau flux d'écriture. Conventions des lots P1-P3 (captures
wire obligatoires pour tout champ non pinné, exécution Opus subagent-driven).

## Cinq items (tous read-only)

### 1. Compteurs +/~/− dans l'en-tête de Changes

L'événement `repositoryStatusSummary` (audit §P2) porte des compteurs
adds/deletes/modifies. CAPTURE en tête d'item : forme exacte (la fixture P1
`status --scan` en contient peut-être déjà un — vérifier avant de recapturer).
Backend : les remonter dans `StatusResult` (`summary?: { adds, mods, dels }`,
absent si l'événement manque). UI : dans le colhead de Changes, après le compte
de fichiers : « +3 ~2 −1 » en couleurs `--added/--modified/--deleted`, masqué si
absent ou tout à zéro. Le mock seed des valeurs cohérentes avec ses fichiers.

### 2. Section « Remote » dans le BranchMenu

`branchListEntry.location` (audit §P2) distingue local/remote. CAPTURE : valeurs
réelles du champ (la fixture branch list existe-t-elle ? sinon capturer).
Backend : exposer `location` dans le DTO Branch. UI : le BranchMenu groupe —
branches locales d'abord (section actuelle sans titre), puis un séparateur
« Remote » et les branches remote-only (estompées, switch autorisé — le CLI
fait le checkout). Le filtre s'applique aux deux groupes. Mock : 2-3 branches
marquées remote-only.

### 3. Ahead/behind par branche dans le BranchMenu

`lore branch info <name>` (audit §P2) — CAPTURE : champs réels (ahead/behind ?
créateur ? date ?). Décision de design : PAS un appel par branche au montage
(2000 branches dans le stress-test mock) — fetch LAZY au survol/focus d'une
ligne (débouncé 150 ms, caché en Map par nom, TTL session), affiché en fin de
ligne « ↑2 ↓5 » discret. Si `branch info` s'avère coûteux ou sans ces champs,
replier l'item sur : ahead/behind de la SEULE branche courante dans le header du
menu (déjà connu via status local/remote) et documenter l'écart.

### 4. Badge « protected » (lecture seule)

L'écriture protect/unprotect reste au CLI (décision d'audit). CAPTURE : le flag
protected apparaît-il dans `branch list` ou `branch info` ? Si OUI : badge
cadenas discret sur la ligne de branche (tooltip « Protected — direct pushes
are rejected by the server »), et le bouton Push affiche ce tooltip enrichi
quand la branche courante est protégée (le push reste tenté — le serveur fait
autorité, on ne devine pas). Si le flag n'existe nulle part en lecture :
documenter, item annulé (constat négatif = résultat valide).

### 5. Panneau « About repository »

`lore repository info` — CAPTURE : champs réels (id, nom, taille, serveur ?).
UI : entrée « About repository » dans le menu du RepoSwitcher (ou l'AvatarMenu
si plus naturel — trancher au plan selon l'espace), ouvrant un petit panneau
modal read-only : nom, id (copiable), chemin local (bouton Reveal), serveur,
branche courante, révision, + les champs utiles découverts à la capture.
Fermeture Escape/clic dehors (pattern ContextMenu).

## Hors périmètre

Toute écriture (protect/unprotect, metadata), `repository config`, la recherche
(lot P5), tri/groupement avancé des branches, pagination du branch info.

## Tests

Vitest : parsing/formatage des compteurs, groupement local/remote, cache du
branch info lazy (Map + TTL logique pure), état protected → tooltip. Rust :
fixtures de chaque capture + extraction DTO (défauts sûrs si champs absents —
pattern merge/staged du P1). Mock : tout doit vivre en dev navigateur.
Vérification finale : suites + parcours navigateur + captures réelles faites.

## Ordre de livraison

1 (compteurs) → 2 (Remote) → 4 (protected — petite, dépend de la capture de 2) →
3 (ahead/behind lazy) → 5 (About).

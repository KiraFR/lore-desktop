# Lore Desktop — Lot P6 « serveur & création » (design)

Date : 2026-07-11. Derniers items P2/P3 de l'audit n°2 qui touchent le serveur.
Conventions P1-P5. ⚠ L'item 3 est CONDITIONNÉ à un arbitrage produit de Jimmy —
le plan le prépare, l'exécution le saute si l'arbitrage n'est pas donné.

## Trois items

### 1. Shared store au clone

L'audit classe le shared store (cache d'objets partagé entre working copies —
gros gain disque avec plusieurs clones) comme le plus pertinent du trio
shared-store. DISCOVERY OBLIGATOIRE en tête d'item (le CLI n'a jamais été
capturé sur ce sujet) : `lore shared-store create/info/set-use-automatically
--help` + essai réel — comment un clone « utilise » le store (flag de clone ?
`set-use-automatically` global avant le clone ?), que renvoie `info` quand
aucun store n'existe, où vit le store.

**Design v1 (à ajuster au constat)** : une case « Use shared store » dans le
flux Clone (RepoPicker + RepoSwitcher, sous le choix du dossier — cochée par
défaut si un store existe déjà, décochée + hint « creates a shared object
store » sinon). Backend : `lore_shared_store_status() -> { exists, path? }` et
la mécanique d'activation découverte (create si besoin + set-use-automatically
ou flag). Si la discovery montre que c'est un réglage GLOBAL une-fois (pas
par-clone), replier l'UI sur un simple toggle « Use shared store for clones »
dans l'AvatarMenu, activé une fois pour toutes — trancher au constat et
documenter.

### 2. « Sync & push » sur push refusé

Quand `lore push` échoue parce que le remote a avancé (l'app a déjà
remoteAhead dans le status la plupart du temps, mais la course existe),
l'artiste voit juste « Push failed ». CAPTURE en tête d'item : message/forme
exacte de l'erreur de push non-fast-forward (créer la situation sur le repo de
test : commit local + commit distant simulé par un second clone temp).
Design : détection de ce cas précis dans le catch du push (match sur l'erreur
pinnée — PAS un match large) → au lieu du toastError, un toast ACTION
« Remote has new changes » avec bouton « Sync & push » qui enchaîne
`sync()` puis `push()` (les deux streaming, progression existante ; si le sync
ouvre un merge conflictuel, le flux s'arrête là proprement — le chip merge
existant prend le relais, pas de push automatique). L'option `push
--fast-forward-merge` mentionnée par l'audit : la DISCOVERY dira si elle fait
ça en un coup côté CLI — si oui, l'utiliser à la place de la chaîne
sync-puis-push et le documenter.

### 3. Créer un repository depuis l'app — ⚠ CONDITIONNÉ

Exclu volontairement au lot repo-switcher (2026-07-08), réouvert par l'audit
(« parité GitHub Desktop/Anchorpoint »). **NE S'EXÉCUTE QUE si Jimmy dit
explicitement oui** — le plan existe pour ne pas refaire le design le jour venu.

DISCOVERY : `lore repository create --help` + essai réel sur le serveur de test
(créer puis vérifier qu'il apparaît dans `repository list` ; le supprimer
ensuite si `repository delete` le permet, sinon repo de test permanent nommé
`desktoptest-created-<date>` et le signaler).
Design : bouton « New repository… » dans le sous-menu Add du RepoSwitcher (et
la carte serveur du RepoPicker) → mini-formulaire nom + dossier parent local →
create + clone/init selon la sémantique découverte → selectRepo. Validation du
nom (charset découvert à la discovery), erreurs serveur en toastError.

## Hors périmètre

`repository delete` depuis l'app, gestion fine du store (purge, taille — un
futur écran Réglages), auto-sync périodique, retry automatique du push.

## Tests

Vitest : détection de l'erreur push pinnée (fonction pure sur le message),
état de la case shared-store, validation du nom de repo. Rust : fixtures des
discoveries + commandes. Mock : les trois flux vivants en dev (levier d'échec
de push non-fast-forward dans le mock). Vérification finale réelle : un push
refusé réel résolu par le toast, un clone avec store partagé, et (si arbitré)
une création de repo bout-en-bout.

## Ordre de livraison

2 (Sync & push — le plus utile au quotidien) → 1 (shared store) →
3 (create, si arbitré).

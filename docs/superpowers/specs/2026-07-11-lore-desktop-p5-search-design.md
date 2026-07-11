# Lore Desktop — Lot P5 « recherche & navigation » (design)

Date : 2026-07-11. Deux items courts de l'audit n°2 (P2). Conventions P1-P4.

## Deux items

### 1. Recherche dans History

**Décision de design (Fable)** : v1 = filtrage CLIENT sur les commits DÉJÀ
chargés (message, auteur, hash court, #rev), sans `lore revision find` — le
préchargement (P2) + la pagination existante donnent déjà des centaines de
commits en mémoire, et `revision find` a une sémantique serveur inconnue
(match unique ? multi ?) qui mériterait sa propre capture pour un gain marginal.
Documenter ce choix dans la spec du composant ; si l'usage réclame du
plein-historique, `revision find` sera un lot futur.

- UI : champ filtre en tête de la colonne graphe (pattern du filtre Changes,
  placeholder « Filter commits »), débouncé 150 ms. Quand actif : la liste
  virtualisée n'affiche que les commits matchés (le GRAPHE des lanes est masqué
  en mode filtre — les arêtes n'ont pas de sens sur une liste filtrée ; lignes
  plates avatar + message + date), compteur « N of M loaded commits » + hint
  « Searching loaded commits only — scroll History to load more » (le bouton
  « Load more » reste accessible en bas de liste filtrée et déclenche
  loadMoreHistory ; les nouveaux commits entrent dans le filtre).
- Matching : insensible à la casse, sous-chaîne sur message/auteur/hash/`#N`.
  Fonction pure `filterCommits(commits, query)` testée vitest (y compris le
  match par numéro « 42 » → #42 en préfixe).
- La sélection de commit survit au filtre si le commit reste visible, sinon
  reset (pattern selectionAfterCommitChange).
- Escape vide le filtre si le champ a le focus (cohérent avec l'éditeur de
  message ; ne pas casser l'Escape du preview panel — garde input existante).

### 2. Filtre dans Locks

Champ filtre en tête de la vue Locks (même pattern/`filterByQuery` que Changes —
réutiliser le helper de changesPartition s'il s'applique tel quel au type
LockEntry, sinon petite surcharge générique `filterByPath`). Matche le chemin ET
le détenteur. Compteur « N of M » quand actif. Trivial, aucun backend.

## Hors périmètre

`revision find` serveur, recherche dans Changes (le filtre existe déjà),
recherche plein-texte dans les diffs, tri des résultats.

## Tests

Vitest : `filterCommits` (casse, hash, #N, requête vide), survie/reset de la
sélection, filtre Locks (path + holder). Navigateur mock : les deux vues, le
hint « loaded commits only », Load more sous filtre. Pas de capture wire ni de
Rust dans ce lot.

## Ordre de livraison

2 (Locks — échauffement trivial) → 1 (History).

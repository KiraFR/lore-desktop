# Lore Desktop — File history par asset

**Date :** 2026-07-09
**Statut :** Design — item n°2 du programme (notifications → **file history** → palier 4)
**Constat CLI (capturé) :** `lore file history <abs> --repository <repo> --json` émet, par révision touchant le fichier : `fileHistory { revision, revisionNumber, size, action }` suivi des `metadata` (message, timestamp, created-by, branch). Les `created-by` sont des UUID **non résolus** — `lore auth info <id…> --repository <repo>` les résout en `authUserInfo { id, name }` (name = email).

## Objectif

Répondre à la question n°1 d'un artiste : *« qui a touché cet asset, quand, pourquoi, et à quel poids ? »* — une timeline du fichier sélectionné, directement dans FilePreview.

## Backend

`lore_file_history(repo_path, path) -> Vec<FileRevisionDto>` dans `commands.rs` :

```rust
FileRevisionDto { revision, revision_number, action, size, message, author, when, when_ms }
```

- Chemin passé en **absolu** (même gotcha cwd que lock/diff/reset).
- Parcours du flux au même pattern que `history_from` : `fileHistory` ouvre une entrée, les `metadata` suivants la remplissent (`message`, `created-by`, `timestamp`) ; `action` mappé par `map_action` (keep → modify).
- Résolution des auteurs : collecte des `created-by` distincts → **un** appel `lore auth info <ids…>` → map id→email ; échec ⇒ l'UUID reste (le front tronque).
- `when` = `relative_time(ms)` ; `when_ms` exposé pour le tooltip de date absolue.
- Test unitaire sur fixture inline (2 révisions + metadata) : ordre, mapping action, message, timestamp.

## Front

- **Types/API** : `FileRevision { revision, revisionNumber, action, size, message, author, when, whenMs }` ; `LoreApi.getFileHistory(repoPath, path)`. Mock : 3 révisions synthétiques (add → modify par 'Maya R' → modify par 'you') pour le dev.
- **FilePreview** : section « History » sous la table de méta — chargée à la sélection (pattern anti-course des diff/preview). Par ligne : glyphe d'action, **rev N**, message (ellipsé), auteur (`you` si = identité, sinon partie locale de l'email), temps relatif avec **date absolue en tooltip** (`whenMs`), taille formatée. Affichage plafonné à 30 lignes + « … and N more revisions ». États loading/erreur discrets (pas de toast).

## Hors périmètre

Ouvrir la révision (nécessite `lore file cat <rev>` — manque CLI), diff entre deux révisions du fichier, sync-to-revision (item futur).

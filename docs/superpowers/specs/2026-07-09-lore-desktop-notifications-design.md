# Lore Desktop — Notifications temps réel

**Date :** 2026-07-09
**Statut :** Design — item n°1 du programme validé (notifications → file history → palier 4)
**Constat CLI (capturé) :** `lore notification subscribe --repository <path> --json` est un flux long-vivant qui émet `notificationSubscribed`, puis `notificationBranchPushed { revision, revisionNumber, branch, userId }` et `notificationResourceLocked|Unlocked { userId, branch, paths }`.

## Objectif

L'app reflète l'activité de l'équipe **sans action de l'utilisateur** : badge Sync qui apparaît quand un coéquipier push, locks à jour en live, toast discret pour les pushes des autres. Aujourd'hui le refresh n'a lieu qu'au focus fenêtre.

## Architecture

### Rust — `src-tauri/src/notifications.rs`

- **État géré** `NotifState(Mutex<Option<(u64, Child)>>)` : l'abonnement courant, taggé par un numéro de **génération** (`AtomicU64` global). Toute (re)souscription incrémente la génération ; un lecteur dont la génération est dépassée s'arrête sans respawn — pas de course entre stop/start rapides.
- **`lore_notifications_start(app, state, repo_path)`** : tue l'abonné précédent, lance un thread qui boucle : spawn `lore notification subscribe --repository <path> --json` (stdout pipé, `CREATE_NO_WINDOW` sous Windows), lit ligne à ligne, parse le JSON, et **forwarde** chaque événement dont le tag passe `is_forwardable` (préfixe `notification`, sauf `notificationSubscribed`) vers le webview via `app.emit("lore://notification", value)`. Fin de flux (serveur redémarré, réseau) → pause 3 s puis respawn tant que la génération est inchangée.
- **`lore_notifications_stop(state)`** : bump génération + kill.
- Le processus enfant meurt avec l'app (pipe stdout cassé) ; le kill explicite couvre les switchs de repo.
- Test unitaire : `is_forwardable`.

### Front

- **`LoreApi.startNotifications(repoPath, onEvent): Promise<() => void>`** — tauri : `listen('lore://notification')` + invoke start, le retour désabonne et invoque stop ; mock : no-op (pas de simulation de bruit en dev).
- **`notifyRouting.ts`** (pur, testé) : `planFor(events, myUserId)` → `{ status, locks, pushToast }` — push ⇒ refresh status+history, toast seulement si `userId !== moi` ; lock/unlock ⇒ refresh locks.
- **`notifications.svelte.ts`** : `watchRepo(path | null)` (idempotent, jeton anti-course) ; les événements sont **coalescés 400 ms** puis le plan s'exécute : `refreshStatus(true)` / `refreshHistory(true)` / `refreshLocks(true)` — tous silencieux, jamais de spinner. Toast : « Rev N pushed by a teammate ».
- **App.svelte** : `$effect` sur `session.signedIn && session.config.currentRepo` → `watchRepo(...)` (null coupe l'abonnement au sign-out ou sans repo).

## Vérification

Suites TS/Rust, puis **simulation d'équipe réelle** : l'app ouverte sur `lore-test-repo` pendant que le CLI (même machine, autre « acteur ») verrouille un fichier → la vue Locks/status bar se met à jour seule ; commit+push CLI → badge Sync apparaît sans toucher l'app. (Le toast « teammate » ne se déclenchera pas — même userId — c'est attendu et couvert par le test unitaire de `planFor`.)

## Hors périmètre

Résolution userId → nom d'affichage dans le toast (nécessiterait un annuaire ; « a teammate » suffit), notifications multi-repos en arrière-plan pour le switcher (P3 de l'audit), notifications OS natives.

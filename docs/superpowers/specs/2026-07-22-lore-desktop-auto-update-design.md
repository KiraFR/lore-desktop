# Lore Desktop — Auto-update (updater Tauri 2 + releases GitHub)

Validé par Jimmy le 2026-07-22. Contexte : app distribuée à l'équipe (Windows, setup NSIS), aucune CI ni release à ce jour. Objectif : les mises à jour s'installent depuis l'app, publication par tag.

> **RÉVISION 2026-07-23** : le repo `KiraFR/lore-desktop` est devenu **PUBLIC** (après audit + purge d'historique). Les releases se font donc directement dessus — le repo séparé `lore-desktop-releases` initialement prévu est abandonné (supprimé). Conséquences : endpoint updater = `https://github.com/KiraFR/lore-desktop/releases/latest/download/latest.json`, publication CI via le `GITHUB_TOKEN` du workflow (`permissions: contents: write`) — **plus de PAT `RELEASES_TOKEN`**, plus de publication croisée ; minutes Actions gratuites (repo public).

## Décisions (arbitrées avec Jimmy)

1. **Hébergement** : releases GitHub du repo `KiraFR/lore-desktop` lui-même (public) — setup NSIS signé + `latest.json` ; l'app pointe sur `https://github.com/KiraFR/lore-desktop/releases/latest/download/latest.json`.
2. **UX** : check silencieux au démarrage + toutes les **4 h** ; si version dispo, bannière discrète « Update available — vX.Y.Z » avec bouton **Install & restart** (progression de téléchargement puis relance). Jamais d'installation sans clic. Preferences ▸ Support : version courante + bouton « Check for updates » (résultat visible : up to date / update / erreur ; en check auto les erreurs réseau restent silencieuses).
3. **Publication** : **GitHub Actions** sur le repo privé, déclenché par tag `v*` — runner Windows, build + signature, publication croisée sur le repo public.

## Mécanique updater

- Crates `tauri-plugin-updater` + `tauri-plugin-process` (relance) ; packages npm `@tauri-apps/plugin-updater` + `@tauri-apps/plugin-process`. Capabilities : `updater:default`, `process:allow-restart`.
- `tauri.conf.json` : `bundle.createUpdaterArtifacts: true` ; section `plugins.updater` avec `endpoints` (URL ci-dessus) et `pubkey` minisign. Windows `installMode: "passive"`.
- **Clés (manipulées par Jimmy uniquement, JAMAIS committées ni transmises à Claude — seule la clé PUBLIQUE entre dans le repo)** : `npx tauri signer generate -w <fichier hors repo>` ; secrets GitHub `TAURI_SIGNING_PRIVATE_KEY` (+ `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` si passphrase) posés via `gh secret set` ; sauvegarde de la clé privée dans le gestionnaire de mots de passe de Jimmy. Tant que la vraie pubkey n'est pas fournie, un placeholder documenté `TAURI_UPDATER_PUBKEY_PLACEHOLDER` occupe le champ (l'updater est alors inopérant en local, c'est attendu).

## CI (`.github/workflows/release.yml`)

Déclencheur `push` sur tags `v*`. Étapes : checkout ; setup Node 20 + Rust stable (cache cargo) ; `npm ci` ; `npx tauri build` avec `TAURI_SIGNING_PRIVATE_KEY`/`…_PASSWORD` en env ; script Node `scripts/make-latest-json.mjs` qui lit la version de `tauri.conf.json`, le `.sig` produit, et écrit `latest.json` (`version`, `notes` = corps du tag ou défaut, `pub_date`, `platforms."windows-x86_64"` = `{signature, url}` avec l'URL du setup dans la release de CE repo, nom d'asset stable encodé) ; publication via `gh release create <tag>` sur le repo courant avec le `GITHUB_TOKEN` du workflow (`permissions: contents: write`) en uploadant le setup `.exe`, son `.sig` et `latest.json`. Garde-fou : le job échoue si la version du tag ≠ version de `tauri.conf.json` (et `Cargo.toml`/`package.json` si versionnés — vérifier la source de vérité au moment de l'implémentation ; le bump reste manuel en v1).

## Dans l'app

- **`src/lib/updater.ts`** (pur, testé vitest) : machine d'état du check — `idle | checking | available(version, notes) | downloading(pct) | ready | upToDate | error(msg)`, politique d'intervalle (4 h, re-check au focus si > 4 h écoulées), et mapping des erreurs (check auto : silencieux ; manuel : visible).
- **`src/lib/updates.svelte.ts`** : store runes qui pilote la machine via l'API ; démarre le cycle au boot.
- **API (parité types/mock/tauri OBLIGATOIRE)** : `checkForUpdate(): Promise<{version, notes} | null>`, `downloadAndInstall(onProgress): Promise<void>` (l'impl tauri relance l'app via plugin-process après install ; ne résout jamais côté appelant), `getAppVersion(): Promise<string>`. `tauri.ts` passe par `@tauri-apps/plugin-updater` ; **mock** : version simulée disponible après ~2 s (bannière visible en dev navigateur), progression simulée, `downloadAndInstall` finit par un no-op logué.
- **UI** : bannière discrète (zone StatusBar, style des chips existants) « Update available — vX.Y.Z · Install & restart » → progression pendant le téléchargement ; Preferences ▸ Support : ligne version courante + bouton « Check for updates » avec état.
- L'updater ne touche PAS à `LoreApi`-le-VCS conceptuellement mais vit dans la même interface pour bénéficier du pattern de parité existant.

## Vérification

- vitest : machine d'état (transitions, intervalle, silencieux vs manuel) ; suites complètes vertes.
- Preview navigateur : bannière mock, progression, bouton Preferences.
- **E2E réel (session principale, après pose des clés)** : publier `v0.1.1` par tag → vérifier que l'exe `v0.1.0` installé affiche la bannière, se met à jour et relance en `v0.1.1`.

## Hors périmètre v1

Canal beta/stable, delta updates, notes de version riches dans la bannière (la bannière montre la version ; les notes complètes sont sur la page release), rollback, macOS/Linux (l'équipe est Windows ; les artefacts multi-OS s'ajouteront au workflow si besoin).

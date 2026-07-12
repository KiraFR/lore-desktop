# Lore Desktop — Écran de préférences + « Open logs » Implementation Plan

> **STATUT : EXÉCUTÉ ET VÉRIFIÉ le 2026-07-12** (subagent-driven Tasks 1-4, double revue : READY ; + ajout d'un vrai glyphe `settings` (engrenage Feather) car `Icon.svelte` n'en avait pas — l'implémenteur avait mis `edit`).
>
> **Suites** : vitest **180 passed / 25 fichiers**, cargo `--lib` **115 passed**, `npm run check` **0/0**.
>
> **Constat CLI pinné** : `lore logfile info` sort une ligne texte `Location: <chemin>` (PAS de NDJSON) → runner brut `run_lore_raw` + parser `logfile_location_from`. Dossier global `%LOCALAPPDATA%\Epic Games\lore\data\logs`.
>
> **Vérif navigateur mock** : menu compte **épuré** (Preferences… + Sign out, plus AUCUN toggle inline). Modal **Préférences** : 4 sections **Account** (display name + email) / **Appearance** (segment Dark-Light, bascule en direct dans les deux sens) / **Clones** (toggle shared store) / **Support** (« Open logs » sans erreur + chemin de logs affiché). Escape ferme. Console sans erreur. Les 3 réglages sont GONE de l'AvatarMenu, présents uniquement dans le modal (décision Jimmy « tout dans Préférences »).
>
> **Déviation** : glyphe `settings` ajouté (engrenage) et utilisé pour l'entrée menu + le titre du modal.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Un vrai écran de **Préférences** (modal) qui regroupe les réglages aujourd'hui éparpillés dans le menu compte (display name, thème, shared store) + une section **Support** avec un bouton « Open logs » qui ouvre le dossier de logs du CLI. Le menu compte (AvatarMenu) est allégé : identité + « Preferences… » + Sign out.

**Architecture:** Aucune nouvelle logique métier — on RELOCALISE l'UI existante (`setDisplayName`, `setTheme`/`resolveTheme`, `sharedStoreStatus/Enable/Disable`) du menu compte vers un nouveau composant `Preferences.svelte` (modal calqué sur `AboutRepo.svelte`, monté par la TitleBar). Le seul ajout backend : `lore_logfile_location` qui shelle `lore logfile info` (sortie **texte brut**, PAS du NDJSON) et parse `Location: <chemin>` ; le front ouvre ce dossier via le `openPath` existant.

**Tech Stack:** Svelte 5 runes, TypeScript, vitest (jsdom), Rust (Tauri v2), PowerShell pour la capture.

---

## Contexte & conventions (à lire avant toute tâche)

- **Constat CLI (capturé le 2026-07-12)** : `lore logfile info` (avec OU sans `--json`) écrit **une seule ligne texte** sur stdout : `Location: C:\Users\jimmy\AppData\Local\Epic Games\lore\data\logs`. **PAS de NDJSON, pas d'événement `complete`** → `run_lore` (qui append `--json` et parse le NDJSON via `parse_events`) NE PEUT PAS servir ici : il faut un runner **stdout brut**. Global (pas besoin de `--repository`). Le dossier existe.
- **Réglages déjà câblés** (à relocaliser, PAS à réécrire) : `setDisplayName(name)` (session.svelte.ts), `setTheme(theme)` + `resolveTheme(cfg.theme)` (session.svelte.ts / theme.ts), `api.sharedStoreStatus()/sharedStoreEnable(serverUrl)/sharedStoreDisable()`. Tous vivent aujourd'hui dans `AvatarMenu.svelte`.
- **Pattern modal** : `AboutRepo.svelte` est le gabarit (scrim `.scrim`, `.panel bind:this`, fermeture Escape + `pointerdown` hors panneau via un `$effect` avec listeners document, monté par `TitleBar.svelte` avec un `$state` `xOpen` ouvert depuis un menu). `Preferences.svelte` s'en inspire.
- **Ouvrir un dossier** : `api.openPath(absPath)` (os_open_path) ouvre un dossier dans l'explorateur ; `api.revealPath` le montre sélectionné dans son parent. Pour le dossier de logs → `openPath`.
- **Contrainte vitest** : pas de `.svelte`/`.svelte.ts` importable ; la logique testée reste en Rust (le parser) ou dans les helpers purs existants. Le wiring composant se vérifie navigateur.
- **Commandes de test** : Rust `cargo test --manifest-path src-tauri/Cargo.toml --lib <filtre>` ; vitest `npx vitest run` ; `npm run check` (0/0) ; dev `npm run dev` → http://localhost:5173.

## Carte des fichiers

**Créés :**
- `src/lib/Preferences.svelte` — le modal Préférences (4 sections).

**Modifiés :**
- `src-tauri/src/lore.rs` — `run_lore_raw` (runner stdout brut).
- `src-tauri/src/commands.rs` — `logfile_location_from` (parser pur) + `lore_logfile_location` (commande) + test.
- `src-tauri/src/lib.rs` — enregistrement de `lore_logfile_location`.
- `src/lib/types.ts` — `LoreApi.logfileLocation`.
- `src/lib/tauri.ts` — `logfileLocation`.
- `src/lib/mock.ts` — `logfileLocation` mock.
- `src/lib/AvatarMenu.svelte` — RETIRER display name / shared store / appearance ; AJOUTER « Preferences… ».
- `src/lib/TitleBar.svelte` — monter `Preferences.svelte` + câbler l'ouverture.

---

### Task 1: Backend — `lore_logfile_location` (runner brut + parser + test)

**Files:**
- Modify: `src-tauri/src/lore.rs` (ajouter `run_lore_raw`), `src-tauri/src/commands.rs` (parser + commande + test), `src-tauri/src/lib.rs`

- [ ] **Step 1: Runner stdout brut**

Dans `src-tauri/src/lore.rs`, ajouter (calqué sur `run_lore` mais SANS `--json` ni parse NDJSON — READ `run_lore` d'abord pour copier exactement le pattern thread+timeout `LORE_TIMEOUT`) :

```rust
/// Run `lore <args>` (NO `--json`) and return raw stdout as a string. For the
/// few commands that print human text instead of NDJSON (e.g. `logfile info`).
/// Same helper-thread + timeout guard as `run_lore`.
pub fn run_lore_raw(args: &[&str]) -> Result<String, String> {
    let owned: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(Command::new("lore").args(&owned).output());
    });
    match rx.recv_timeout(LORE_TIMEOUT) {
        Ok(Ok(o)) => Ok(String::from_utf8_lossy(&o.stdout).into_owned()),
        Ok(Err(e)) => Err(format!("failed to launch lore: {e}")),
        Err(_) => Err("lore command timed out".to_string()),
    }
}
```

(⚠ vérifier que `Command` et `LORE_TIMEOUT` sont déjà en scope dans `lore.rs` — ils le sont, utilisés par `run_lore`.)

- [ ] **Step 2: Écrire le test du parser (échoue)**

Dans `src-tauri/src/commands.rs`, ajouter un module de test :

```rust
#[cfg(test)]
mod logfile_tests {
    use super::*;

    #[test]
    fn parses_the_location_line() {
        let out = "Location: C:\\Users\\jimmy\\AppData\\Local\\Epic Games\\lore\\data\\logs\n";
        assert_eq!(
            logfile_location_from(out).as_deref(),
            Some("C:\\Users\\jimmy\\AppData\\Local\\Epic Games\\lore\\data\\logs"),
        );
    }

    #[test]
    fn none_when_no_location_line() {
        assert_eq!(logfile_location_from("some unrelated output\n"), None);
        assert_eq!(logfile_location_from(""), None);
    }
}
```

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib logfile_tests` — Expected: erreur de compilation (`logfile_location_from` inexistant).

- [ ] **Step 3: Implémenter le parser + la commande**

Ajouter dans `commands.rs` (importer `run_lore_raw` depuis `crate::lore` — vérifier la façon dont `run_lore`/`run_lore_op` sont importés en tête de fichier et suivre le même `use`) :

```rust
/// The logs directory from `lore logfile info` — the CLI prints a single line
/// `Location: <abs path>` (plain text, not NDJSON). Returns the trimmed path.
fn logfile_location_from(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .find_map(|l| l.strip_prefix("Location:"))
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
}

/// Absolute path of the CLI's log directory, for the Preferences "Open logs"
/// button. Global (no repository needed).
#[tauri::command]
pub async fn lore_logfile_location() -> Result<String, String> {
    blocking(move || {
        let out = run_lore_raw(&["logfile", "info"])?;
        logfile_location_from(&out).ok_or_else(|| "couldn't read the log location".to_string())
    })
    .await
}
```

Dans `src-tauri/src/lib.rs`, ajouter à l'`invoke_handler` :

```rust
        commands::lore_logfile_location,
```

- [ ] **Step 4: Vérifier**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib logfile_tests` — Expected: `ok` (2 tests).
Run: `cargo test --manifest-path src-tauri/Cargo.toml --lib` — Expected: `ok`, 0 failed.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/lore.rs src-tauri/src/commands.rs src-tauri/src/lib.rs
git commit -m "feat(logs): lore_logfile_location — parse the CLI log directory path"
```

---

### Task 2: Surface d'API `logfileLocation` (types / tauri / mock)

**Files:**
- Modify: `src/lib/types.ts`, `src/lib/tauri.ts`, `src/lib/mock.ts`

- [ ] **Step 1: Type + API + mock**

`src/lib/types.ts` — dans `LoreApi`, près de `revealPath`/`openPath` :

```ts
  /** Absolute path of the CLI's log directory (for the Preferences "Open logs" button). */
  logfileLocation(): Promise<string>
```

`src/lib/tauri.ts` — près de `openPath:` :

```ts
  logfileLocation: () => invoke<string>('lore_logfile_location'),
```

`src/lib/mock.ts` — près de `openPath` :

```ts
  async logfileLocation() {
    await delay(80)
    return 'C:/Users/jimmy/AppData/Local/Epic Games/lore/data/logs'
  },
```

- [ ] **Step 2: Vérifier**

Run: `npm run check` — Expected: 0 errors, 0 warnings.

- [ ] **Step 3: Commit**

```bash
git add src/lib/types.ts src/lib/tauri.ts src/lib/mock.ts
git commit -m "feat(logs): logfileLocation API surface + mock"
```

---

### Task 3: `Preferences.svelte` — le modal

**Files:**
- Create: `src/lib/Preferences.svelte`

- [ ] **Step 1: Construire le composant**

READ `AboutRepo.svelte` (gabarit du modal : scrim, panel, Escape/pointerdown-outside) ET la version actuelle de `AvatarMenu.svelte` (pour reprendre VERBATIM la logique de display name, thème, shared store). Créer `src/lib/Preferences.svelte` :

```svelte
<script lang="ts">
  import { api } from './api'
  import { session, setDisplayName, setTheme } from './session.svelte'
  import { resolveTheme } from './theme'
  import { toastError } from './toast'
  import Icon from './Icon.svelte'

  let { onclose }: { onclose: () => void } = $props()

  // --- Account: display name (relocated from AvatarMenu) ---
  let name = $state(session.config.displayName ?? '')
  const email = $derived(session.identity?.email ?? null)
  async function saveName() {
    if ((session.config.displayName ?? '') === name.trim()) return
    await setDisplayName(name)
  }

  // --- Appearance (relocated) ---
  const theme = $derived(resolveTheme(session.config.theme))

  // --- Clones: shared store (relocated) ---
  let storeOn = $state<boolean | null>(null)
  $effect(() => {
    api.sharedStoreStatus()
      .then((s) => { storeOn = s.autoUse ?? s.exists })
      .catch(() => { storeOn = null })
  })
  async function toggleStore() {
    if (storeOn === null) return
    const serverUrl = session.config.serverUrl
    if (!serverUrl) return
    const target = !storeOn
    const prev = storeOn
    storeOn = target
    try {
      if (target) await api.sharedStoreEnable(serverUrl)
      else await api.sharedStoreDisable()
    } catch (e) {
      storeOn = prev
      toastError(target ? "Couldn't enable the shared store" : "Couldn't disable the shared store", e)
    }
  }

  // --- Support: logs ---
  let logsPath = $state<string | null>(null)
  $effect(() => { api.logfileLocation().then((p) => { logsPath = p }).catch(() => { logsPath = null }) })
  async function openLogs() {
    try {
      const p = logsPath ?? (await api.logfileLocation())
      logsPath = p
      await api.openPath(p)
    } catch (e) {
      toastError("Couldn't open the logs folder", e)
    }
  }

  // Close on Escape / pointerdown outside (pattern AboutRepo).
  let panelEl = $state<HTMLDivElement>()
  $effect(() => {
    function onDoc(e: PointerEvent) { if (panelEl && !panelEl.contains(e.target as Node)) onclose() }
    function onKey(e: KeyboardEvent) { if (e.key === 'Escape') { e.stopPropagation(); onclose() } }
    document.addEventListener('pointerdown', onDoc)
    document.addEventListener('keydown', onKey)
    return () => { document.removeEventListener('pointerdown', onDoc); document.removeEventListener('keydown', onKey) }
  })
</script>

<div class="scrim">
  <div class="panel" bind:this={panelEl} role="dialog" aria-modal="true" aria-label="Preferences">
    <div class="title"><Icon name="settings" size={16} /> Preferences</div>

    <div class="sec">Account</div>
    <label class="field">
      <span class="lbl">Display name</span>
      <input bind:value={name} placeholder="e.g. Jimmy D." onblur={saveName}
             onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); saveName() } }} />
    </label>
    {#if email}<div class="row"><span class="lbl">Email</span><span class="val">{email}</span></div>{/if}

    <div class="sec">Appearance</div>
    <div class="row">
      <span class="lbl">Theme</span>
      <div class="seg">
        <button class:active={theme === 'dark'} onclick={() => setTheme('dark')}>Dark</button>
        <button class:active={theme === 'light'} onclick={() => setTheme('light')}>Light</button>
      </div>
    </div>

    <div class="sec">Clones</div>
    <label class="row toggle" title="A shared object store lets every clone on this machine reuse the same on-disk objects instead of each keeping a full copy — saves disk space and speeds up new clones.">
      <span class="lbl">Use shared store<span class="hint">Clones reuse one local object store — saves disk</span></span>
      <input type="checkbox" checked={storeOn === true} disabled={storeOn === null || !session.config.serverUrl} onchange={toggleStore} />
    </label>

    <div class="sec">Support</div>
    <div class="row">
      <span class="lbl">Logs<span class="hint">{logsPath ?? 'CLI log directory'}</span></span>
      <button class="ghostbtn" onclick={openLogs}>Open logs</button>
    </div>
  </div>
</div>

<style>
  .scrim { position: fixed; inset: 0; background: rgba(0, 0, 0, .35); z-index: 90; display: grid; place-items: center; }
  .panel { width: 460px; max-width: calc(100vw - 40px); background: var(--panel); border: 1px solid var(--border-strong); border-radius: 10px; box-shadow: 0 12px 30px rgba(0, 0, 0, .45); padding: 14px 16px 16px; }
  .title { display: flex; align-items: center; gap: 8px; font-size: 13px; font-weight: 500; margin-bottom: 8px; }
  .title :global(svg) { color: var(--text-muted); }
  .sec { font-size: 10px; text-transform: uppercase; letter-spacing: .04em; color: var(--text-dim); margin: 14px 0 6px; }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 5px 0; font-size: 12.5px; }
  .field { display: flex; flex-direction: column; gap: 4px; padding: 2px 0; }
  .field input { width: 100%; padding: 7px 9px; background: var(--bg); border: 1px solid var(--border); border-radius: 6px; color: var(--text); font-size: 12px; }
  .lbl { display: flex; flex-direction: column; min-width: 0; color: var(--text); }
  .hint { font-size: 10.5px; color: var(--text-dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .val { font-family: var(--font-mono); font-size: 12px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; }
  .seg { display: inline-flex; border: 1px solid var(--border); border-radius: 7px; overflow: hidden; flex: none; }
  .seg button { padding: 3px 12px; font-size: 11.5px; border: none; border-radius: 0; background: transparent; color: var(--text-muted); }
  .seg button.active { background: var(--accent); color: var(--on-accent); }
  .seg button:hover:not(.active) { background: var(--panel-hover); color: var(--text); }
  .toggle input { accent-color: var(--accent); flex: none; }
  .ghostbtn { flex: none; padding: 4px 10px; font-size: 11.5px; color: var(--text-muted); background: var(--bg); border: 1px solid var(--border); border-radius: 6px; }
  .ghostbtn:hover { color: var(--text); background: var(--panel-hover); }
</style>
```

(⚠ Vérifier : que `Icon` a un glyphe `settings` — sinon utiliser un glyphe existant approprié et le noter ; que `setDisplayName`/`setTheme` sont bien exportés par `session.svelte` ; que les variables CSS utilisées existent dans `app.css`. Reprendre la logique shared-store/theme/display-name VERBATIM de `AvatarMenu` pour ne pas diverger.)

- [ ] **Step 2: Typecheck**

Run: `npm run check` — Expected: 0 errors, 0 warnings.

- [ ] **Step 3: Commit**

```bash
git add src/lib/Preferences.svelte
git commit -m "feat(prefs): Preferences modal (account, appearance, clones, logs)"
```

---

### Task 4: Alléger `AvatarMenu.svelte` + monter le modal dans `TitleBar.svelte`

**Files:**
- Modify: `src/lib/AvatarMenu.svelte`, `src/lib/TitleBar.svelte`

- [ ] **Step 1: Alléger AvatarMenu**

Dans `src/lib/AvatarMenu.svelte` :
- RETIRER du script : l'état `name`/`saveName`, tout le bloc shared store (`storeOn`, le `$effect`, `toggleStore`), le `theme`/`setTheme`/`resolveTheme`, et les imports devenus inutiles (`setDisplayName`, `setTheme`, `resolveTheme`, `api`, `toastError` si plus utilisés — vérifier).
- RETIRER du markup : le bloc `.field` (display name), le `.storetoggle`, le `.appearance`, et leurs styles associés.
- AJOUTER une prop `onpreferences` et un bouton « Preferences… » avant Sign out.

Remplacer la déclaration des props :

```ts
  let { onclose, onpreferences }: { onclose: () => void; onpreferences: () => void } = $props()
```

Markup — le corps du menu devient (garder `.who` identité + le séparateur + Sign out ; insérer Preferences avant Sign out) :

```svelte
<div class="menu">
  <div class="who">
    <span class="ava">{initials}</span>
    <div class="ids">
      <span class="nm">{label}</span>
      <span class="em">{email ?? 'Open a repository to load your identity'}</span>
    </div>
  </div>
  <div class="div"></div>
  <button class="action" onclick={() => { onclose(); onpreferences() }}>
    <Icon name="settings" size={15} /> Preferences…
  </button>
  <button class="action out" onclick={doSignOut} disabled={!!repo.busy}>
    <Icon name="external" size={15} /> Sign out
  </button>
</div>
```

Nettoyer le `<style>` : supprimer `.field`, `.field label`, `.field input`, `.storetoggle`, `.stlabel`, `.sthint`, `.appearance`, `.aplabel`, `.seg`… (tout ce qui n'est plus référencé) ; garder `.menu`, `.who`, `.ava`, `.ids`, `.nm`, `.em`, `.div`, `.action`, `.action.out`. (⚠ `svelte-check` ne signale PAS le CSS mort, mais supprimer proprement pour l'hygiène.)

- [ ] **Step 2: Monter Preferences dans TitleBar**

Dans `src/lib/TitleBar.svelte` (READ d'abord le câblage d'`AboutRepo`/`aboutOpen` s'il existe, ou celui de l'AvatarMenu) :

```ts
  import Preferences from './Preferences.svelte'
```
```ts
  let prefsOpen = $state(false)
```

À l'usage de `<AvatarMenu … />`, ajouter la prop :

```svelte
    <AvatarMenu onclose={() => (avatarOpen = false)} onpreferences={() => (prefsOpen = true)} />
```

(⚠ adapter au nom réel de l'état qui contrôle l'ouverture de l'AvatarMenu — probablement `avatarOpen` ; vérifier.)

et monter le modal à la fin du `<header>` (comme AboutRepo) :

```svelte
  {#if prefsOpen}<Preferences onclose={() => (prefsOpen = false)} />{/if}
```

- [ ] **Step 3: Typecheck**

Run: `npm run check` — Expected: 0 errors, 0 warnings.

- [ ] **Step 4: Commit**

```bash
git add src/lib/AvatarMenu.svelte src/lib/TitleBar.svelte
git commit -m "feat(prefs): slim the account menu, open Preferences from it"
```

---

### Task 5: Vérification finale (suites + navigateur)

**Files:** aucun nouveau.

- [ ] **Step 1: Suites**

```powershell
npx vitest run
cargo test --manifest-path src-tauri/Cargo.toml --lib
npm run check
```

Expected : vitest tous passed (les tests existants ne bougent pas — pas de nouveau test front, la logique relocalisée est inchangée) ; cargo `ok` (nouveau `logfile_tests`) ; svelte-check 0/0.

- [ ] **Step 2: Parcours navigateur mock**

`npm run dev` → http://localhost:5173. Menu compte (avatar) : ne contient PLUS que identité + **« Preferences… »** + Sign out (les toggles ont disparu). Cliquer « Preferences… » → modal avec 4 sections : **Account** (éditer le display name → blur → persiste ; email affiché), **Appearance** (bascule Dark/Light → le thème change en direct, segment actif suit), **Clones** (toggle shared store → coché/décoché, persiste), **Support** (« Open logs » → pas d'erreur console ; le chemin de logs mock s'affiche en petit). Escape ferme ; clic hors panneau ferme. Console sans erreur.

- [ ] **Step 3: Marquer le plan exécuté**

Ajouter un bloc « STATUT : EXÉCUTÉ ET VÉRIFIÉ le <date> » (compte de tests, points navigateur). Commit :

```bash
git add docs/superpowers/plans/2026-07-12-lore-desktop-preferences-and-logs.md
git commit -m "docs: mark preferences + logs plan executed and verified"
```

---

## Self-review (fait à l'écriture du plan)

- **Couverture design** : « Open logs » → Task 1 (runner brut + parser pinné sur la capture + commande) + Task 2 (API) + Task 3 (bouton Support). Écran Préférences → Task 3 (modal 4 sections, logique relocalisée verbatim). Menu compte épuré + ouverture → Task 4. « Tout dans Préférences » (décision Jimmy) : les 3 réglages sont RETIRÉS de l'AvatarMenu (Task 4) et présents UNIQUEMENT dans le modal (Task 3). Vérif → Task 5.
- **Placeholders** : aucun TBD/TODO ; chaque step porte son code ; les seuls « vérifier » sont des points d'intégration bornés (glyphe `settings`, nom de l'état d'ouverture de l'AvatarMenu, imports devenus inutiles à retirer, variables CSS).
- **Cohérence de types** : `logfile_location_from(&str)->Option<String>` (Task 1) ↔ `lore_logfile_location()->Result<String,String>` ↔ `logfileLocation():Promise<string>` (Task 2) ↔ `api.logfileLocation()` (Task 3) ; `Preferences` prend `{onclose}` (Task 3) monté par TitleBar (Task 4) ; `AvatarMenu` prend `{onclose,onpreferences}` (Task 4). Aucune régression de la logique relocalisée (mêmes appels `setDisplayName`/`setTheme`/`sharedStore*`).

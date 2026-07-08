# Lore Desktop — Persistent Repo Switcher (GitHub-Desktop-style)

**Date:** 2026-07-08
**Status:** Design — not yet implemented

## Goal

Clicking **Current repository** opens a dropdown that lists the repositories you've
already opened or cloned (persisted locally), so you can switch between them — or add
a new one — **without ever losing the current repository**. This mirrors GitHub
Desktop's repository switcher.

## Problem today

- The TitleBar **Current repository** button calls `clearCurrentRepo`, which sets
  `config.currentRepo = null`. `App.svelte` then renders the full-screen `RepoPicker`
  (clone / open-folder). So a single click **drops the current repo and lands on the
  clone/open page** — the exact frustration reported.
- `AppConfig` already persists `currentRepo` + `recentRepos: string[]` to
  `config.json`, but **`recentRepos` is never populated or shown** — there is no
  known-repo list to switch between.

## Visual reference (GitHub Desktop, provided screenshots)

1. **Repo dropdown** — opened from "Current repository": a search/filter field, an
   **Add ▾** button (top-right), then a scrollable list of repositories. Each row =
   an icon (folder / lock for private) + the repo name; the currently-open one is
   highlighted. (GitHub Desktop groups by owner; see *Grouping* below.)
2. **Add ▾ menu** — three items: *Clone repository…*, *Create new repository…*,
   *Add existing repository…*. **We include only Clone + Add existing — NOT Create.**
3. **Clone dialog** — a modal to pick a server repo + a local destination path.

## Design

### Persistence

- `config.json` `recentRepos` becomes the **known-repos list** — an ordered list of
  absolute repo paths (most-recently-used first). The infra already exists
  (`config.rs` `AppConfigDto.recent_repos`, `loadConfig`/`saveConfig`).
- A repo is **added** to `recentRepos` when it is opened (Add existing) or cloned.
- **Switching** to a repo sets `config.currentRepo` and moves that path to the front
  of `recentRepos`, then persists (`saveConfig`).
- The display **name** is the folder basename (`path.split(/[\\/]/).pop()`). No extra
  metadata is stored — paths only.

### UI

- **TitleBar** "Current repository ▾" button opens a **`RepoSwitcher.svelte`
  dropdown** (modeled on `BranchMenu.svelte`) instead of calling `clearCurrentRepo`.
- **`RepoSwitcher.svelte`** contains, top to bottom:
  - A **Filter** input (live-filters the list by name/path).
  - An **Add ▾** button whose menu has exactly two items:
    - **Clone repository…** → the existing clone flow (list the server's repos, pick
      one + a destination) → on success, add the path to `recentRepos` and switch to it.
    - **Add existing repository…** → the native folder picker (`pickFolder`), validated
      by `getStatus` (a Lore working copy is a dir containing `.lore/`) → add + switch.
  - The **repo list**: one row per `recentRepos` entry — folder icon + name, with the
    dir path as a dim subtitle; the current repo is highlighted. Clicking a row
    switches to it and closes the dropdown. Virtualized only if the list grows large
    (unlikely for repos).
- **First run** (empty `recentRepos`) still shows the full-screen `RepoPicker` so the
  user has a clear place to clone/open their first repo. Once at least one repo is
  known, the switcher dropdown is the entry point and the full-screen picker is not
  used for everyday switching.
- A repo can be **removed from the list** via a row action / right-click "Remove"
  (drops it from `recentRepos`; does **not** delete files on disk).

### Grouping

Lore repositories are plain local folders with no owner/org metadata, so the list is
**flat**, most-recently-used first (a small "Recent" affordance is implicit in the
ordering). Owner/server grouping like GitHub Desktop's is not applicable here and is
out of scope.

## Components & data flow

- `RepoSwitcher.svelte` (new) — the dropdown; reads `config.recentRepos`, calls a
  shared `switchRepo(path)` / `addRepo(path)` in `session.svelte.ts` (persist +
  reorder), and reuses the clone + `pickFolder` logic currently in `RepoPicker.svelte`
  (extract those into shared helpers so both the first-run picker and the switcher use
  them).
- `session.svelte.ts` — `switchRepo(path)` (set `currentRepo`, move to front of
  `recentRepos`, `saveConfig`), `addRepo(path)` (prepend if new, then switch),
  `removeRepo(path)`.
- `TitleBar.svelte` — the "Current repository" button toggles the switcher (remove the
  `clearCurrentRepo` behavior).
- `App.svelte` — unchanged branching: full-screen `RepoPicker` only when
  `recentRepos` is empty / no `currentRepo`.

## Out of scope

- **Create new repository** (deliberately excluded from the Add menu).
- Owner/server grouping of the list.
- Any change to the underlying clone / open-folder mechanics (reused as-is).

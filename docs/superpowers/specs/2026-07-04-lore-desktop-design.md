# Lore Desktop — design

- **Date:** 2026-07-04
- **Repo:** github.com/KiraFR/lore-desktop (standalone product)
- **Tracking:** TICKET-127
- **Adapts** the original internal design (`SoonerOrLater/Web/docs/superpowers/specs/2026-06-24-lore-desktop-app-design.md`) into a standalone, SSO-agnostic product.

## Goal

A standalone desktop client for the **Lore VCS**, in the spirit of GitHub Desktop, that **any team can point at their own Lore server + SSO**. It provides the collaboration-safety and power-user features a lightweight in-Editor plugin lacks: selective commits, branches + directional merge, history with ticket links, **Perforce-style file locks**, and **binary-aware conflict resolution**.

## Why standalone + generic

- **Not tied to one org:** the SSO endpoint and Lore server are entered at sign-in, so anyone can use it with their own backend — our ticket-app IdP is just one option.
- **Complements in-Editor plugins:** the community SimpleLore/LoreQuickCommit plugin (verified on its source) only does whole-workspace `lore stage . --scan` + commit + push and a **destructive `lore sync --reset`** pull — no locks, branches, history, selective staging, or conflict resolution. This app is the real client for those (the team-collaboration-safety layer).
- **Fills the gap until 2027:** Epic's official Unreal-Editor plugin + official desktop + web clients are on the Lore roadmap for ~2027 (in progress); this ships now.

## Architecture

- **Shell:** Tauri v2 (Rust backend + webview UI). Scaffold committed (`a633b55`).
- **Frontend:** Svelte + Vite + TypeScript.
- **Lore integration:** the Rust backend **shells out to a bundled `lore` CLI** and parses its `--json` output; the app ships a pinned `lore` binary. (Same pattern as GitHub Desktop wrapping `git`; the CLI surface + `--json` were validated during the SSO work.)
- **Auth token:** stored in the OS keychain (the `lore` CLI already uses the `keyring` crate for this).

## Generic SSO (the login)

- The signed-out screen asks for the **Lore server URL** (e.g. `lore://host:port`).
- The app runs `lore auth login --auth-url <auth_url> lore://<host:port>`. The `auth_url` (the SSO/IdP endpoint) is **discovered from the loreserver** — it advertises `[environment.endpoint] auth_url` — with a **manual override** field for setups that need it.
- The `lore` CLI drives the browser sign-in (StartAuthSession → `login_url` → GetAuthSession polling) against whatever IdP the `auth_url` points to — our ticket-app SSO, or any compatible one. On success the token lands in the OS keychain and authenticates every later `lore` call.
- Net: **IdP-agnostic** — enter your server, sign in through your SSO, done.

## Scope — sliced

**Slice 1 (first buildable loop):**
- Sign-in: generic SSO (configurable server URL, browser login via the CLI, keychain).
- Repository picker / working directory selection.
- Changes view: list changed files with status; commit-message box; commit; push.
- Status (ahead/behind / synced) + sync (pull).

**Slice 2+ (later):** history (+ ticket links), branches (list/switch/create), directional source→target merge with binary-aware conflict resolution (mine/theirs), **file locks** (the Perforce-style differentiator), binary before/after compare, live refresh from `lore notification subscribe`.

## UI

Per the approved mockups: title bar (repo · branch · sync · push · account), Changes tab with lock chips + binary before/after, History, branch switcher + create dialog, directional merge dialog + conflict view. **Slice 1 = the title bar + sign-in + Changes/commit/push/status.**

## Distribution

- Platforms: Windows first, macOS supported (Tauri per-OS bundles). The pinned `lore` CLI ships inside the bundle.
- Own git repo (github.com/KiraFR/lore-desktop); CI + signed releases later.

## Open decisions (resolve at plan time)

- Exact `lore` CLI subcommands + `--json` shapes for status and the changed-files list (confirm against the CLI / the SimpleLore reference which showed `lore status --scan`, `lore status --revision-only`, `lore stage . --scan`, `lore commit`, `lore push`, `lore sync`).
- Repo picker: how the app selects and remembers the working directory it runs `lore` in.
- Keychain: the `lore` CLI stores the credential; the app triggers `lore auth login` and relies on the stored credential for subsequent calls.

## Status

Scaffold committed + pushed (`a633b55`). Next: **writing-plans for Slice 1 → subagent-driven build.**

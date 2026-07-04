# Lore Desktop

A GitHub-Desktop-style client for the [Lore](https://github.com/EpicGames/lore) VCS — sign in to any Lore server + SSO, pick a repository, review changes, commit, push, and sync. Built with **Tauri v2 + Svelte 5 + TypeScript**.

## Status: Slice 1 — UI with mock data

This slice is the **UI, driven entirely by an in-memory mock** so the design can be iterated in the browser with zero backend. There is no `lore` CLI and no Tauri command wiring yet — every data call goes through `src/lib/api.ts`, which currently re-exports a stateful fake (`src/lib/mock.ts`, backed by `localStorage`).

The mock implements the exact `LoreApi` TypeScript contract (`src/lib/types.ts`) that the real backend will implement later, so the **wiring slice** is a drop-in swap of `src/lib/api.ts`'s internals (mock → `@tauri-apps/api` `invoke`) with the components untouched.

What's here: generic sign-in, a repository picker, and a Changes view (file list with add/modify/delete tags + binary hints, commit composer, push/sync, ahead/behind), across signed-out / loading / empty / busy / error states, light + dark.

## Develop

```bash
npm install
npm run dev      # design loop — opens http://localhost:5173 in the browser (no Tauri build)
npm test         # vitest — mock/contract tests
npm run check    # svelte-check + tsc
npm run build    # production web build
```

`npm run dev` is the design-iteration server: edit any `.svelte` file or `src/app.css` and see it live. The design system lives in `src/app.css` (CSS variables for colors/spacing/radius, light + dark).

> Running inside the Tauri shell (`npm run tauri dev`) also works but is unnecessary for design and requires the Rust toolchain.

## Architecture

- `src/lib/types.ts` — the `LoreApi` contract + data types (`StatusResult`, `ChangedFile`, `RepoEntry`, `AppConfig`).
- `src/lib/mock.ts` — the stateful mock (fake repos + per-repo change sets; commit clears files, push zeroes "ahead", etc.).
- `src/lib/api.ts` — the app's single data boundary. **Swap this for the real backend later.**
- `src/lib/session.svelte.ts` — reactive app state (Svelte 5 rune store).
- `src/lib/*.svelte` — `SignIn`, `TitleBar`, `RepoPicker`, `Changes`, `StatusPill`; wired by `src/App.svelte`.

## Next slice (deferred)

Wire the real `lore` CLI: add `src-tauri` Tauri commands that shell `lore … --json` (NDJSON) and parse it, then replace `src/lib/api.ts` internals with `invoke` calls of the same `LoreApi` signatures. Add `@tauri-apps/plugin-dialog` for the native folder picker, real `lore auth login` (browser + keychain), and map `lore status`'s `action:"keep"` → `"modify"`.

# Lore Desktop — visual design (dark theme + themeable)

- **Date:** 2026-07-05
- **Repo:** github.com/KiraFR/lore-desktop
- **Supersedes the visual choices** in `2026-07-04-lore-desktop-design.md` (functional design still applies). This doc defines the *look*.

## Direction

GitHub Desktop's **structure and UX** (3-zone toolbar, two-pane, Changes/History tabs, commit box), re-skinned in the studio's **dark theme with a blue accent** — matching the mockups approved in an earlier session (dark Changes view with binary before/after compare, file locks, ticket links, History, branch create, directional merge with binary conflict resolution). Not a GitHub Desktop clone — same bones, own skin.

**Standalone / SSO-agnostic** (decided this session): the sign-in takes a **Lore server URL** and drives that server's SSO — it is **not** branded "Sign in with Sooner or Later". Same dark polish, generic content.

## Themeable design system (the core requirement)

Every color is a **CSS custom property** (design token). Dark is the default and the only theme shipped now, but the system is built so **adding a theme later = one token block + a picker**, with zero component changes.

- Tokens live in `src/app.css` on `:root` (the dark defaults). A future theme is `:root[data-theme="light"] { … }` (only the tokens it overrides), selected by setting `document.documentElement.dataset.theme` from persisted config. No `data-theme` set ⇒ dark.
- Components reference **only tokens** (`var(--accent)`, never a raw hex). This is the invariant that makes theming free.
- A theme picker UI + persistence is a **later slice** — this slice just lays the token structure and ships dark.

**Dark token set** (default `:root`):

```
--bg:            #1b1d21   /* app background */
--bg-elev:       #212429   /* toolbar, status bar, elevated bars */
--panel:         #24272c   /* inputs, cards, selected rows, commit box */
--panel-hover:   #2a2e34
--border:        #33373d   /* hairlines */
--border-strong: #3d424a
--text:          #e6e8eb   /* primary */
--text-muted:    #9198a1   /* secondary / paths / labels */
--text-dim:      #767c85   /* hints */
--accent:        #3067d4   /* primary buttons, links, active tab, focus */
--accent-hover:  #3a72e0
--accent-soft:   #14304d   /* accent-tinted fill: icon circle, lock "you" pill, selected row */
--on-accent:     #ffffff
--added:         #3fb950   /* + / added */
--modified:      #d29922   /* ~ / modified */
--deleted:       #f85149   /* − / deleted */
--warn-bg:       #3d2f0f   /* amber badge/card bg (Modified, Conflicts) */
--warn-text:     #e3b341
--radius:        7px
--radius-lg:     10px
--font-sans: system-ui, -apple-system, "Segoe UI", sans-serif
--font-mono: ui-monospace, "Cascadia Code", monospace
```

## Screens

Aesthetic per the approved mockups (dark). Layout/structure carries from the existing functional design.

- **Sign-in (standalone):** centered — book icon in an `--accent-soft` circle, "Welcome to Lore Desktop", one-line subtitle, a **Lore server URL** field, an "Advanced" disclosure (auth-URL override), a primary **Sign in** button with an external-link icon, and the hint "Signs in through your server's SSO · opens your browser".
- **Title bar:** current repository (folder icon + name + ▾) · current branch (branch icon + name + ▾) · right side: Sync (with count), Push (accent button, with count), user avatar (initials in an `--accent-soft` circle).
- **Changes view:** Changes/History tab bar (active tab underlined `--accent`); file list rows = checkbox + status glyph (`+`/`~`·M/`−` in `--added`/`--modified`/`--deleted`) + path (filename `--text`, directory `--text-muted`) + optional lock chip (`🔒 you`); commit box = summary input, description textarea, ticket-link hint ("TICKET-NNN will be linked"), a primary "Commit to <branch>" button. Right pane = the file preview (**later slice**: binary before/after compare + `Modified` badge + Type/Size/Lock).
- **Status bar (bottom):** sync state ("Synced 2 min ago") on the left; lock summary ("1 lock held by you · 2 by teammates") on the right.
- **History, branch picker/create, directional merge + binary conflict resolution:** **later slices**, styled per the mockups.

## Re-skin scope (this slice)

Apply the dark design system to the **already-built Slice-1 UI** without adding new features:

1. Rewrite `src/app.css` to the themeable token structure + dark values above; add the `data-theme` hook (default dark).
2. Restyle `SignIn.svelte` (welcome layout, book icon, dark field), `TitleBar.svelte` (repo·branch·sync·push·avatar, dark), `RepoPicker.svelte`, `StatusPill.svelte`, and `Changes.svelte` (dark toolbar + file list + commit box) to reference tokens only.
3. Add a bottom **status bar** to the signed-in shell (sync state; lock summary is placeholder text until the locks slice).
4. Keep the current data/behavior (mock; commit-all; no diff pane, no locks, no history/merge). The two-pane binary-compare right pane, per-file staging, locks, history, and merge are their own later slices.

Out of scope now: theme picker UI, light theme values, the rich right pane, locks, history, branches, merge.

## Status

Direction approved (dark + blue accent + standalone sign-in + themeable). Next: writing-plans for the re-skin, then subagent-driven build.

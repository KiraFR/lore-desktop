# Lore Desktop — Previews d'assets (paliers 1–2)

**Date :** 2026-07-09
**Statut :** Design validé — à implémenter
**Référence visuelle :** Anchorpoint (vignettes d'assets + panneau de preview). Choix utilisateur : vignettes dans le panneau FilePreview uniquement (les listes attendront le palier 3), PSD inclus (cas simples), lecteur audio inclus.

## Objectif

Quand un artiste sélectionne un fichier changé, il voit **le vrai contenu** de son working copy : vignette image pour les textures/rendus (`png, jpg, jpeg, webp, bmp, gif, tga, tif, tiff, dds, exr, hdr, psd`), lecteur pour l'audio (`wav, ogg, mp3, flac`). Les dimensions réelles de l'image rejoignent la table de méta.

**Limite assumée :** la boîte « Before · previous revision » garde son placeholder — le CLI n'a pas de `lore file cat <revision>` (manque déjà tracé dans l'audit, à remonter à l'équipe Lore). Les previews sont donc working-copy seulement ; un fichier `delete` n'a pas de preview.

## Architecture

### Pipeline de vignettes (Rust, un seul chemin pour toutes les images)

Nouveau module `src-tauri/src/preview.rs`, commande :

```
lore_preview(app, repo_path, path, max_px) -> PreviewDto
PreviewDto { kind: "image" | "audio" | "none", data_url: Option<String>, width: Option<u32>, height: Option<u32> }
```

1. **Classification par extension** (réutilise `ext_of` de `commands.rs`, rendu `pub(crate)`) : `AUDIO_EXTS` → kind `audio` (pas de décodage ; le front construit l'URL de streaming) ; `PREVIEW_IMAGE_EXTS` → pipeline image ; sinon `none`.
2. **Cache disque** : `app_cache_dir()/thumbs/<hash>.png`, clé = hash de `(chemin absolu, mtime, taille, max_px)` (hasher std `DefaultHasher`, format hex). Hit → data-URL immédiat sans décodage.
3. **Décodage** :
   - `dds` : `ddsfile` (header) + `texture2ddecoder` (BC1–BC7, etc.) → RGBA8 ;
   - `psd` : crate `psd` (RGBA aplatie ; les modes non gérés → erreur → `none`) ;
   - `exr`/`hdr` : crate `image` (features par défaut) → f32 → tone-mapping Reinhard `x/(1+x)` + gamma 2.2 → RGBA8 ;
   - le reste : `image::open`.
4. **Vignette** : `thumbnail(max_px, max_px)` (ratio préservé, max 512 côté appelant) → PNG encodé → écrit au cache → `data:image/png;base64,…`. Dimensions **source** retournées.
5. Tout dans `blocking()` ; toute erreur de décodage → `Ok(kind: "none")` (jamais de toast pour une preview — l'icône générique reste).

Dépendances Cargo ajoutées : `image = "0.25"`, `ddsfile = "0.5"`, `texture2ddecoder = "0.1"`, `psd = "0.3"`, `base64 = "0.22"`.

### Audio en streaming (asset protocol)

- `Cargo.toml` : `tauri = { …, features = ["protocol-asset"] }`.
- `tauri.conf.json` : `app.security.assetProtocol = { "enable": true, "scope": [] }` (scope statique vide — tout passe par le scope dynamique).
- Dans `lore_preview`, quand kind = `audio` : `app.asset_protocol_scope().allow_file(&abs)` avant de répondre — le front peut alors streamer via `convertFileSrc`.

### API front

```ts
export interface PreviewData {
  kind: 'image' | 'audio' | 'none'
  /** image : data-URL PNG de la vignette ; audio : URL streamable ; none : null */
  url: string | null
  width?: number
  height?: number
}
LoreApi.getPreview(repoPath: string, path: string): Promise<PreviewData>
```

- **tauri.ts** : `invoke('lore_preview', { repoPath, path, maxPx: 512 })` ; si `kind === 'audio'`, `url = convertFileSrc(repoPath + '/' + path)` (import `convertFileSrc` de `@tauri-apps/api/core`).
- **mock.ts** : images → SVG data-URL généré (damier + nom du fichier, dims factices 2048×2048) ; audio → data-URL d'un mini WAV silencieux (constante ~300 octets) pour que le lecteur se rende ; sinon `none`. Aucun réseau.

### FilePreview.svelte

- Effet sur la sélection d'un fichier **binaire** non-`delete` : `api.getPreview(...)` (état `previewLoading`, résultat `preview`). Même pattern anti-course que le diff (vérifier `file?.path` avant d'assigner).
- Rendu :
  - `kind image` : la boîte « After · working copy » affiche `<img src={preview.url}>` (object-fit contain, fond damier CSS léger) ; « Before » inchangé ;
  - `kind audio` : une seule boîte avec `<audio controls src={preview.url}>` ;
  - `none` / erreur / chargement : placeholders actuels.
- Table de méta : ligne « Dimensions » (`{width} × {height}`) quand connues.
- La note « Binary asset — visual compare, no text diff. » devient « Before » spécifique : « Previous revision preview needs server support — working copy only. » quand une vignette After est affichée.

### Retouche connexe

`gif` rejoint `BINARY_EXTS` dans `commands.rs` (aujourd'hui un .gif modifié tenterait un diff texte) — il est déjà couvert par le pipeline image.

## Tests

- **Rust** (`preview.rs`) : PNG et TGA synthétiques générés par le crate `image` dans un dossier temp → vignette produite (signature PNG, dims sources correctes, ≤ max_px) ; extension inconnue → `none` ; extension audio → `audio` sans data ; second appel → hit cache (le fichier cache existe et le résultat est identique).
- **TS** : aucun nouveau module pur nécessaire ; parité mock vérifiée par le passage navigateur.
- **Vérification manuelle** : mock (vignette SVG + lecteur wav visibles) puis app réelle sur `lore-test-repo` avec un PNG et un WAV générés, vignette et lecture audio constatées, cache vérifié (2e sélection instantanée).

## Hors périmètre (paliers suivants)

Mini-vignettes dans les listes (palier 3, avec le cache posé ici), viewer 3D turntable (gltf/fbx/obj, spine), vignettes embarquées `.blend`/`.uasset` (palier 4, plutôt via le plugin UE), preview « Before » (bloqué CLI).

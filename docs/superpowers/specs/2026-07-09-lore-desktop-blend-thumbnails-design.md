# Lore Desktop — Vignettes embarquées des .blend (palier 4, partie autonome)

**Date :** 2026-07-09
**Statut :** Design — item n°3 du programme. Les `.uasset` et Spine restent hors périmètre (couplés au futur plugin UE).

## Objectif

Un `.blend` sélectionné (ou dans une liste) montre la vignette que Blender **embarque dans le fichier** au save — sans Blender installé, sans décodage de scène.

## Format (connu, stable)

- En-tête 12 octets : `BLENDER` + taille de pointeur (`_`=4, `-`=8) + endianness (`v` little, `V` big) + version.
- Fichier éventuellement compressé : magic **zstd** `28 B5 2F FD` (Blender ≥ 3.0) ou **gzip** `1F 8B` (anciens) — décompresser d'abord.
- Flux de blocs : code 4 o, taille u32, pointeur (4/8 o), index SDNA u32, count u32, puis les données. Fin au bloc `ENDB`.
- La vignette vit dans le bloc **`TEST`** : `i32 width, i32 height`, puis `w×h×4` octets RGBA, stockés **bottom-up** (flip vertical requis).

## Implémentation

- `preview.rs` : `decode_blend(path)` = lire → décompresser si magic zstd/gzip (crates `zstd`, `flate2`) → `blend_thumbnail(&data)` (parseur de blocs pur, testable) → RGBA flippée. Branché dans `decode()` ; `"blend"` rejoint `IMAGE_EXTS` du pipeline (cache/thumbs/dimensions gratuits). Un `.blend` sans bloc `TEST` (save sans vignette) ⇒ erreur ⇒ `none` (icône générique, comme tout échec de décodage).
- Front : `"blend"` rejoint la regex de `previewKind.ts` (vignettes de listes) et celle du mock.
- Tests Rust : `.blend` synthétiques construits dans le test — non compressé et zstd — avec un bloc TEST 2×2 : dimensions, flip vertical vérifié pixel à pixel ; fichier sans TEST ⇒ None ; en-tête invalide ⇒ None.

## Hors périmètre

`.uasset` (thumbnail cache UE — via le plugin UE), Spine, génération de vignette quand le `.blend` n'en embarque pas.

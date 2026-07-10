# Lore Desktop — Vignettes Substance (.sbsar embarquée, .spp opportuniste)

**Date :** 2026-07-10
**Statut :** Design validé (« ok vasy ») — enquête de faisabilité du 2026-07-09

## Constats (sources dans l'audit de faisabilité)

- **`.sbsar`** = archive **7-zip** contenant le graphe compilé et, dans la plupart des fichiers publiés, une icône PNG (`assemblies/content/…/icon*.png` ; l'API officielle PySBS a `addIcon`). Extraction directe possible, sans moteur Substance.
- **`.spp`** = conteneur **HDF5** ; aucune vignette documentée, Anchorpoint ne le supporte pas nativement. Tentative opportuniste : scan borné de magics PNG/JPEG (même approche que le fallback `.uasset`) — si Painter stocke une préview en blob non compressé, on la trouve ; sinon `none` propre.
- **`.sbs`** = XML procédural, rien d'affichable sans moteur — hors périmètre (voie future : déléguer à `sbsrender` si le Substance Automation Toolkit est détecté).

## Implémentation (`preview.rs`)

1. **`decode_sbsar(path)`** : crate 7z pur-Rust (`sevenz-rust`), itérer les entrées, choisir en priorité un nom contenant `icon`/`thumbnail` et finissant en `.png` (sinon premier `.png`), extraire en mémoire (cap 10 Mo) → `image::load_from_memory`. Branché dans `decode()` ; `"sbsar"` rejoint `IMAGE_EXTS`.
2. **`decode_spp(path)`** : vérifier le magic HDF5 (`\x89HDF\r\n\x1a\n`), puis réutiliser le scan borné d'images embarquées du fallback `.uasset` (factorisé en helper partagé si besoin), candidats acceptés si dims ≤ 2048. `"spp"` rejoint `IMAGE_EXTS`.
3. `previewKind.ts` : regex + `sbsar|spp` (vignettes de listes ; le mock suit automatiquement).

## Tests

- Synthétiques committés : archive 7z construite en test (writer `sevenz-rust`) avec `assemblies/content/0000/icon1.png` (mini-PNG généré) → image ; 7z sans PNG → none ; fichier HDF5-magic + PNG embarqué → image ; HDF5-magic seul → none.
- **Vérification réelle en attente** : aucun `.sbsar`/`.spp` sur la machine — à valider dès qu'un fichier d'exemple est fourni (noté comme dette de vérification).

# Lore Desktop — Previews palier 3 : vignettes de listes + viewer 3D

**Date :** 2026-07-09
**Statut :** Design — découle du spec previews (2026-07-09), palier 3 annoncé et validé
**Référence :** Anchorpoint (vignettes par ligne + panneau modèle 3D)

## A. Mini-vignettes dans les listes

Chaque ligne de fichier *image prévisualisable* affiche une vignette 20 px (coin arrondi) devant le nom, dans **Changes**, le **détail de commit** (History) et **Locks**. Les autres fichiers gardent leur rendu actuel.

- **API** : `LoreApi.getPreview(repoPath, path, maxPx?)` gagne un paramètre optionnel (défaut 512). Les listes demandent `maxPx = 64` — le pipeline Rust et son cache disque sont déjà paramétrés par taille.
- **Store `thumbs.svelte.ts`** : `SvelteMap<path, string | null>` (`null` = pas de vignette), file d'attente à **concurrence 4** pour ne pas mitrailler le CLI/disque, `requestThumb(path)` idempotent, `clearThumbs()` appelé par `refreshStatus` (le cache disque reste, seule la carte mémoire se vide — une vignette d'un fichier modifié se régénère via la clé mtime).
- **`previewKind.ts`** (pur, testé) : `isPreviewableImage(path)` — la regex partagée par le store et le mock.
- Fichiers `delete` : pas de requête (fichier absent).

## B. Viewer 3D turntable

Pour `glb, gltf, obj, fbx` : le panneau FilePreview remplace les boîtes Before/After par un **canvas three.js** (~340 px) : auto-rotation douce qui s'arrête à la première interaction, orbit à la souris, zoom molette, éclairage studio neutre (hémisphérique + directionnel), grille discrète, cadrage automatique sur la bounding box.

- **Rust** : ces extensions deviennent `kind: "model"` dans `lore_preview` — pas de décodage ; `allow_file` + `allow_directory(parent)` sur l'asset protocol (un `.gltf` référence des `.bin`/textures à côté, un `.fbx` ses textures). Le front streame via `convertFileSrc`.
- **Front** : dépendances `three` + `@types/three`. `ModelViewer.svelte` (props `url`, `name`) : loader choisi par l'extension du nom (`GLTFLoader`/`OBJLoader`/`FBXLoader` de `three/examples/jsm`), normalisation (centrage + distance caméra = 1,8 × la plus grande dimension), `ResizeObserver`, disposal complet au démontage (renderer, controls, géométries). Échec de chargement → retombe sur les placeholders actuels, jamais de toast.
- Textures externes manquantes (FBX) : le modèle s'affiche avec ses matériaux sans maps — assumé.
- `PreviewData.kind` gagne `'model'`.
- **Mock** : `getPreview` renvoie `kind 'model'` pour ces extensions ; l'URL n'est fournie que pour `.obj` (un cube OBJ généré en data-URL — format texte) pour que le viewer soit exerçable en dev ; `fbx/glb` mock → `url: null` (fallback placeholders). Un `Content/Props/SM_Crate.obj` rejoint les fichiers seed.

## Hors périmètre

Spine (runtime dédié + fichiers couplés atlas/json — palier 4 avec .blend/.uasset), Alembic (`abc`, pas de loader three.js standard), vignettes 3D dans les listes (les modèles gardent leur ligne sans vignette).

## Tests

- `previewKind.test.ts` : classification par extension.
- Vitest existants intacts ; Rust : test du kind `model` (pas de data, pas de décodage).
- Manuel mock : vignettes dans les 3 listes, cube OBJ en turntable.
- Manuel app réelle : vignettes sur `T_Gradient.png`, cube `.obj` généré dans `lore-test-repo` en turntable.

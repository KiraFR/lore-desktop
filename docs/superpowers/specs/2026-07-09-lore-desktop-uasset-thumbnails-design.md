# Lore Desktop — Vignettes embarquées des .uasset

**Date :** 2026-07-09
**Statut :** Design — complète le palier 4 (les `.blend` sont faits ; Spine reste hors périmètre)

## Objectif

Un `.uasset`/`.umap` sélectionné (ou en ligne de liste) montre la vignette que l'éditeur Unreal serialise dans le package au save — sans Unreal installé côté lecteur, en pur Rust. (Le futur plugin UE restera la voie « propre » pour générer/rafraîchir des vignettes, mais l'extraction couvre déjà tous les assets sauvegardés par l'éditeur.)

## Format (FPackageFileSummary → thumbnail table → FObjectThumbnail)

- **En-tête** : tag `0x9E2A83C1` (LE), `LegacyFileVersion` (i32 négatif, -6…-9), versions UE4/UE5, tableau CustomVersions (count + 20 o/entrée), `TotalHeaderSize`, `FolderName` (FString), `PackageFlags`, Name/Export/Import counts+offsets…, puis **`ThumbnailTableOffset` (i32)**. Les positions dépendent de la version — parser de façon défensive (gates par version, support UE5.x en priorité, y compris paquets « unversioned » qui serialisent comme la dernière version).
- **Table** : à `ThumbnailTableOffset` : `i32 count`, puis par entrée : ClassName (FString), ObjectPathWithoutPackageName (FString), `i32 FileOffset`.
- **FObjectThumbnail** à chaque FileOffset : `i32 ImageWidth`, `i32 ImageHeight` (une valeur négative encode une variante de compression — ignorer le signe), `i32 DataSize`, puis les octets compressés : **PNG ou JPEG** — sniffer le magic (`89 50 4E 47` / `FF D8 FF`) et décoder via le crate `image` (`load_from_memory`).
- FString : `i32 len` ; len ≥ 0 ⇒ UTF-8/ANSI de `len` octets (dont NUL final) ; len < 0 ⇒ UTF-16LE de `-len` unités.

## Stratégie d'implémentation (`preview.rs`)

1. `decode_uasset(path)` : lecture **seekée** (pas tout le fichier — une texture éditeur peut peser des centaines de Mo) : en-tête (premiers ~64 Ko) → `ThumbnailTableOffset` → seek table → seek premier FObjectThumbnail exploitable → payload → `image::load_from_memory`.
2. **Fallback tolérant aux versions** : si le parse structuré échoue (offsets exotiques, version non gérée), scan borné du fichier (chunks avec chevauchement, plafond 64 Mo) à la recherche de magics PNG/JPEG ; chaque hit est tenté au décodage et accepté si dims ≤ 1024 (les vignettes UE font ≤ 256) ; premier succès gagne.
3. Absence de vignette (asset jamais sauvé par l'éditeur, projet avec vignettes désactivées, fichier cooked) ⇒ `none` (icône générique, jamais de toast) — comportement standard du pipeline.
4. `uasset` + `umap` rejoignent `IMAGE_EXTS` (cache/thumbs/dimensions gratuits) et la regex de `previewKind.ts` (vignettes de listes + mock).

## Vérification

- Tests Rust synthétiques committés : FString (les deux encodages), FObjectThumbnail parsé depuis des octets construits, fallback scan sur un fichier garbage+PNG, fichier sans vignette ⇒ none.
- **Vérification réelle** (non committée) : le projet UE local `C:\Users\jimmy\Documents\SoonerOrLater\Games\FirstPerson\Content` contient 500+ `.uasset` réels (UE 5.7) — l'extraction doit réussir sur des textures/BP types (ex. `Characters\Mannequins\Textures\Manny\T_Manny_02_BN.uasset`) en LECTURE SEULE, et échouer proprement (none) sur les assets sans vignette.

## Hors périmètre

Spine ; génération de vignettes manquantes (plugin UE) ; `.uexp/.ubulk` cooked.

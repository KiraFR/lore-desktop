# Lore Desktop — Lecteur audio (carte waveform)

**Date :** 2026-07-09
**Statut :** Design validé — à implémenter
**Choix utilisateur :** variante A (carte waveform, fallback barre), contrôles loop + volume (pas de vitesse), pas d'autoplay.

## Objectif

Remplacer le `<audio controls>` natif de FilePreview par un lecteur intégré au thème, pensé sound designer : forme d'onde cliquable en deux tons, temps aux centisecondes, boucle, volume mémorisé.

## Composant

`src/lib/AudioPlayer.svelte` — réutilisable (le palier 3 « vignettes dans les listes » pourra pointer dessus), props :

```ts
let { src, name }: { src: string; name: string } = $props()
```

### Lecture

- Un `<audio>` **caché** créé par le composant : le streaming reste au navigateur/asset-protocol, aucun décodage nécessaire pour jouer.
- État réactif via les événements `loadedmetadata` (durée), `timeupdate` (position), `play`/`pause`/`ended`.
- **Pas d'autoplay.** Lecture stoppée (pause + reset) quand `src` change ou que le composant est démonté.
- **Loop** : toggle (pill icône répétition), mappe `audio.loop`. État local, défaut off.
- **Volume** : slider compact (70 px), persisté dans `localStorage['loredesktop.volume']` (0–1, défaut 1), relu à la création.

### Forme d'onde

- `fetch(src)` → `ArrayBuffer` ; si `byteLength > 50 Mo`, **pas de décodage** → mode barre (fallback).
- `AudioContext.decodeAudioData` (contexte fermé après usage). Échec (codec non géré par Web Audio, fichier corrompu) → mode barre ; le lecteur reste fonctionnel tant que `<audio>` sait lire.
- **`src/lib/audioPeaks.ts`** (module pur, testé vitest) :

```ts
/** Max absolu par bucket, toutes voies confondues, normalisé pour que le pic global = 1 (silence ⇒ zéros). */
export function computePeaks(channels: Float32Array[], buckets: number): number[]
/** "m:ss.cc" — centisecondes, l'échelle des SFX. */
export function formatTime(seconds: number): string
```

- Rendu : 120 barres SVG arrondies, hauteur `max(4 %, peak × 100 %)`, **deux tons par progression** (jouées = `var(--accent)`, restantes = ton mut du thème), fine ligne de tête de lecture.
- **Seek** : pointeur (down + drag, pointer capture) sur la zone waveform → `currentTime = ratio × duration`. Le fallback barre est cliquable pareil.
- Accessibilité : zone waveform `role="slider"` avec `aria-valuemin/max/now` et flèches ←/→ = ±1 s ; boutons avec `aria-label`.

### Habillage

- Carte aux tokens de l'app (`--panel`, `--border`, `--accent`, `--text-muted`), bouton play rond accent.
- Temps `--font-mono` : `0:00.46 / 0:01.20` (formatTime).
- Ligne d'infos sous les contrôles : `{sampleRate/1000} kHz · mono|stereo|N ch · durée`, tirée du buffer décodé. (Pas de bit depth : Web Audio ne l'expose pas — la maquette initiale le montrait, retiré du périmètre.)
- En mode barre (fallback), la ligne d'infos se réduit à la durée.

## Intégration

- `FilePreview.svelte` : la branche audio devient `<AudioPlayer src={preview.url} name={baseName(file.path)} />` ; la note « Audio asset — plays the working copy. » reste sous le lecteur.
- **Mock** : `silentWavDataUrl()` devient un burst de sinus 440 Hz avec enveloppe décroissante (~0,5 s, toujours généré à la volée) pour que la waveform soit visible en dev. Le test mock existant (préfixe data-URL) reste valable.

## Tests

- `audioPeaks.test.ts` : buckets corrects, normalisation (pic = 1), silence ⇒ zéros, multi-canaux (max des voies), `formatTime` (0 → `0:00.00`, 1.204 → `0:01.20`, 65.5 → `1:05.50`).
- Manuel mock : waveform visible sur `Audio/sfx_hit.wav`, seek au clic, loop, volume persistant après reload.
- Manuel app réelle : `Audio/sine_440.wav` de `lore-test-repo` — waveform du sinus, lecture, seek, infos `8 kHz · mono · 0:01.00`.

## Hors périmètre

Vitesse de lecture, spectrogramme, A/B before/after (bloqué CLI), waveform dans les listes (palier 3).

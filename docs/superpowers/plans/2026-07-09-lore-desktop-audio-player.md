# Waveform Audio Player Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Spec:** `docs/superpowers/specs/2026-07-09-lore-desktop-audio-player-design.md`

**Goal:** Replace the native `<audio controls>` with a themed waveform player (two-tone seekable waveform, centisecond times, loop, persisted volume, progress-bar fallback).

**Architecture:** Pure helpers in `audioPeaks.ts` (TDD), a self-contained `AudioPlayer.svelte` (hidden `<audio>` for playback, Web Audio decode for peaks, SVG bars), swapped into FilePreview's audio branch. Mock WAV becomes a sine burst so the waveform is visible in dev.

**Tech Stack:** Svelte 5 runes, Web Audio API, vitest.

**Commands:** `npm run check` · `npm test`

---

### Task 1: Pure helpers (`audioPeaks.ts`) — TDD

**Files:**
- Create: `src/lib/audioPeaks.ts`, `src/lib/audioPeaks.test.ts`

- [x] **Step 1: Failing tests**

`src/lib/audioPeaks.test.ts`:

```ts
import { describe, it, expect } from 'vitest'
import { computePeaks, formatTime } from './audioPeaks'

describe('computePeaks', () => {
  it('returns the requested bucket count', () => {
    const ch = new Float32Array(1000).fill(0.5)
    expect(computePeaks([ch], 120)).toHaveLength(120)
  })
  it('normalizes so the global peak is 1', () => {
    const ch = new Float32Array(100)
    ch[10] = 0.25
    ch[60] = 0.5
    const peaks = computePeaks([ch], 10)
    expect(Math.max(...peaks)).toBe(1)
    expect(peaks[1]).toBeCloseTo(0.5)
  })
  it('silence stays all zeros', () => {
    const peaks = computePeaks([new Float32Array(100)], 10)
    expect(peaks.every((p) => p === 0)).toBe(true)
  })
  it('takes the max across channels', () => {
    const a = new Float32Array(100)
    const b = new Float32Array(100)
    a[5] = 0.2
    b[5] = 0.8
    const peaks = computePeaks([a, b], 10)
    expect(peaks[0]).toBe(1) // 0.8 dominates and normalizes to 1
  })
})

describe('formatTime', () => {
  it('formats zero and invalid as 0:00.00', () => {
    expect(formatTime(0)).toBe('0:00.00')
    expect(formatTime(NaN)).toBe('0:00.00')
  })
  it('keeps centiseconds', () => {
    expect(formatTime(1.204)).toBe('0:01.20')
  })
  it('carries minutes', () => {
    expect(formatTime(65.5)).toBe('1:05.50')
  })
})
```

- [x] **Step 2: Run — FAIL (module missing)** — `npm test`

- [x] **Step 3: Implement**

`src/lib/audioPeaks.ts`:

```ts
/** Pure helpers for the audio player: waveform peaks + time formatting. */

/** Max absolute sample per bucket across all channels, normalized so the
 *  global peak is 1 (silence ⇒ all zeros). */
export function computePeaks(channels: Float32Array[], buckets: number): number[] {
  const n = channels[0]?.length ?? 0
  const out = new Array<number>(Math.max(0, buckets)).fill(0)
  if (!n || buckets <= 0) return out
  for (const ch of channels) {
    for (let b = 0; b < buckets; b++) {
      const start = Math.floor((b * n) / buckets)
      const end = Math.max(start + 1, Math.floor(((b + 1) * n) / buckets))
      let m = 0
      for (let i = start; i < end; i++) {
        const v = Math.abs(ch[i])
        if (v > m) m = v
      }
      if (m > out[b]) out[b] = m
    }
  }
  const max = Math.max(...out)
  return max > 0 ? out.map((v) => v / max) : out
}

/** "m:ss.cc" — centiseconds, the scale that matters for SFX. */
export function formatTime(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) seconds = 0
  const m = Math.floor(seconds / 60)
  const s = seconds - m * 60
  const whole = Math.floor(s)
  const cs = Math.floor((s - whole) * 100)
  return `${m}:${String(whole).padStart(2, '0')}.${String(cs).padStart(2, '0')}`
}
```

- [x] **Step 4: Run — PASS** — `npm test`

- [x] **Step 5: Commit**

```bash
git add src/lib/audioPeaks.ts src/lib/audioPeaks.test.ts
git commit -m "feat(audio): peak computation and centisecond time helpers"
```

---

### Task 2: `AudioPlayer.svelte` + integration

**Files:**
- Create: `src/lib/AudioPlayer.svelte`
- Modify: `src/lib/Icon.svelte` (icons play/pause/repeat/volume), `src/lib/FilePreview.svelte` (use the component), `src/lib/mock.ts` (sine-burst WAV)

- [x] **Step 1: Icons** — add to the `paths` record in `Icon.svelte`:

```ts
    play: 'M5 3l14 9-14 9V3z',
    pause: 'M6 4h4v16H6z M14 4h4v16h-4z',
    repeat: 'M17 1l4 4-4 4 M3 11V9a4 4 0 0 1 4-4h14 M7 23l-4-4 4-4 M21 13v2a4 4 0 0 1-4 4H3',
    volume: 'M11 5L6 9H2v6h4l5 4V5z M15.54 8.46a5 5 0 0 1 0 7.07',
```

- [x] **Step 2: Create `src/lib/AudioPlayer.svelte`**

```svelte
<script lang="ts">
  import { computePeaks, formatTime } from './audioPeaks'
  import Icon from './Icon.svelte'

  let { src, name }: { src: string; name: string } = $props()

  const VOLUME_KEY = 'loredesktop.volume'
  const BUCKETS = 120
  const MAX_DECODE_BYTES = 50 * 1024 * 1024

  let audio = $state<HTMLAudioElement | null>(null)
  let playing = $state(false)
  let current = $state(0)
  let duration = $state(0)
  let loop = $state(false)
  let volume = $state(Math.min(1, Math.max(0, Number(localStorage.getItem(VOLUME_KEY) ?? '1') || 0)))
  let peaks = $state<number[] | null>(null)
  let info = $state<{ sampleRate: number; channels: number } | null>(null)
  let waveEl = $state<HTMLDivElement>()

  // One <audio> per src — never autoplays, always stopped on change/unmount.
  $effect(() => {
    const a = new Audio(src)
    a.preload = 'metadata'
    a.addEventListener('timeupdate', () => (current = a.currentTime))
    a.addEventListener('loadedmetadata', () => (duration = a.duration || 0))
    a.addEventListener('play', () => (playing = true))
    a.addEventListener('pause', () => (playing = false))
    audio = a
    return () => {
      a.pause()
      a.removeAttribute('src')
      audio = null
      playing = false
      current = 0
      duration = 0
    }
  })
  $effect(() => { if (audio) audio.loop = loop })
  $effect(() => {
    if (audio) audio.volume = volume
    localStorage.setItem(VOLUME_KEY, String(volume))
  })

  // Waveform peaks via Web Audio; any failure just leaves the bar fallback.
  $effect(() => {
    const s = src
    peaks = null
    info = null
    ;(async () => {
      try {
        const buf = await (await fetch(s)).arrayBuffer()
        if (buf.byteLength > MAX_DECODE_BYTES) return
        const ctx = new AudioContext()
        try {
          const decoded = await ctx.decodeAudioData(buf)
          if (src !== s) return
          const channels = Array.from({ length: decoded.numberOfChannels }, (_, i) => decoded.getChannelData(i))
          peaks = computePeaks(channels, BUCKETS)
          info = { sampleRate: decoded.sampleRate, channels: decoded.numberOfChannels }
        } finally {
          ctx.close()
        }
      } catch { /* bar mode */ }
    })()
  })

  function toggle() {
    if (!audio) return
    if (playing) audio.pause()
    else audio.play()
  }

  let seeking = false
  function seekAt(e: PointerEvent) {
    if (!audio || !duration || !waveEl) return
    const r = waveEl.getBoundingClientRect()
    const ratio = Math.min(1, Math.max(0, (e.clientX - r.left) / r.width))
    audio.currentTime = ratio * duration
    current = audio.currentTime
  }
  function onDown(e: PointerEvent) { seeking = true; waveEl?.setPointerCapture(e.pointerId); seekAt(e) }
  function onMove(e: PointerEvent) { if (seeking) seekAt(e) }
  function onUp() { seeking = false }
  function onKey(e: KeyboardEvent) {
    if (!audio || !duration) return
    if (e.key === 'ArrowRight') { e.preventDefault(); audio.currentTime = Math.min(duration, audio.currentTime + 1) }
    if (e.key === 'ArrowLeft') { e.preventDefault(); audio.currentTime = Math.max(0, audio.currentTime - 1) }
  }

  const progress = $derived(duration > 0 ? current / duration : 0)
  const chLabel = $derived(!info ? '' : info.channels === 1 ? 'mono' : info.channels === 2 ? 'stereo' : `${info.channels} ch`)
</script>

<div class="player">
  <div class="row">
    <button class="play" onclick={toggle} aria-label={playing ? `Pause ${name}` : `Play ${name}`}>
      <Icon name={playing ? 'pause' : 'play'} size={16} />
    </button>
    <div class="mid">
      <div class="wave" bind:this={waveEl} role="slider" tabindex="0" aria-label="Seek"
           aria-valuemin={0} aria-valuemax={Math.round(duration)} aria-valuenow={Math.round(current)}
           onpointerdown={onDown} onpointermove={onMove} onpointerup={onUp} onkeydown={onKey}>
        {#if peaks}
          <svg viewBox="0 0 {BUCKETS * 4} 56" preserveAspectRatio="none" aria-hidden="true">
            {#each peaks as p, i (i)}
              <rect x={i * 4} width="2.6" y={28 - Math.max(2.2, p * 26)} height={Math.max(4.4, p * 52)} rx="1.2"
                    fill={i / BUCKETS < progress ? 'var(--accent)' : 'var(--border-strong)'} />
            {/each}
            {#if duration}<line x1={progress * BUCKETS * 4} y1="0" x2={progress * BUCKETS * 4} y2="56" stroke="var(--text)" stroke-width="1.5" />{/if}
          </svg>
        {:else}
          <div class="bar"><div class="fill" style="width:{progress * 100}%"></div></div>
        {/if}
      </div>
      <div class="times"><span class="cur">{formatTime(current)}</span><span>{formatTime(duration)}</span></div>
    </div>
  </div>
  <div class="ctrl">
    <button class="loopbtn" class:on={loop} onclick={() => (loop = !loop)} title="Loop playback">
      <Icon name="repeat" size={13} /> Loop
    </button>
    <span class="vol"><Icon name="volume" size={14} /><input type="range" min="0" max="1" step="0.05" bind:value={volume} aria-label="Volume" /></span>
    <span class="fmt">{#if info}{(info.sampleRate / 1000).toLocaleString()} kHz · {chLabel} · {formatTime(duration)}{:else if duration}{formatTime(duration)}{/if}</span>
  </div>
</div>

<style>
  .player { border: 1px solid var(--border); border-radius: 10px; background: var(--panel); padding: 12px 14px; margin: 4px 0; }
  .row { display: flex; align-items: center; gap: 12px; }
  .play { width: 38px; height: 38px; border-radius: 50%; padding: 0; background: var(--accent); color: var(--on-accent); border: none; display: grid; place-items: center; flex: none; }
  .play :global(svg) { margin-left: 1px; }
  .mid { flex: 1; min-width: 0; }
  .wave { height: 56px; cursor: pointer; touch-action: none; }
  .wave:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; border-radius: 4px; }
  .wave svg { width: 100%; height: 100%; display: block; }
  .bar { height: 4px; margin-top: 26px; background: var(--border); border-radius: 2px; position: relative; }
  .bar .fill { position: absolute; inset: 0 auto 0 0; background: var(--accent); border-radius: 2px; }
  .times { display: flex; justify-content: space-between; font-family: var(--font-mono); font-size: 11px; color: var(--text-muted); margin-top: 4px; }
  .times .cur { color: var(--text); }
  .ctrl { display: flex; align-items: center; gap: 14px; margin-top: 8px; padding-left: 50px; }
  .loopbtn { display: inline-flex; align-items: center; gap: 5px; font-size: 11.5px; padding: 3px 9px; border-radius: 6px; }
  .loopbtn.on { background: var(--accent-soft); color: var(--accent-text); border-color: var(--accent); }
  .vol { display: inline-flex; align-items: center; gap: 6px; color: var(--text-muted); }
  .vol input { width: 70px; accent-color: var(--accent); }
  .fmt { margin-left: auto; font-size: 11px; color: var(--text-dim); }
</style>
```

- [x] **Step 3: FilePreview** — replace the audio branch:

```svelte
        {#if preview?.kind === 'audio' && preview.url}
          <AudioPlayer src={preview.url} name={baseName(file.path)} />
          <p class="note muted"><Icon name="info" size={14} /> Audio asset — plays the working copy.</p>
        {:else}
```

(import `AudioPlayer from './AudioPlayer.svelte'`; drop the now-unused `.audio` CSS rules.)

- [x] **Step 4: Mock WAV becomes a sine burst** — in `mock.ts`, replace `silentWavDataUrl` with:

```ts
/** Small 440 Hz sine burst with decay (~0.5 s) so the mock waveform has a visible shape. */
export function mockWavDataUrl(): string {
  const sampleRate = 8000, samples = 4000
  const buf = new ArrayBuffer(44 + samples * 2)
  const v = new DataView(buf)
  const w4 = (o: number, s: string) => { for (let i = 0; i < 4; i++) v.setUint8(o + i, s.charCodeAt(i)) }
  w4(0, 'RIFF'); v.setUint32(4, 36 + samples * 2, true); w4(8, 'WAVE')
  w4(12, 'fmt '); v.setUint32(16, 16, true); v.setUint16(20, 1, true); v.setUint16(22, 1, true)
  v.setUint32(24, sampleRate, true); v.setUint32(28, sampleRate * 2, true); v.setUint16(32, 2, true); v.setUint16(34, 16, true)
  w4(36, 'data'); v.setUint32(40, samples * 2, true)
  for (let i = 0; i < samples; i++) {
    const env = Math.exp(-i / 1200)
    v.setInt16(44 + i * 2, Math.round(Math.sin((2 * Math.PI * 440 * i) / sampleRate) * env * 20000), true)
  }
  let bin = ''
  new Uint8Array(buf).forEach((b) => (bin += String.fromCharCode(b)))
  return `data:audio/wav;base64,${btoa(bin)}`
}
```

and update the `getPreview` audio branch to `url: mockWavDataUrl()`.

- [x] **Step 5: Verify + commit**

Run: `npm run check && npm test` — PASS.

```bash
git add src/lib/AudioPlayer.svelte src/lib/Icon.svelte src/lib/FilePreview.svelte src/lib/mock.ts
git commit -m "feat(audio): themed waveform player - seek, loop, persisted volume, bar fallback"
```

---

### Task 3: Verification

- [x] **Step 1: Suites** — `npm run check && npm test` PASS.
- [x] **Step 2: Mock (browser)** — sélectionner `Audio/sfx_hit.wav` : waveform en forme de burst, play/pause, seek au clic (temps bouge), Loop togglable, volume modifié + reload → persisté ; `T_Icon_Sword.png` inchangé.
- [x] **Step 3: Real app** — `Audio/sine_440.wav` : waveform du sinus (enveloppe plate), infos « 8 kHz · mono · 0:01.00 », lecture réelle streaming.
- [x] **Step 4: Commit fixes** if any.

---

## Self-review

- **Spec coverage :** carte waveform + fallback (Task 2), helpers purs testés (Task 1), loop/volume persistant/pas d'autoplay (Task 2), seek pointeur + clavier + ARIA (Task 2), ligne d'infos sans bit depth (Task 2), burst mock (Task 2 Step 4). ✔
- **Placeholders :** aucun. ✔
- **Type consistency :** `computePeaks(Float32Array[], number): number[]` et `formatTime(number): string` identiques tests/impl/composant ; props `{src, name}` alignées avec l'appel FilePreview. ✔

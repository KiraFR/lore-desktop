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

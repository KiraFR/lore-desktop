<script lang="ts">
  import { computePeaks, formatTime, wavSampleRate } from './audioPeaks'
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
  let muted = $state(false)
  let peaks = $state<number[] | null>(null)
  // sampleRate is the SOURCE rate (WAV header) — decodeAudioData resamples to
  // the device rate, so its sampleRate would lie; null hides the kHz segment.
  let info = $state<{ sampleRate: number | null; channels: number } | null>(null)
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
  // `muted` zeroes the output while remembering the level to restore. Only the
  // level (not the mute flag) is persisted.
  const shownVol = $derived(muted ? 0 : volume)
  $effect(() => {
    if (audio) audio.volume = shownVol
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
          const srcRate = wavSampleRate(buf)
          const decoded = await ctx.decodeAudioData(buf)
          if (src !== s) return
          const channels = Array.from({ length: decoded.numberOfChannels }, (_, i) => decoded.getChannelData(i))
          peaks = computePeaks(channels, BUCKETS)
          info = { sampleRate: srcRate, channels: decoded.numberOfChannels }
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

  // Custom volume slider — the native <input type=range> only responded to a
  // click, not a drag (its bind:value write-back fought the drag). This reuses
  // the waveform's pointer-capture pattern, which drags smoothly.
  let volEl = $state<HTMLDivElement>()
  let volDrag = false
  function volAt(e: PointerEvent) {
    if (!volEl) return
    const r = volEl.getBoundingClientRect()
    volume = Math.min(1, Math.max(0, (e.clientX - r.left) / r.width))
    muted = false
  }
  function onVolDown(e: PointerEvent) { volDrag = true; volEl?.setPointerCapture(e.pointerId); volAt(e) }
  function onVolMove(e: PointerEvent) { if (volDrag) volAt(e) }
  function onVolUp() { volDrag = false }
  function onVolKey(e: KeyboardEvent) {
    if (e.key === 'ArrowRight' || e.key === 'ArrowUp') { e.preventDefault(); volume = Math.min(1, +(volume + 0.05).toFixed(2)); muted = false }
    if (e.key === 'ArrowLeft' || e.key === 'ArrowDown') { e.preventDefault(); volume = Math.max(0, +(volume - 0.05).toFixed(2)); muted = false }
  }
  function toggleMute() {
    // Unmuting a level-0 slider gives a sensible default rather than staying silent.
    if (muted) { muted = false; if (volume === 0) volume = 0.5 }
    else muted = true
  }

  const progress = $derived(duration > 0 ? current / duration : 0)
  const chLabel = $derived(!info ? '' : info.channels === 1 ? 'mono' : info.channels === 2 ? 'stereo' : `${info.channels} ch`)
  const fmtLabel = $derived.by(() => {
    if (!info) return duration ? formatTime(duration) : ''
    const parts: string[] = []
    if (info.sampleRate) parts.push(`${(info.sampleRate / 1000).toLocaleString()} kHz`)
    parts.push(chLabel, formatTime(duration))
    return parts.join(' · ')
  })
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
    <span class="vol">
      <button class="mute" onclick={toggleMute} aria-label={shownVol === 0 ? 'Unmute' : 'Mute'} title={shownVol === 0 ? 'Unmute' : 'Mute'}>
        <Icon name={shownVol === 0 ? 'volumeOff' : 'volume'} size={14} />
      </button>
      <div class="vslider" bind:this={volEl} role="slider" tabindex="0" aria-label="Volume"
           aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.round(shownVol * 100)}
           onpointerdown={onVolDown} onpointermove={onVolMove} onpointerup={onVolUp} onkeydown={onVolKey}>
        <div class="vtrack"><div class="vfill" style="width:{shownVol * 100}%"></div></div>
        <div class="vthumb" style="left:{shownVol * 100}%"></div>
      </div>
    </span>
    <span class="fmt">{fmtLabel}</span>
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
  .vol { display: inline-flex; align-items: center; gap: 7px; color: var(--text-muted); }
  .mute { padding: 2px; border: none; background: transparent; color: var(--text-muted); display: grid; place-items: center; border-radius: 5px; }
  .mute:hover { color: var(--text); background: var(--panel-hover); }
  .vslider { position: relative; width: 80px; height: 16px; display: flex; align-items: center; cursor: pointer; touch-action: none; }
  .vslider:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; border-radius: 4px; }
  .vtrack { width: 100%; height: 4px; background: var(--border); border-radius: 2px; overflow: hidden; }
  .vfill { height: 100%; background: var(--accent); border-radius: 2px; }
  .vthumb { position: absolute; top: 50%; width: 11px; height: 11px; border-radius: 50%; background: var(--accent); transform: translate(-50%, -50%); pointer-events: none; box-shadow: 0 0 0 2px var(--panel); }
  .fmt { margin-left: auto; font-size: 11px; color: var(--text-dim); }
</style>

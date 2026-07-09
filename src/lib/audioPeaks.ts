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

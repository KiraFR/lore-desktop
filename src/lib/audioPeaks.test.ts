import { describe, it, expect } from 'vitest'
import { computePeaks, formatTime, wavSampleRate } from './audioPeaks'

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

describe('wavSampleRate', () => {
  function wavHeader(rate: number): ArrayBuffer {
    const buf = new ArrayBuffer(44)
    const v = new DataView(buf)
    const w4 = (o: number, s: string) => { for (let i = 0; i < 4; i++) v.setUint8(o + i, s.charCodeAt(i)) }
    w4(0, 'RIFF'); v.setUint32(4, 36, true); w4(8, 'WAVE')
    w4(12, 'fmt '); v.setUint32(16, 16, true); v.setUint16(20, 1, true); v.setUint16(22, 1, true)
    v.setUint32(24, rate, true)
    w4(36, 'data'); v.setUint32(40, 0, true)
    return buf
  }
  it('reads the fmt chunk sample rate', () => {
    expect(wavSampleRate(wavHeader(8000))).toBe(8000)
    expect(wavSampleRate(wavHeader(44100))).toBe(44100)
  })
  it('rejects non-WAV data', () => {
    expect(wavSampleRate(new ArrayBuffer(10))).toBeNull()
    expect(wavSampleRate(new ArrayBuffer(64))).toBeNull()
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

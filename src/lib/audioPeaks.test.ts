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

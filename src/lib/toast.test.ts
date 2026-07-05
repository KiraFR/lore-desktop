import { describe, it, expect, beforeEach, vi } from 'vitest'
import { get } from 'svelte/store'
import { toasts, toastError, dismissToast } from './toast'

describe('toast store', () => {
  beforeEach(() => { toasts.set([]) })

  it('toastError adds a red toast with title + message', () => {
    toastError('Clone failed', new Error('boom'))
    const list = get(toasts)
    expect(list).toHaveLength(1)
    expect(list[0].title).toBe('Clone failed')
    expect(list[0].message).toBe('boom')
  })

  it('dismissToast removes the toast', () => {
    const id = toastError('Nope')
    dismissToast(id)
    expect(get(toasts)).toHaveLength(0)
  })

  it('auto-expires after the TTL', () => {
    vi.useFakeTimers()
    toastError('Nope')
    expect(get(toasts)).toHaveLength(1)
    vi.advanceTimersByTime(6000)
    expect(get(toasts)).toHaveLength(0)
    vi.useRealTimers()
  })
})

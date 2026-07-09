import { describe, it, expect, beforeEach, vi } from 'vitest'
import { get } from 'svelte/store'
import { toasts, toastError, toastAction, toastInfo, dismissToast } from './toast'

describe('toast store', () => {
  beforeEach(() => { toasts.set([]) })

  it('toastError adds a red toast with title + message', () => {
    toastError('Clone failed', new Error('boom'))
    const list = get(toasts)
    expect(list).toHaveLength(1)
    expect(list[0].title).toBe('Clone failed')
    expect(list[0].message).toBe('boom')
    expect(list[0].variant).toBe('error')
  })

  it('toastAction adds an info toast carrying the action', () => {
    let ran = false
    toastAction('2 files pushed', { label: 'Release locks', run: () => { ran = true } })
    const list = get(toasts)
    expect(list).toHaveLength(1)
    expect(list[0].variant).toBe('info')
    expect(list[0].action?.label).toBe('Release locks')
    list[0].action!.run()
    expect(ran).toBe(true)
  })

  it('toastInfo adds a plain info toast', () => {
    toastInfo('Rev 7 pushed by a teammate')
    const list = get(toasts)
    expect(list).toHaveLength(1)
    expect(list[0].variant).toBe('info')
    expect(list[0].action).toBeUndefined()
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

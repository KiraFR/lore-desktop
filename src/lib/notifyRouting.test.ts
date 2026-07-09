import { describe, it, expect } from 'vitest'
import { planFor, type LoreNotification } from './notifyRouting'

const push = (userId: string, revisionNumber = 7): LoreNotification => ({
  tagName: 'notificationBranchPushed',
  data: { revision: 'abc', revisionNumber, branch: 'b1', userId },
})
const locked = (paths: string[]): LoreNotification => ({
  tagName: 'notificationResourceLocked',
  data: { userId: 'u2', branch: 'b1', paths },
})

describe('planFor', () => {
  it('a teammate push refreshes status and raises a toast', () => {
    const plan = planFor([push('other')], 'me')
    expect(plan.status).toBe(true)
    expect(plan.pushToast).toEqual({ revisionNumber: 7 })
    expect(plan.locks).toBe(false)
  })
  it('my own push refreshes silently', () => {
    const plan = planFor([push('me')], 'me')
    expect(plan.status).toBe(true)
    expect(plan.pushToast).toBeNull()
  })
  it('lock events only touch locks', () => {
    const plan = planFor([locked(['a.uasset'])], 'me')
    expect(plan).toEqual({ status: false, locks: true, pushToast: null })
  })
  it('coalesced bursts combine flags', () => {
    const plan = planFor([locked(['a']), push('other', 9), locked(['b'])], 'me')
    expect(plan.status).toBe(true)
    expect(plan.locks).toBe(true)
    expect(plan.pushToast).toEqual({ revisionNumber: 9 })
  })
  it('unknown identity never toasts', () => {
    expect(planFor([push('other')], null).pushToast).toBeNull()
  })
})

/** Pure routing for coalesced server notifications. */

export interface LoreNotification {
  tagName: string
  data: Record<string, unknown>
}

export interface RefreshPlan {
  status: boolean
  locks: boolean
  /** Set when a teammate (not `myUserId`) pushed — worth a toast. */
  pushToast: { revisionNumber: number } | null
}

export function planFor(events: LoreNotification[], myUserId: string | null): RefreshPlan {
  const plan: RefreshPlan = { status: false, locks: false, pushToast: null }
  for (const e of events) {
    if (e.tagName === 'notificationBranchPushed') {
      plan.status = true
      const uid = typeof e.data.userId === 'string' ? e.data.userId : null
      // No toast without a known identity: better silent than pinging on our own pushes.
      if (uid && myUserId && uid !== myUserId) {
        plan.pushToast = { revisionNumber: Number(e.data.revisionNumber ?? 0) }
      }
    } else if (e.tagName === 'notificationResourceLocked' || e.tagName === 'notificationResourceUnlocked') {
      plan.locks = true
    }
  }
  return plan
}

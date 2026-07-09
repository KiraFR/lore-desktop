import { api } from './api'
import { session } from './session.svelte'
import { refreshStatus, refreshLocks, refreshHistory } from './repo.svelte'
import { planFor, type LoreNotification } from './notifyRouting'
import { toastInfo } from './toast'

let stop: (() => void) | null = null
let watchToken = 0
let pending: LoreNotification[] = []
let timer: ReturnType<typeof setTimeout> | null = null

/** Subscribe to the given repo's live server events (null = unsubscribe).
 *  Idempotent; a newer call always supersedes an in-flight one. */
export async function watchRepo(repoPath: string | null) {
  const token = ++watchToken
  stop?.()
  stop = null
  if (!repoPath) return
  const s = await api.startNotifications(repoPath, onEvent)
  if (token !== watchToken) {
    s() // superseded while awaiting
    return
  }
  stop = s
}

function onEvent(e: LoreNotification) {
  pending.push(e)
  if (timer) clearTimeout(timer)
  timer = setTimeout(flush, 400) // coalesce bursts into one refresh round
}

function flush() {
  timer = null
  const events = pending
  pending = []
  const plan = planFor(events, session.identity?.id ?? null)
  if (plan.status) {
    refreshStatus(true)
    refreshHistory(true)
  }
  if (plan.locks) refreshLocks(true)
  if (plan.pushToast) toastInfo(`Rev ${plan.pushToast.revisionNumber} pushed by a teammate`)
}

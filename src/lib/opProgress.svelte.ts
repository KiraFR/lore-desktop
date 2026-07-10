import type { OpProgress } from './types'

// Live progress of the long-running operations, one slot per kind (an opId
// already isolates concurrent ops backend-side; the UI shows one bar per
// button/flow, so per-kind slots are enough). Null = idle.
export const opProgress = $state({
  clone: null as OpProgress | null,
  sync: null as OpProgress | null,
  push: null as OpProgress | null,
})

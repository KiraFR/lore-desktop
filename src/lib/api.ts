import { mock } from './mock'
import { tauriApi } from './tauri'
import type { LoreApi } from './types'

const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

export const api: LoreApi = inTauri ? tauriApi : mock
export * from './types'

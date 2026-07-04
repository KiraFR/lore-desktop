// The app's single data boundary. In this slice it re-exports the mock; the
// wiring slice will replace the body with a Tauri-`invoke` implementation of the
// same `LoreApi` interface — components never change.
import { mock } from './mock'
import type { LoreApi } from './types'

export const api: LoreApi = mock
export * from './types'

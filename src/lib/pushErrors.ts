/**
 * Non-fast-forward push detection (the remote advanced under us).
 * The backend surfaces the CLI `error` event as the JSON serialization of its
 * `data` (check_ok, src-tauri/src/lore.rs); tauri invoke rejects with that raw
 * string. PINNED on the real capture src-tauri/tests/fixtures/push_nonff.ndjson
 * — no broad match.
 */

/** Verbatim shape the frontend sees for the captured refusal (check_ok output,
 *  serde_json sorts keys alphabetically). The runtime string may also carry a
 *  trailing " (lore exited with …)" suffix from the streaming runner, which the
 *  marker match tolerates. */
export const NON_FF_PUSH_SAMPLE = '{"errorInner":"Branch has diverged, sync to merge remote changes","errorType":4294967295}'

/** Discriminant substring of the refusal (from the CLI's errorInner). */
export const NON_FF_PUSH_MARKER = 'Branch has diverged'

export function isNonFastForwardPush(message: string): boolean {
  return message.includes(NON_FF_PUSH_MARKER)
}

/** Tauri invoke rejects with a string; the mock throws an Error. */
export function errorMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e)
}

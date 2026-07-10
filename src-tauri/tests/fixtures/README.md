# Captured `lore 0.8.3 --json` fixtures

Real output from a 2-commit test repo (`desktoptest1`) on `lore.example.com`,
used as the test oracle for the Rust parsers. Regenerate with the commands in the
Slice A plan, Task 2.

## Pinned encodings

**Stream:** one `{"tagName","data"}` object per line, ending with
`{"tagName":"complete","data":{"status":N}}`. `history --json` emits **two**
`complete` events: the listing, then a trailing `authUserInfo` block.

**`repositoryStatusRevision`:** `branchName` (display name, e.g. "main"),
`revisionLocalNumber` / `revisionRemoteNumber` (u64), `isLocalAhead` /
`isRemoteAhead` (JSON `true`/`false`). Ahead counts = `localNumber - remoteNumber`
(and vice-versa), gated by the boolean.

**`repositoryStatusFile`:** `action` is a **string** — `"keep"` (content modify),
`"add"`, `"delete"`, `"move"`, `"copy"`. Map `keep → modify`. Fields: `path`,
`size` (u64), `type` (`"file"`/`"directory"`/`"link"`), `flagDirty` / `flagStaged`
/ … (JSON booleans), `fromPath`. **No binary field** → infer `isBinary` from the
extension. A clean repo emits zero file events (only the revision + `complete`).

**`revisionHistoryEntry`:** `revision` (hash string), `revisionNumber` (u64),
`parent` = `[hash, hash]` (parent[0] = direct parent, parent[1] = merge parent or
all-zeros). A zero hash is 64 `'0'` chars; filter those out.

**`metadata`** (interleaved after each `revisionHistoryEntry`, until the next
entry): `{"key":"…","value":{"tagName":"string|numeric|context","data":<v>}}` —
extract `value.data`. Relevant keys:
- `message` → commit message (string).
- `created-by` → author **user id** (uuid string). Resolve to a display name via
  the trailing `authUserInfo` events.
- `committed-by` → committer user id.
- `timestamp` → commit time, epoch **milliseconds** (numeric).
- `branch` → branch context id (NOT the display name).

**`authUserInfo`** (after the listing's `complete`): `{"id":"<uuid>","name":"<display>"}`
→ the id→name map used to resolve `created-by` / `committed-by`.

**`fileInfo`** (`file info <paths…> --json`, batch — one event per file, even
when the paths passed on the CLI are absolute): `path` (relative to the repo
root, **not** echoed absolute), `size` (u64, size at the repo's current
revision — this is the "old" side of the weight delta; the "new" side is the
local `size` of `repositoryStatusFile`). Also present but not currently
consumed: `context` (uuid), `hash` / `localHash` (content hash strings —
`localHash` is all-zeros when the file has local modifications, i.e. not
computed), `isFile` / `isDir`, `flagModified` / `flagDeleted` / `flagAdded` /
`flagConflict` (booleans), `mode` (number), `localSize` (u64, local disk
size), `filterSize` (number). No per-file error event was observed; batch
still ends with the usual `{"tagName":"complete","data":{"status":0}}`.

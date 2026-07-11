/** Known repo paths whose folder has vanished from disk. Unknown paths (the
 *  existence check hasn't run yet) count as present — never alarm early. */
export function missingRepos(paths: string[], exists: Map<string, boolean>): Set<string> {
  return new Set(paths.filter((p) => exists.get(p) === false))
}

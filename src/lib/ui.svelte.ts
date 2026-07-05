// Which primary view is showing. Shared so the nav rail, the branch menu
// (which opens Merge), and Merge itself can all drive navigation.
export type View = 'changes' | 'history' | 'merge' | 'locks'

export const ui = $state({ view: 'changes' as View })

export function setView(v: View) {
  ui.view = v
}

export type Theme = 'light' | 'dark'
const CACHE_KEY = 'loredesktop.theme'

/** The effective theme from config (unset → dark, the app's original look). */
export function resolveTheme(theme: Theme | null | undefined): Theme {
  return theme === 'light' ? 'light' : 'dark'
}

/** Apply a theme to the document + cache it for a flash-free next boot. Dark is
 *  the CSS default, so we REMOVE the attribute for dark and set it for light. */
export function applyTheme(theme: Theme): void {
  const root = document.documentElement
  if (theme === 'light') root.dataset.theme = 'light'
  else delete root.dataset.theme
  try { localStorage.setItem(CACHE_KEY, theme) } catch { /* ignore */ }
}

/** Last applied theme, cached in localStorage — read synchronously at startup
 *  (before config loads) to avoid a dark→light flash. Defaults to dark. */
export function cachedTheme(): Theme {
  try { return localStorage.getItem(CACHE_KEY) === 'light' ? 'light' : 'dark' } catch { return 'dark' }
}

export function otherTheme(t: Theme): Theme {
  return t === 'light' ? 'dark' : 'light'
}

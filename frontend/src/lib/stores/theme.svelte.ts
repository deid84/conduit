export type ThemePreference = 'light' | 'dark' | 'system'
export type Theme = 'light' | 'dark'

function load(): ThemePreference {
  try { return (localStorage.getItem('theme') as ThemePreference) ?? 'system' } catch { return 'system' }
}

function applyToDOM(pref: ThemePreference) {
  if (typeof document === 'undefined') return
  if (pref === 'system') document.documentElement.removeAttribute('data-theme')
  else                   document.documentElement.setAttribute('data-theme', pref)
}

export const themeStore = $state({ preference: load() })

// Resolve 'system' → actual 'light' | 'dark' based on OS preference at call time.
export function effectiveTheme(): Theme {
  if (themeStore.preference !== 'system') return themeStore.preference
  return (typeof window !== 'undefined' && window.matchMedia('(prefers-color-scheme: dark)').matches)
    ? 'dark' : 'light'
}

export function setTheme(pref: ThemePreference) {
  themeStore.preference = pref
  try { localStorage.setItem('theme', pref) } catch { /* */ }
  applyToDOM(pref)
}

// Apply immediately on module load so the DOM reflects the saved preference
// before the first render, avoiding a flash.
applyToDOM(themeStore.preference)

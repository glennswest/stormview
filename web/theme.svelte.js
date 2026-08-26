// Theme selection: sets data-theme on <html> (the token overrides in
// themes.css do the rest). Two inputs, in priority order: the viewer's own
// explicit pick (localStorage, per browser) beats the server's configured
// default; the default beats 'storm'. Shared by every storm web UI so the
// fleet looks like one system.

export const THEMES = [
  // dark
  { id: 'storm', label: 'Storm' },
  { id: 'one', label: 'One' },
  { id: 'gruvbox', label: 'Gruvbox' },
  { id: 'catppuccin', label: 'Catppuccin' },
  { id: 'rose', label: 'Rosé' },
  { id: 'midnight', label: 'Midnight' },
  { id: 'nord', label: 'Nord' },
  { id: 'solar', label: 'Solar' },
  { id: 'phosphor', label: 'Phosphor' },
  // light
  { id: 'light', label: 'Light' },
  { id: 'frost', label: 'Frost' },
  { id: 'paper', label: 'Paper' },
]

const KEY = 'storm-theme'

function valid(id) {
  return THEMES.some((t) => t.id === id)
}

function storedChoice() {
  try {
    const t = localStorage.getItem(KEY)
    return valid(t) ? t : null
  } catch {
    return null
  }
}

export const theme = $state({ current: storedChoice() || 'storm' })

function apply(id) {
  theme.current = id
  if (id === 'storm') delete document.documentElement.dataset.theme
  else document.documentElement.dataset.theme = id
}

/// The viewer picked a theme — apply it and remember it as their choice.
export function applyTheme(id) {
  if (!valid(id)) return
  apply(id)
  try {
    localStorage.setItem(KEY, id)
  } catch {}
}

/// The server's configured default — applies only for viewers who never
/// picked one themselves, and is not persisted as a choice.
export function setDefaultTheme(id) {
  if (!valid(id) || storedChoice()) return
  apply(id)
}

export function initTheme() {
  apply(theme.current)
}

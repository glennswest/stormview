// Shared formatting and ANSI rendering, so every storm UI prints the same
// things the same way. Mirrors the Rust helpers in src/lib.rs.

export function formatBytes(bytes) {
  if (bytes == null) return '-'
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1)
  const val = bytes / Math.pow(1024, i)
  return val.toFixed(i > 0 ? 1 : 0) + ' ' + units[i]
}

export function formatDuration(secs) {
  if (secs == null) return '-'
  const d = Math.floor(secs / 86400)
  const h = Math.floor((secs % 86400) / 3600)
  const m = Math.floor((secs % 3600) / 60)
  const s = Math.floor(secs % 60)
  if (d > 0) return `${d}d ${h}h`
  if (h > 0) return `${h}h ${m}m`
  if (m > 0) return `${m}m ${s}s`
  return `${s}s`
}

export function timeAgo(ts) {
  if (!ts) return '-'
  const diff = (Date.now() - new Date(ts).getTime()) / 1000
  if (diff < 60) return Math.floor(diff) + 's ago'
  if (diff < 3600) return Math.floor(diff / 60) + 'm ago'
  if (diff < 86400) return Math.floor(diff / 3600) + 'h ago'
  return Math.floor(diff / 86400) + 'd ago'
}

export function escapeHtml(s) {
  return String(s)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
}

// Colors come from the theme's --ansi-* tokens (inline styles may reference
// CSS variables), so rendered output re-colors with the theme.
const ANSI_STYLES = {
  1: 'font-weight:bold',
  2: 'opacity:0.7',
  3: 'font-style:italic',
  4: 'text-decoration:underline',
  30: 'color:var(--ansi-black)',
  31: 'color:var(--ansi-red)',
  32: 'color:var(--ansi-green)',
  33: 'color:var(--ansi-yellow)',
  34: 'color:var(--ansi-blue)',
  35: 'color:var(--ansi-magenta)',
  36: 'color:var(--ansi-cyan)',
  37: 'color:var(--ansi-white)',
  90: 'color:var(--ansi-br-black)',
  91: 'color:var(--ansi-br-red)',
  92: 'color:var(--ansi-br-green)',
  93: 'color:var(--ansi-br-yellow)',
  94: 'color:var(--ansi-br-blue)',
  95: 'color:var(--ansi-br-magenta)',
  96: 'color:var(--ansi-br-cyan)',
  97: 'color:var(--ansi-br-white)',
}

export function ansiToHtml(text) {
  text = String(text)
    .replace(/\x1b\[\d*[ABCDHJ]/g, '')
    .replace(/\x1b\[\d*;\d*[Hf]/g, '')
    .replace(/\x1b\[\??\d*[hlr]/g, '')

  let result = ''
  let openSpans = 0
  const parts = text.split(/\x1b\[/)

  result += escapeHtml(parts[0])
  for (let i = 1; i < parts.length; i++) {
    const match = parts[i].match(/^([\d;]*)m([\s\S]*)/)
    if (match) {
      const codes = match[1]
      const rest = match[2]
      if (codes === '0' || codes === '') {
        while (openSpans > 0) { result += '</span>'; openSpans-- }
      } else {
        const styles = []
        for (const code of codes.split(';')) {
          if (code === '0') {
            while (openSpans > 0) { result += '</span>'; openSpans-- }
          } else if (ANSI_STYLES[code]) {
            styles.push(ANSI_STYLES[code])
          }
        }
        if (styles.length > 0) {
          result += '<span style="' + styles.join(';') + '">'
          openSpans++
        }
      }
      result += escapeHtml(rest)
    } else {
      result += '\x1b[' + escapeHtml(parts[i])
    }
  }
  while (openSpans > 0) { result += '</span>'; openSpans-- }
  return result
}

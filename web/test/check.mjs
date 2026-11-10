// Compiles every component, then mounts LoginPanel in jsdom and walks its
// password, enrol and totp steps. Run by check.sh (needs svelte + jsdom
// installed next to it, and node --conditions=browser).
import { copyFileSync, readdirSync, readFileSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'
import { JSDOM } from 'jsdom'

const dir = process.argv[2]
let failed = 0
const ok = (cond, what) => {
  console.log(`${cond ? 'pass' : 'FAIL'}  ${what}`)
  if (!cond) failed++
}

// --- every component compiles ---------------------------------------------
const { compile } = await import('svelte/compiler')
const css = {}
for (const f of readdirSync(dir).filter((f) => f.endsWith('.svelte'))) {
  let res
  try {
    res = compile(readFileSync(join(dir, f), 'utf8'), { filename: f, generate: 'client' })
  } catch (e) {
    ok(false, `compile ${f}: ${e.message}`)
    continue
  }
  // Zero warnings (stormcentral#193): hosts turn Svelte warnings into errors,
  // so a warning here is a failure (#15).
  for (const w of res.warnings) ok(false, `${f}:${w.start?.line ?? '?'}: ${w.code} ${w.message.split('\n')[0]}`)
  ok(true, `compile ${f}`)
  css[f] = res.css?.code ?? ''
  // flat beside each other here: ./X.svelte → ./X.js, ../utils.js → ./utils.js
  const code = res.js.code
    .replace(/from '\.\/(\w+)\.svelte'/g, "from './$1.js'")
    .replace(/from '\.\.\/utils\.js'/g, "from './utils.js'")
  writeFileSync(f.replace(/\.svelte$/, '.js'), code)
}
copyFileSync(join(dir, '..', 'utils.js'), 'utils.js')

// --- HealthDot's glow comes from the dot's token (#6) -----------------------
{
  const c = css['HealthDot.svelte'] ?? ''
  ok(!/rgba?\(|#[0-9a-f]{3,8}\b/i.test(c), 'HealthDot: no fixed colours in its CSS')
  for (const [state, token] of [['ok', '--ok'], ['warn', '--warn-strong'], ['error', '--error']]) {
    const rule = c.match(new RegExp(`\\.dot\\.${state}[^{]*\\{([^}]*)\\}`))?.[1] ?? ''
    ok(rule.includes(`background: var(${token})`) && rule.includes(`color-mix(in srgb, var(${token})`),
      `HealthDot: ${state} glow is mixed from ${token}, like its fill`)
  }
}

// --- LoginPanel, driven ---------------------------------------------------
const dom = new JSDOM('<!doctype html><body></body>', { pretendToBeVisual: true })
// every DOM global the svelte client reaches for (Text, Comment, Node, …)
globalThis.window = dom.window
for (const k of Object.getOwnPropertyNames(dom.window)) {
  if (k in globalThis) continue
  try {
    globalThis[k] = dom.window[k]
  } catch {}
}
const { mount, unmount, flushSync } = await import('svelte')
const load = async (f) => import(pathToFileURL(join(process.cwd(), f)).href)
const LoginPanel = (await load('LoginPanel.js')).default
const ComponentCard = (await load('ComponentCard.js')).default
const DataGridC = (await load('DataGrid.js')).default
const { actionTone, formatDuration } = await load('utils.js')

const settle = async (ms = 0) => {
  await new Promise((r) => setTimeout(r, ms))
  flushSync()
}
const $ = (s) => document.querySelector(s)
const $$ = (s) => [...document.querySelectorAll(s)]
const type = (el, v) => {
  el.value = v
  el.dispatchEvent(new dom.window.Event('input', { bubbles: true }))
  flushSync()
}
const submit = () => $('form').dispatchEvent(new dom.window.Event('submit', { cancelable: true, bubbles: true }))
const text = (s) => $(s)?.textContent.trim()

async function run(props, Component = LoginPanel) {
  const target = document.body.appendChild(document.createElement('div'))
  const app = mount(Component, { target, props })
  flushSync()
  return () => {
    unmount(app)
    target.remove()
  }
}

// 1. plain sign-in: no step returned, panel stays on the password form
{
  const seen = []
  const done = await run({ onsubmit: async (u, p) => void seen.push([u, p]) })
  type($('input[type=text]'), 'glenn')
  type($('input[type=password]'), 'pw')
  submit()
  await settle()
  ok(seen.length === 1 && seen[0][0] === 'glenn' && seen[0][1] === 'pw', 'onsubmit gets username and password')
  ok(!!$('input[type=password]') && !$('input.code'), 'no step returned: stays on the password step')
  done()
}

// 2. password error: inline, password cleared
{
  const done = await run({ onsubmit: async () => { throw new Error('bad password') } })
  type($('input[type=password]'), 'pw')
  submit()
  await settle()
  ok(text('.error') === 'bad password' && $('.error.visible'), 'password error shows inline')
  ok($('input[type=password]').value === '', 'password clears on error')
  done()
}

// 3. enrolment: QR, show-key toggle, auto-submit, error, restart, back
{
  const codes = []
  let reply = async () => {}
  const done = await run({
    onsubmit: async () => ({ step: 'enroll', qr: 'data:image/png;base64,AAAA', secret: 'JBSWY3DPEHPK3PXPJBSW' }), // not a secret: test fixture (RFC sample TOTP key, "Hello!")
    oncode: async (c, s) => {
      codes.push([c, s])
      return reply()
    },
  })
  type($('input[type=password]'), 'pw')
  submit()
  await settle()
  ok(text('.title') === 'Set up your authenticator', 'enroll step shows its title')
  ok($('img.qr')?.getAttribute('src') === 'data:image/png;base64,AAAA', 'enroll step shows the QR image')
  ok(!$('code.secret'), 'key hidden until asked')
  $$('button.link')[0].click()
  flushSync()
  ok(text('code.secret') === 'JBSW Y3DP EHPK 3PXP JBSW', 'show the key: grouped in fours')
  ok($('.panel.wide'), 'enroll panel is wide')

  type($('input.code'), '12a34')
  ok($('input.code').value === '1234' && codes.length === 0, 'code field keeps digits only, waits for six')
  ok($('button[type=submit]').disabled, 'submit disabled under six digits')
  type($('input.code'), '1234567')
  await settle()
  ok(codes.length === 1 && codes[0][0] === '123456' && codes[0][1] === 'enroll', 'six digits auto-submit oncode(code, "enroll")')

  reply = async () => { throw new Error('wrong code') }
  type($('input.code'), '000000')
  await settle()
  ok(text('.error') === 'wrong code', 'code error shows inline')
  ok($('input.code').value === '' && !!$('input.code'), 'code clears, stays on the code step')

  reply = async () => { throw Object.assign(new Error('ticket expired — start again'), { restart: true }) }
  type($('input.code'), '111111')
  await settle()
  ok(!!$('input.code'), 'restart error: still on the code step at first')
  await settle(1600)
  ok(!!$('input[type=password]') && !$('input.code'), 'restart error: back to the password step after 1.5s')

  type($('input[type=password]'), 'pw')
  submit()
  await settle()
  ok(!!$('input.code'), 'password again: enroll step again')
  $$('button.link').at(-1).click()
  flushSync()
  ok(!!$('input[type=password]') && !$('input.code'), '← back returns to the password step')
  done()
}

// 4. totp step
{
  const codes = []
  const done = await run({
    askUsername: false,
    onsubmit: async () => ({ step: 'totp' }),
    oncode: async (c, s) => void codes.push([c, s]),
  })
  type($('input[type=password]'), 'pw')
  submit()
  await settle()
  ok(text('.title') === 'Authenticator code' && !$('img.qr') && !$('.panel.wide'), 'totp step: code only, no QR')
  type($('input.code'), '654321')
  await settle()
  ok(codes.length === 1 && codes[0][0] === '654321' && codes[0][1] === 'totp', 'oncode(code, "totp")')
  done()
}

// 5. remember this device (#22)
{
  // unset: nothing shown on either code step, oncode gets no third value
  const calls = []
  const done = await run({
    onsubmit: async () => ({ step: 'enroll', qr: 'data:image/png;base64,AAAA', secret: 'JBSWY3DPEHPK3PXP' }), // test fixture, not a secret
    oncode: async (...a) => void calls.push(a),
  })
  ok(!$('.remember'), 'remember unset: nothing on the password step')
  type($('input[type=password]'), 'pw')
  submit()
  await settle()
  ok(!$('.remember') && !$('form input[type=checkbox]'), 'remember unset: no checkbox on the enroll step')
  type($('input.code'), '123456')
  await settle()
  ok(calls.length === 1 && calls[0][2] === undefined, `remember unset: oncode's third argument is undefined (got ${calls[0]?.[2]})`)
  done()
}
{
  const rule = (css['LoginPanel.svelte'] ?? '').match(/\.remember\.svelte-[\w-]+\s*\{([^}]*)\}/)?.[1] ?? ''
  ok(/min-height:\s*44px/.test(rule), 'remember: the row is touch-sized (min-height: 44px)')
}
for (const [step, checked] of [['totp', true], ['enroll', true], ['totp', false], ['enroll', false]]) {
  const calls = []
  const label = 'remember this device for 30 days'
  const done = await run({
    remember: { label, checked },
    onsubmit: async () => (step === 'totp' ? { step } : { step, qr: 'data:image/png;base64,AAAA', secret: 'JBSWY3DPEHPK3PXP' }),
    oncode: async (...a) => {
      calls.push(a)
      if (calls.length === 1) throw new Error('wrong code')
    },
  })
  ok(!$('.remember'), `remember ${step}/${checked}: not on the password step`)
  type($('input[type=password]'), 'pw')
  submit()
  await settle()
  const box = () => $('.remember input[type=checkbox]')
  ok(!!box() && text('.remember') === label, `remember ${step}/${checked}: checkbox with its label on the ${step} step`)
  ok(box()?.checked === checked, `remember ${step}/${checked}: starts at ${checked}`)
  // left as it starts, then flipped by the user: oncode gets what is shown
  type($('input.code'), '111111')
  await settle()
  ok(calls[0]?.[2] === checked, `remember ${step}/${checked}: oncode(code, step, ${checked}) as left (got ${calls[0]?.[2]})`)
  box().click()
  flushSync()
  type($('input.code'), '222222')
  await settle()
  ok(calls[1]?.[1] === step && calls[1]?.[2] === !checked, `remember ${step}/${checked}: flipped, oncode gets ${!checked} (got ${calls[1]?.[2]})`)
  // back and in again: starts at `checked` once more
  $$('button.link').at(-1).click()
  flushSync()
  type($('input[type=password]'), 'pw')
  submit()
  await settle()
  ok(box()?.checked === checked, `remember ${step}/${checked}: a new code step starts at ${checked} again`)
  done()
}

// --- action tones (#4) ------------------------------------------------------
const act = (id, extra = {}) => ({ id, label: id, method: 'POST', path: `/api/v1/x/${id}`, enabled: true, danger: false, ...extra })
const toned = [
  act('golden', { tone: 'ok' }),
  act('retry', { tone: 'warn' }),
  act('open', { tone: 'accent' }),
  act('rebuild', { tone: 'muted' }),
  act('stop', { danger: true, tone: 'ok' }),
  act('weird', { tone: 'purple' }),
  act('start'),
  act('restart'),
  act('start2', { id: 'start', label: 'start2', tone: 'muted' }),
]
const want = { golden: 'ok', retry: 'warn', open: 'accent', rebuild: 'muted', stop: 'danger', weird: '' }

ok(actionTone({ tone: 'accent' }) === 'accent' && actionTone({ danger: true, tone: 'ok' }) === 'danger'
  && actionTone({ tone: 'nope' }) === '' && actionTone(undefined) === '', 'actionTone: danger wins, unknown tones are plain')

// formatDuration matches format_duration in src/lib.rs, negatives clamped (#5).
for (const [secs, want] of [[42, '42s'], [90, '1m 30s'], [3700, '1h 1m'], [90000, '1d 1h'],
  [0, '0s'], [-5, '0s'], [-90, '0s'], [-90000, '0s'], [null, '-']]) {
  const got = formatDuration(secs)
  ok(got === want, `formatDuration(${secs}) = ${JSON.stringify(got)} (want ${JSON.stringify(want)})`)
}

const btnClass = (label) => {
  const b = $$('button').find((b) => b.textContent.trim() === label)
  return b ? [...b.classList].filter((c) => !c.startsWith('svelte-')).join(' ') : '(missing)'
}
{
  const done = await run({ component: { id: 'c1', kind: 'process', label: 'c1', health: 'ok', actions: toned } }, ComponentCard)
  for (const [label, cls] of Object.entries(want)) ok(btnClass(label) === cls, `ComponentCard: ${label} → '${cls}' (got '${btnClass(label)}')`)
  ok(btnClass('start') === 'ok' && btnClass('restart') === 'warn', 'ComponentCard: no tone falls back to the id (start → ok, restart → warn)')
  ok(btnClass('start2') === 'muted', 'ComponentCard: a tone beats the id fallback')
  done()
}
{
  const done = await run({ columns: [{ key: 'actions', label: '', render: 'actions' }], rows: [{ id: 'r1', actions: toned }] }, DataGridC)
  for (const [label, cls] of Object.entries(want)) ok(btnClass(label) === cls, `DataGrid: ${label} → '${cls}' (got '${btnClass(label)}')`)
  ok(btnClass('start') === '', 'DataGrid: no id fallback (as before)')
  done()
}

// --- clicks inside the actions / select cells don't click the row (#15) ------
{
  const rowClicks = []
  const actions = []
  const done = await run({
    columns: [{ key: 'name', label: 'Name' }, { key: 'actions', label: '', render: 'actions' }],
    rows: [{ id: 'r1', name: 'one', actions: [act('restart')] }],
    selectable: true,
    onrowclick: (r) => rowClicks.push(r.id),
    onaction: (r, a) => actions.push([r.id, a.id]),
  }, DataGridC)
  $$('button').find((b) => b.textContent.trim() === 'restart').click()
  flushSync()
  ok(actions.length === 1 && actions[0][0] === 'r1' && actions[0][1] === 'restart', 'DataGrid: action button calls onaction(row, action)')
  ok(rowClicks.length === 0, `DataGrid: action button does not click the row (got ${rowClicks.length})`)
  $('tbody input[type=checkbox]').click()
  flushSync()
  ok(rowClicks.length === 0, `DataGrid: select checkbox does not click the row (got ${rowClicks.length})`)
  $$('tbody td').find((td) => td.textContent.trim() === 'one').click()
  flushSync()
  ok(rowClicks.length === 1 && rowClicks[0] === 'r1', 'DataGrid: a plain cell still clicks the row')
  done()
}

// --- select-all is scoped to its own grid's rows (#12) ----------------------
{
  // Top grid: two shelves, each shelf's drives in a nested grid, one shared
  // `selected` (bind:selected between levels). Read the selection back from
  // the rows' `selected` class.
  const drives = (s, n) => Array.from({ length: n }, (_, i) => ({ id: `${s}${i}`, name: `${s}${i}` }))
  const shelves = { A: drives('a', 3), B: drives('b', 4) }
  const done = await run({
    columns: [{ key: 'name', label: 'Name' }],
    rows: [{ id: 'A', name: 'A' }, { id: 'B', name: 'B' }],
    getChildren: (r) => [{ title: 'drives', rows: shelves[r.id], getChildren: null }],
    selectable: 'multi',
  }, DataGridC)
  $$('button.expander').forEach((b) => b.click())
  flushSync()
  const grids = () => $$('table') // [top, A, B]
  const head = (g) => grids()[g].querySelector('thead input[type=checkbox]')
  const nameOf = (tr) => tr.querySelector(':scope > td:not(.ctl)')?.textContent.trim()
  const box = (id) => $$('tbody tr').find((tr) => nameOf(tr) === id)?.querySelector(':scope > td.ctl input[type=checkbox]')
  const sel = () => $$('tbody tr.selected').map(nameOf).sort().join(',')
  const click = (el) => {
    el.click()
    flushSync()
  }

  ok(grids().length === 3, `select-all: top grid and two nested grids (got ${grids().length})`)
  for (const id of ['a0', 'a1', 'a2']) click(box(id))
  ok(sel() === 'a0,a1,a2', `select-all: three drives on A ticked (got ${sel()})`)
  ok(head(1).checked && !head(1).indeterminate, 'select-all: A header checked when all of A is')
  ok(!head(2).checked && !head(2).indeterminate, 'select-all: B header unchecked with none of B ticked')
  ok(!head(0).checked && !head(0).indeterminate, 'select-all: top header unchecked with no shelf ticked')

  click(head(2))
  ok(sel() === 'a0,a1,a2,b0,b1,b2,b3', `select-all on B adds B's drives, keeps A's (got ${sel()})`)
  ok(head(2).checked && head(1).checked, 'select-all: A and B headers both checked')

  click(box('b1'))
  ok(!head(2).checked && head(2).indeterminate, 'select-all: B header indeterminate when some of B is ticked')
  click(head(2))
  ok(sel() === 'a0,a1,a2,b0,b1,b2,b3', `select-all from indeterminate selects all of B (got ${sel()})`)
  click(head(2))
  ok(sel() === 'a0,a1,a2', `select-all off on B removes only B's drives (got ${sel()})`)

  // 3 ticked elsewhere, A has 3 rows: the old code showed A checked by count
  // alone. Untick one of A and tick a B drive: still 3 ticked, A not all.
  click(box('a2'))
  click(box('b0'))
  ok(!head(1).checked && head(1).indeterminate, 'select-all: A header not checked by a matching global count')

  click(head(0))
  ok(sel() === 'A,B,a0,a1,b0', `top select-all adds the shelves, keeps the drives (got ${sel()})`)
  ok(head(0).checked && !head(0).indeterminate, 'select-all: top header checked')
  click(head(0))
  ok(sel() === 'a0,a1,b0', `top select-all off removes only the shelves (got ${sel()})`)
  done()
}

// --- a section's getChildren: null is a leaf grid; absent inherits (#13) ----
{
  const tree = { r: [{ id: 'm', name: 'm' }], m: [{ id: 'l', name: 'l' }] }
  const kidsOf = (r) => (tree[r.id] ? [{ title: 'kids', rows: tree[r.id] }] : [])
  const grid = (section) => ({
    columns: [{ key: 'name', label: 'Name' }],
    rows: [{ id: 'r', name: 'r' }],
    getChildren: (r) => [{ title: 'kids', rows: tree[r.id] ?? [], ...section }],
  })
  const ctlCols = (t) => t.querySelectorAll(':scope > thead > tr > th.ctl').length

  let done = await run(grid({ getChildren: null }), DataGridC)
  $('button.expander').click()
  flushSync()
  let tables = $$('table')
  ok(tables.length === 2, `getChildren null: nested grid shown (got ${tables.length})`)
  ok(tables.length === 2 && ctlCols(tables[1]) === 0 && !tables[1].querySelector('td.ctl'),
    `getChildren null: leaf grid has no expander column (got ${tables[1] ? ctlCols(tables[1]) : '?'} th.ctl)`)
  done()

  done = await run({ ...grid({}), getChildren: kidsOf }, DataGridC)
  $('button.expander').click()
  flushSync()
  tables = $$('table')
  ok(tables.length === 2 && ctlCols(tables[1]) === 1 && !!tables[1].querySelector('button.expander'),
    "getChildren absent: nested grid inherits the parent's (expander on its row)")
  done()

  done = await run({ ...grid({ getChildren: undefined }), getChildren: kidsOf }, DataGridC)
  $('button.expander').click()
  flushSync()
  ok(!!$$('table')[1]?.querySelector('button.expander'), 'getChildren undefined: inherits too')
  done()
}

console.log(failed ? `${failed} failed` : 'all passed')
process.exit(failed ? 1 : 0)

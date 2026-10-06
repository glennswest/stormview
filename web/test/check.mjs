// Compiles every component, then mounts LoginPanel in jsdom and walks its
// password, enrol and totp steps. Run by check.sh (needs svelte + jsdom
// installed next to it, and node --conditions=browser).
import { readdirSync, readFileSync, writeFileSync } from 'node:fs'
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
for (const f of readdirSync(dir).filter((f) => f.endsWith('.svelte'))) {
  let res
  try {
    res = compile(readFileSync(join(dir, f), 'utf8'), { filename: f, generate: 'client' })
  } catch (e) {
    ok(false, `compile ${f}: ${e.message}`)
    continue
  }
  for (const w of res.warnings) console.log(`warn  ${f}: ${w.code} ${w.message.split('\n')[0]}`)
  ok(true, `compile ${f}`)
  writeFileSync(f.replace(/\.svelte$/, '.js'), res.js.code)
}

// --- LoginPanel, driven ---------------------------------------------------
const dom = new JSDOM('<!doctype html><body></body>', { pretendToBeVisual: true })
for (const k of ['window', 'document', 'navigator', 'Node', 'Element', 'HTMLElement', 'Event', 'getComputedStyle', 'requestAnimationFrame']) {
  globalThis[k] ??= dom.window[k]
}
const { mount, unmount, flushSync } = await import('svelte')
const LoginPanel = (await import(pathToFileURL(join(process.cwd(), 'LoginPanel.js')).href)).default

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

async function run(props) {
  const target = document.body.appendChild(document.createElement('div'))
  const app = mount(LoginPanel, { target, props })
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
    onsubmit: async () => ({ step: 'enroll', qr: 'data:image/png;base64,AAAA', secret: 'JBSWY3DPEHPK3PXPJBSW' }),
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

console.log(failed ? `${failed} failed` : 'all passed')
process.exit(failed ? 1 : 0)

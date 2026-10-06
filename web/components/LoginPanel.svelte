<script>
  // The login screen, as a reusable panel: the host passes the instance name
  // and an async onsubmit(username, password); errors render inline. Fully
  // token-driven, so it wears whatever theme the page has. Set
  // askUsername={false} for password-only hosts.
  //
  // An optional second step: onsubmit may resolve to { step: 'totp' } (ask
  // for an authenticator code) or { step: 'enroll', qr, secret } (show the
  // QR image and key, then ask for a code to prove it). The code goes to
  // oncode(code, step); a thrown error shows inline like the password's, and
  // one with `restart: true` (say, an expired ticket) returns to the password
  // step. Any other resolved value leaves the panel as it is.
  import { tick } from 'svelte'

  let {
    title = 'storm',
    subtitle = 'sign in to continue',
    askUsername = true,
    onsubmit,
    oncode = null,
  } = $props()

  let step = $state('password') // password | totp | enroll
  let enrol = $state(null) // { qr, secret }
  let username = $state('')
  let password = $state('')
  let code = $state('')
  let showSecret = $state(false)
  let error = $state('')
  let busy = $state(false)
  let shake = $state(false)
  let codeInput = $state(null)

  function fail(err, fallback) {
    error = err?.message || fallback
    shake = true
    setTimeout(() => (shake = false), 450)
  }

  async function submit(e) {
    e.preventDefault()
    if (!password || busy) return
    busy = true
    error = ''
    try {
      const r = await onsubmit(username, password)
      if (r?.step === 'totp' || r?.step === 'enroll') {
        enrol = r.step === 'enroll' ? { qr: r.qr, secret: r.secret } : null
        showSecret = false
        code = ''
        password = ''
        step = r.step
      }
    } catch (err) {
      password = ''
      fail(err, 'login failed')
    } finally {
      busy = false
    }
  }

  async function submitCode(e) {
    e?.preventDefault()
    if (busy || code.length !== 6) return
    busy = true
    error = ''
    try {
      await oncode(code, step)
    } catch (err) {
      code = ''
      fail(err, 'code not accepted')
      if (err?.restart) setTimeout(back, 1500)
    } finally {
      busy = false
    }
    // the input was disabled while busy, which drops focus; give it back
    await tick()
    if (step !== 'password') codeInput?.focus()
  }

  function onCodeInput(e) {
    code = e.currentTarget.value.replace(/\D/g, '').slice(0, 6)
    e.currentTarget.value = code
    if (code.length === 6) submitCode()
  }

  function back() {
    step = 'password'
    enrol = null
    code = ''
    error = ''
  }

  function grouped(s) {
    return (s || '').replace(/(.{4})/g, '$1 ').trim()
  }
</script>

<div class="wrap">
  {#if step === 'password'}
    <form class="panel" class:shake onsubmit={submit}>
      <div class="glyph">⛈</div>
      <div class="title">{title}</div>
      <div class="subtitle">{subtitle}</div>
      {#if askUsername}
        <!-- svelte-ignore a11y_autofocus -->
        <input
          type="text"
          placeholder="User"
          bind:value={username}
          autofocus
          autocomplete="username"
          disabled={busy}
        />
        <input
          type="password"
          placeholder="Password"
          bind:value={password}
          autocomplete="current-password"
          disabled={busy}
        />
      {:else}
        <!-- svelte-ignore a11y_autofocus -->
        <input
          type="password"
          placeholder="Password"
          bind:value={password}
          autofocus
          autocomplete="current-password"
          disabled={busy}
        />
      {/if}
      <button type="submit" disabled={busy || !password}>
        {busy ? 'Signing in…' : 'Sign in'}
      </button>
      <div class="error" class:visible={!!error}>{error || ' '}</div>
    </form>
  {:else}
    <form class="panel" class:shake class:wide={step === 'enroll'} onsubmit={submitCode}>
      <div class="glyph">{step === 'enroll' ? '⚿' : '⛈'}</div>
      {#if step === 'enroll'}
        <div class="title">Set up your authenticator</div>
        <div class="subtitle">required for every account</div>
        <p class="lead">
          Scan this with an authenticator app (1Password, Google Authenticator, Authy, a YubiKey…),
          then enter the 6-digit code it shows.
        </p>
        <img class="qr" src={enrol?.qr} alt="authenticator QR code" />
        <button type="button" class="link" onclick={() => (showSecret = !showSecret)}>
          {showSecret ? 'hide the key' : "can't scan? show the key"}
        </button>
        {#if showSecret}
          <code class="secret">{grouped(enrol?.secret)}</code>
        {/if}
      {:else}
        <div class="title">Authenticator code</div>
        <div class="subtitle">second step</div>
      {/if}
      <!-- svelte-ignore a11y_autofocus -->
      <input
        class="code"
        bind:this={codeInput}
        inputmode="numeric"
        autocomplete="one-time-code"
        aria-label="6-digit code"
        placeholder="000000"
        value={code}
        oninput={onCodeInput}
        autofocus
        disabled={busy}
      />
      <button type="submit" disabled={busy || code.length !== 6}>
        {busy ? 'Checking…' : step === 'enroll' ? 'Confirm and sign in' : 'Sign in'}
      </button>
      <div class="error" class:visible={!!error}>{error || ' '}</div>
      <button type="button" class="link" onclick={back} disabled={busy}>← back</button>
    </form>
  {/if}
</div>

<style>
  .wrap {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    background:
      radial-gradient(ellipse 60% 45% at 50% 0%, color-mix(in srgb, var(--accent) 7%, transparent), transparent),
      var(--bg);
    padding: 20px;
  }
  .panel {
    position: relative;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 14px;
    box-shadow: var(--shadow);
    padding: 40px 36px 28px;
    width: min(340px, 92vw);
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 14px;
    text-align: center;
    overflow: hidden;
  }
  .panel.wide { width: min(400px, 94vw); }
  /* a thin brand-to-accent thread across the top of the card */
  .panel::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: linear-gradient(90deg, var(--brand), var(--accent));
  }
  .glyph {
    width: 58px;
    height: 58px;
    margin: 0 auto 2px;
    display: grid;
    place-items: center;
    font-size: 26px;
    border-radius: 50%;
    background: var(--panel-raised);
    border: 1px solid var(--border-strong);
    color: var(--accent);
  }
  .title {
    font-size: 22px;
    font-weight: 700;
    letter-spacing: -0.4px;
    color: var(--text);
  }
  .subtitle {
    font-size: 12px;
    color: var(--text-faint);
    text-transform: uppercase;
    letter-spacing: 1.2px;
    margin-bottom: 8px;
  }
  .lead {
    margin: 0;
    color: var(--text-dim);
    font-size: 12.5px;
    line-height: 1.5;
  }
  /* a QR code needs a light quiet zone whatever the theme */
  .qr {
    width: 200px;
    height: 200px;
    margin: 4px auto;
    border-radius: 8px;
    background: #fff;
    padding: 8px;
    image-rendering: pixelated;
  }
  .secret {
    font-family: var(--mono);
    font-size: 12px;
    color: var(--text);
    background: var(--panel-raised);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 8px;
    word-break: break-all;
    user-select: all;
  }
  input {
    background: var(--panel-raised);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 11px 14px;
    font-size: 14px;
    text-align: center;
    outline: none;
    transition: border-color 0.15s, box-shadow 0.15s;
    font-family: var(--font);
  }
  input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 18%, transparent);
  }
  input.code {
    font-family: var(--mono);
    font-size: 26px;
    letter-spacing: 10px;
    padding: 10px 12px 10px 22px;
  }
  button {
    padding: 11px 14px;
    font-size: 14px;
    font-weight: 600;
    border-radius: var(--radius-sm);
    background: var(--accent-bg);
    border: 1px solid var(--border-strong);
    color: var(--accent);
    cursor: pointer;
    transition: filter 0.15s;
    font-family: var(--font);
  }
  button:hover:not(:disabled) { filter: brightness(1.2); }
  button:disabled { opacity: 0.45; cursor: default; }
  button.link {
    background: none;
    border: none;
    color: var(--text-faint);
    font-size: 12px;
    font-weight: 400;
    padding: 2px;
  }
  button.link:hover:not(:disabled) { color: var(--accent); filter: none; }
  .error {
    font-size: 12px;
    color: var(--error);
    min-height: 16px;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .error.visible { opacity: 1; }

  .shake { animation: shake 0.4s; }
  @keyframes shake {
    20% { transform: translateX(-7px); }
    45% { transform: translateX(6px); }
    70% { transform: translateX(-4px); }
    90% { transform: translateX(2px); }
  }
</style>

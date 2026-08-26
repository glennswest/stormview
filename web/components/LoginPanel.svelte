<script>
  // The login screen, as a reusable panel: the host passes the instance name
  // and an async onsubmit(password); errors render inline. Fully token-
  // driven, so it wears whatever theme the page has.
  let { title = 'storm', subtitle = 'sign in to continue', onsubmit } = $props()

  let password = $state('')
  let error = $state('')
  let busy = $state(false)
  let shake = $state(false)

  async function submit(e) {
    e.preventDefault()
    if (!password || busy) return
    busy = true
    error = ''
    try {
      await onsubmit(password)
    } catch (err) {
      error = err?.message || 'login failed'
      password = ''
      shake = true
      setTimeout(() => (shake = false), 450)
    } finally {
      busy = false
    }
  }
</script>

<div class="wrap">
  <form class="panel" class:shake onsubmit={submit}>
    <div class="glyph">⛈</div>
    <div class="title">{title}</div>
    <div class="subtitle">{subtitle}</div>
    <!-- svelte-ignore a11y_autofocus -->
    <input
      type="password"
      placeholder="Password"
      bind:value={password}
      autofocus
      autocomplete="current-password"
      disabled={busy}
    />
    <button type="submit" disabled={busy || !password}>
      {busy ? 'Signing in…' : 'Sign in'}
    </button>
    <div class="error" class:visible={!!error}>{error || ' '}</div>
  </form>
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
  /* a thin brand-to-accent thread across the top of the card */
  .panel::before {
    content: '';
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: linear-gradient(90deg, var(--brand), var(--purple), var(--accent));
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

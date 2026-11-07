<script>
  let { health = 'unknown', size = 10 } = $props()
</script>

<span
  class="dot {health}"
  style="width:{size}px;height:{size}px"
  title={health}
></span>

<style>
  /* Glow is the dot's own token, faded — no fixed colours, so every theme's
     halo matches its dot (#6). */
  .dot {
    display: inline-block;
    border-radius: 50%;
    flex-shrink: 0;
  }
  .dot.ok { background: var(--ok); box-shadow: 0 0 6px color-mix(in srgb, var(--ok) 50%, transparent); }
  .dot.warn { background: var(--warn-strong); box-shadow: 0 0 6px color-mix(in srgb, var(--warn-strong) 50%, transparent); }
  .dot.error { background: var(--error); box-shadow: 0 0 6px color-mix(in srgb, var(--error) 60%, transparent); animation: pulse 1.2s infinite; }
  .dot.idle { background: var(--text-ghost); }
  .dot.unknown { background: var(--text-ghost); opacity: 0.5; }

  @keyframes pulse {
    50% { opacity: 0.4; }
  }
</style>

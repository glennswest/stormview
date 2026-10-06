// A minimal host app, built by check.sh with vite + vite-plugin-svelte
// against the packed stormview: the root export, a subpath and a component
// must all resolve and compile the way a storm web UI's build does them.
import 'stormview/themes.css'
import { initTheme, formatBytes } from 'stormview'
import { actionTone } from 'stormview/utils'
import { mount } from 'svelte'
import HealthDot from 'stormview/components/HealthDot.svelte'

initTheme()
mount(HealthDot, { target: document.getElementById('app'), props: { health: 'healthy' } })
console.log(formatBytes(1024), actionTone({ id: 'start' }))

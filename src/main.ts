import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'
import { applyTheme, cachedTheme } from './lib/theme'

// Apply the last-known theme before first paint — flash-free; bootstrap()
// reconciles with the persisted config once it loads.
applyTheme(cachedTheme())

const app = mount(App, {
  target: document.getElementById('app')!,
})

export default app

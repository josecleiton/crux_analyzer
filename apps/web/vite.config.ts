import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { cspPlugin } from './csp.js'

// A static documentation site is a first-class deployment of this app
// (`just site <src> <name> [base]`), and such a site is often served from a
// subpath — `https://org.gitlab.io/crux-docs/` — where root-absolute asset
// URLs would 404. `CRUX_BASE` is that prefix; `import.meta.env.BASE_URL`
// carries it into the app, so `model.json` is fetched from the same place.
const base = normalizeBase(process.env.CRUX_BASE)

/** Vite requires a base with both a leading and a trailing slash. */
function normalizeBase(raw: string | undefined): string {
  const trimmed = raw?.trim()
  if (!trimmed || trimmed === '/') return '/'
  // A full origin (https://cdn.example.com/docs/) is valid too — leave the
  // scheme alone and only guarantee the trailing slash.
  if (/^https?:\/\//.test(trimmed)) return trimmed.endsWith('/') ? trimmed : `${trimmed}/`
  return `/${trimmed.replace(/^\/+/, '').replace(/\/+$/, '')}/`
}

// https://vite.dev/config/
export default defineConfig({
  base,
  plugins: [react(), cspPlugin()],
})

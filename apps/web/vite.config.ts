import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { cspPlugin } from './csp.js'
import { noticesPlugin } from './notices.js'

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
  plugins: [react(), cspPlugin(), noticesPlugin()],
  build: {
    rolldownOptions: {
      output: {
        // Dependencies that ship a legal header keep it. React's
        // `react.production.js` carries `@license React … Copyright (c) Meta
        // Platforms`, and the minifier's default is to drop it — which meant
        // this bundle redistributed MIT code with the copyright notice removed.
        // `THIRD-PARTY-NOTICES.md` covers the packages that ship no header;
        // this covers the ones that do, in the artifact itself.
        comments: { legal: true },
        codeSplitting: {
          groups: [
            {
              // elkjs is EPL-2.0 (we elect it over GPL-3.0-or-later) and 82% of
              // the bundle. Its own chunk keeps EPL-licensed code out of a file
              // that also contains ours — so EPL-2.0's "any new file that
              // contains any contents of the Program" never has to be argued —
              // and takes the layout engine out of the entry chunk, which is
              // what the 500 kB warning was about. See docs/security.md.
              name: 'elk',
              test: /[\\/]node_modules[\\/]elkjs[\\/]/,
            },
          ],
        },
      },
    },
  },
})

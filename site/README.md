# browserlane landing page (`site/`)

The marketing landing page for **browserlane.com** — slogan: **"Agentic
browser testing and debugging for real web apps."**

This is a standalone Next.js app, deliberately separate from `website/`
(the Fumadocs docs site that serves docs.browserlane.com, which stays a pure
docs site). It is the Next.js successor to the static repo-root `index.html`.

## Run locally

```bash
cd site
pnpm install
pnpm dev        # http://localhost:3100
```

Other scripts: `pnpm build`, `pnpm start`, `pnpm lint`, `pnpm typecheck`.

## Deploy (Vercel)

Create a Vercel project with **Root Directory = `site`** — `vercel.json`
pins the Next.js framework preset, pnpm frozen-lockfile install, and
`pnpm build`. Environment variables:

| Variable | Purpose |
|---|---|
| `RESEND_API_KEY` | Resend API key for the newsletter form |
| `RESEND_AUDIENCE_ID` | Resend audience that collects subscribers |

Without them the `/api/subscribe` endpoint returns a friendly 503 (the form
tells visitors to email bl@browserlane.com instead), so deploying before
Resend is configured is safe. GitHub stars are fetched server-side with a
1-hour ISR revalidate and degrade to no-count if the API is unreachable.

Newsletter abuse posture: the endpoint checks same-origin and a honeypot,
but script-driven signup abuse should be handled at the platform layer —
add a Vercel WAF rate-limit rule for `POST /api/subscribe`, and enable
double opt-in on the Resend audience so bombed addresses never get added.

**Install one-liners keep working after the domain cutover.** The repo-root
`install.sh`/`install.ps1` (canonical copies, also served by GitHub Pages
today) are synced into `public/` by `scripts/sync-install-scripts.mjs` on
every dev/build, so this deployment serves
`browserlane.com/install.sh` and `/install.ps1` itself. This requires the
Vercel project setting "Include source files outside of the Root Directory
in the Build Step" to stay ON (the default) — the sync script fails the
build loudly if it can't find the sources. The installers and `bl update`
download binaries from GitHub Releases, so nothing else depends on the old
GitHub Pages site; once the Vercel domain is live, the repo-root
`index.html` + `CNAME` (GitHub Pages) can be retired.

## Stack

- Next.js (App Router) + TypeScript
- Tailwind CSS v4 — brand tokens live in `app/globals.css` (`@theme`)
- [`motion`](https://motion.dev) (`motion/react`) for the scroll-linked
  layer animations; everything respects `prefers-reduced-motion`
- Geist Sans/Mono via the `geist` package (self-hosted, no network fonts)

## Theming (system / light / dark)

Three-way toggle in the nav (`components/ui/theme-toggle.tsx`), persisted
to `localStorage('bl-theme')`; an inline script in `app/layout.tsx` applies
the `.dark` class before first paint (no flash). "System" follows
`prefers-color-scheme` live.

Two token layers in `app/globals.css`:

- **Raw brand palette** (`ink`, `slate`, `edge`, `cloud*`, `ivory*`, `clay`,
  `kraft`, `manilla`, `focus`, `danger`) — theme-independent. Used by the
  product visuals (browser frame, terminals, code panels), which stay dark
  in both modes, like code blocks in docs.
- **Semantic tokens** (`canvas`, `card`, `line`, `fg`, `muted`, `dim`,
  `faint`) — flip with the theme via `--bl-*` CSS variables on
  `:root`/`.dark`. Used by all page chrome (nav, copy, cards, footer).

Rule of thumb when editing: page text/surfaces → semantic tokens; anything
inside a terminal/browser visual → raw palette.

## Structure

```
app/
  layout.tsx            fonts + metadata + pre-paint theme script
  page.tsx              section assembly + GitHub stars fetch
  globals.css           Tailwind + brand tokens (raw + semantic)
  api/subscribe/        newsletter endpoint (Resend Contacts API)
components/
  site-nav.tsx          fixed top nav (links, stars, theme toggle)
  hero.tsx              slogan + headline + browser/CLI/MCP product visual
  trust-strip.tsx       four-claims strip under the hero
  two-surfaces.tsx      CLI vs MCP side-by-side
  observability.tsx     debugging/evidence artifact cards
  quickstart.tsx        install commands (#install anchor)
  newsletter-form.tsx   Resend signup form (honeypot + aria-live status)
  site-footer.tsx       newsletter strip, link columns, Connect (GitHub/
                        LinkedIn/bl@browserlane.com)
  logo.tsx              brand mark (copied from website/components)
  layer-story/
    layers.tsx          the 8 layers' copy + verified `bl` commands
    layer-scroll-story.tsx   sticky scroll orchestration (IntersectionObserver)
    browser-stack-visual.tsx the morphing browser "product object"
  ui/                   CTAButton, TerminalPanel, SectionHeading,
                        CopyButton, ThemeToggle
lib/
  github.ts             starred-count fetch (ISR 1h) + formatting
```

## The scroll story

Desktop: the copy steps scroll in the left column while
`BrowserStackVisual` sits in a `position: sticky` right column; an
IntersectionObserver band around the viewport center picks the active step,
which drives the visual's `layer` prop. Mobile: no pinning — each step
renders its own static visual, so the narrative survives with animations
(or JavaScript) disabled.

Every command shown on the page is real — verified against `bl --help`
output at v0.1.3. If commands change, update `components/layer-story/layers.tsx`
and the panels in `hero.tsx` / `two-surfaces.tsx` / `observability.tsx` /
`quickstart.tsx`.

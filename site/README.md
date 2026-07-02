# browserlane landing page (`site/`)

The marketing landing page for **browserlane.com** — "agentic browser testing
and debugging for real web apps."

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

## Stack

- Next.js (App Router) + TypeScript
- Tailwind CSS v4 — brand tokens live in `app/globals.css` (`@theme`)
- [`motion`](https://motion.dev) (`motion/react`) for the scroll-linked
  layer animations; everything respects `prefers-reduced-motion`
- Geist Sans/Mono via the `geist` package (self-hosted, no network fonts)

## Brand tokens

The palette is the Anthropic palette (browserlane's official brand):
Book Cloth `#CC785C` accent (`clay`), Kraft `#D4A27F` hover, Slate darks
(`ink`/`slate`/`edge`), Cloud grays, Ivory lights, Focus blue `#61AAF2`,
Error `#BF4D43` (`danger`). Utilities like `bg-ink`, `text-clay`,
`border-edge` come from the `@theme` block in `app/globals.css`.

## Structure

```
app/
  layout.tsx            fonts + metadata
  page.tsx              section assembly
  globals.css           Tailwind + brand tokens
components/
  site-nav.tsx          fixed top nav
  hero.tsx              headline + browser/CLI/MCP product visual
  trust-strip.tsx       four-claims strip under the hero
  two-surfaces.tsx      CLI vs MCP side-by-side
  observability.tsx     debugging/evidence artifact cards
  quickstart.tsx        install commands (#install anchor)
  site-footer.tsx
  logo.tsx              brand mark (copied from website/components)
  layer-story/
    layers.tsx          the 8 layers' copy + verified `bl` commands
    layer-scroll-story.tsx   sticky scroll orchestration (IntersectionObserver)
    browser-stack-visual.tsx the morphing browser "product object"
  ui/                   CTAButton, TerminalPanel, SectionHeading, CopyButton
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

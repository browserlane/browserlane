# Deploying the browserlane docs (maintainer handoff)

This is an internal checklist for the repo owner — **not** a published docs page.
The site is a Fumadocs (Next.js) app that lives in `website/`. The CLI/MCP
reference under `content/docs/{cli-reference,mcp-reference}` is **auto-generated**
from the `bl` binary by `scripts/gen-reference.mjs` (it runs as `prebuild`, so it
re-generates before every `next build`).

The generator resolves the binary in this order:

1. `BL_BIN` (absolute path)
2. `../target/release/bl` (local cargo build)
3. `bl` on `PATH`
4. **Download a release binary** for the current platform — detects OS/arch,
   resolves which release to fetch (see below), downloads
   `bl-<tag>-<target>.tar.gz` from
   `github.com/browserlane/browserlane/releases/download/<tag>/…`,
   verifies it against `SHA256SUMS`, caches it under `website/.bl-cache/`
   (gitignored), and `chmod +x`'s it.

Step 4 is what makes the **Vercel build work without a local binary**. By default
it builds against the **latest published release** (resolved from the GitHub API
at build time), so the docs track new `bl` releases with no version management.
Set `BL_VERSION=vX.Y.Z` only if you want to pin the docs to a specific tag.

---

## One-time: create the Vercel project (your account actions)

These are dashboard steps only you can do:

1. **New Project** → import `github.com/browserlane/browserlane`.
2. **Root Directory** → set to `website` (the app is not at the repo root).
   Vercel will pick up `website/vercel.json` (framework `nextjs`, pnpm install,
   `pnpm build`).
3. **Environment Variables** (Project → Settings → Environment Variables):
   - **Leave `BL_VERSION` unset** — the generator then builds against the latest
     published release, so the docs always track the newest `bl` automatically.
     Set `BL_VERSION=vX.Y.Z` only if you ever want to pin the docs to a specific
     release.
4. **Deploy.** The first build runs `pnpm build` → `prebuild` downloads the
   Linux `bl` asset, regenerates the reference, then `next build` (webpack).

> The `--webpack` flag in `package.json`'s `build` script is intentional —
> Next 16's default Turbopack can't load the fumadocs-mdx loaders yet. Don't
> remove it.

## One-time: custom domain (your account actions)

1. Project → **Settings → Domains** → add `docs.browserlane.com`.
2. At your DNS provider, add the record Vercel shows — a **CNAME** for
   `docs` → `cname.vercel-dns.com` (Vercel will display the exact target).
3. Wait for DNS + TLS to provision. The site already advertises this canonical
   host: `metadataBase`, `sitemap.ts`, and `robots.ts` all point at
   `https://docs.browserlane.com`.

---

## Keeping the reference fresh on each `bl` release

The reference is regenerated on every deploy and (with `BL_VERSION` unset) always
fetches the latest release — so refreshing the docs after a `bl` release is just
**triggering a Vercel deploy**, no version bumping. Set up a Deploy Hook and ping
it from the **product repo's** release workflow.

### 1. Create a Deploy Hook (your account action)

Vercel → Project → **Settings → Git → Deploy Hooks** → create one (e.g. name
`bl-release`, branch `main`). Copy the URL it gives you and store it as a
**GitHub Actions secret** named `VERCEL_DEPLOY_HOOK` in
`github.com/browserlane/browserlane`
(Settings → Secrets and variables → Actions).

### 2. Add this workflow to the repo

> Not committed here on purpose — it needs the `VERCEL_DEPLOY_HOOK` secret, which
> only you can add. Drop this in `.github/workflows/docs-deploy.yml` yourself once
> the secret exists.

```yaml
name: Redeploy docs on release

on:
  release:
    types: [published]
  # Manual re-trigger from the Actions tab.
  workflow_dispatch:

jobs:
  ping-vercel:
    runs-on: ubuntu-latest
    steps:
      - name: Trigger Vercel deploy hook
        env:
          HOOK: ${{ secrets.VERCEL_DEPLOY_HOOK }}
        run: |
          if [ -z "$HOOK" ]; then
            echo "VERCEL_DEPLOY_HOOK secret is not set" >&2
            exit 1
          fi
          curl -fsSL -X POST "$HOOK"
          echo "Pinged Vercel deploy hook — docs will rebuild and regenerate the reference."
```

With `BL_VERSION` unset (the recommended setup), the deploy hook is all you need:
each release fires the workflow → Vercel rebuilds → the generator fetches the
newly-released `bl` and regenerates the reference. No tag bumping anywhere.

---

## Local preview

```bash
cd website
pnpm install
pnpm build      # runs the generator (uses your local ../target/release/bl) then next build
pnpm start      # serve the production build
```

To exercise the download path locally (what Vercel does), point `BL_BIN` at a
nonexistent path so the resolver falls through to the release download:

```bash
BL_BIN=/nonexistent pnpm gen:reference          # fetches the latest release
# or pin a specific tag:  BL_BIN=/nonexistent BL_VERSION=v0.1.1 pnpm gen:reference
```

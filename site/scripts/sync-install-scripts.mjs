/**
 * Copies the canonical installers (repo root) into public/ so the Vercel
 * deployment keeps serving them at the URLs baked into every install
 * one-liner once browserlane.com points here instead of GitHub Pages:
 *
 *   curl -fsSL https://browserlane.com/install.sh | sh
 *   irm https://browserlane.com/install.ps1 | iex
 *
 * Runs as predev/prebuild. Fails loudly if the sources are missing —
 * on Vercel that means "Include source files outside of the Root
 * Directory in the Build Step" got disabled (it must stay enabled).
 */
import { copyFileSync, mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const siteDir = dirname(dirname(fileURLToPath(import.meta.url)));
const repoRoot = dirname(siteDir);
const publicDir = join(siteDir, 'public');

mkdirSync(publicDir, { recursive: true });

for (const name of ['install.sh', 'install.ps1']) {
  const src = join(repoRoot, name);
  try {
    copyFileSync(src, join(publicDir, name));
    console.log(`sync-install-scripts: copied ${name}`);
  } catch (err) {
    console.error(
      `sync-install-scripts: FAILED to copy ${src} — the install one-liners ` +
        `(browserlane.com/${name}) would 404 on this deployment.`,
    );
    throw err;
  }
}

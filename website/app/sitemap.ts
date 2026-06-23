import { source } from '@/lib/source';
import type { MetadataRoute } from 'next';

const SITE_URL = 'https://docs.browserlane.com';

/**
 * Enumerate every documentation route from the Fumadocs source tree (so newly
 * generated reference pages are picked up automatically) and emit an absolute
 * sitemap entry for each, rooted at the canonical docs host.
 */
export default function sitemap(): MetadataRoute.Sitemap {
  const now = new Date();
  return source.getPages().map((page) => ({
    // page.url is a site-root-relative path like "/" or "/cli-reference/index".
    url: new URL(page.url, SITE_URL).toString(),
    lastModified: now,
    changeFrequency: 'weekly',
    priority: page.url === '/' ? 1 : 0.7,
  }));
}

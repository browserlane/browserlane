import type { MetadataRoute } from 'next';

const SITE_URL = 'https://docs.browserlane.com';

/** Allow indexing of the whole docs site and point crawlers at the sitemap. */
export default function robots(): MetadataRoute.Robots {
  return {
    rules: {
      userAgent: '*',
      allow: '/',
    },
    sitemap: `${SITE_URL}/sitemap.xml`,
    host: SITE_URL,
  };
}

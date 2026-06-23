import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';

/**
 * Shared layout options used by both the home layout and the docs layout.
 * Keeps the nav title and the GitHub link consistent across the site.
 */
export function baseOptions(): BaseLayoutProps {
  return {
    nav: {
      title: 'browserlane',
    },
    githubUrl: 'https://github.com/browserlane/browserlane',
  };
}

import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';
import { Logo } from '@/components/logo';

/**
 * Shared layout options used by both the home layout and the docs layout.
 * Keeps the nav logo/title and the GitHub link consistent across the site.
 */
export function baseOptions(): BaseLayoutProps {
  return {
    nav: {
      title: (
        <span
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: '0.5rem',
          }}
        >
          <Logo size={22} />
          <span style={{ fontWeight: 500, letterSpacing: '-0.01em' }}>
            browserlane
          </span>
        </span>
      ),
    },
    githubUrl: 'https://github.com/browserlane/browserlane',
  };
}

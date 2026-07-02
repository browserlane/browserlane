import Link from 'next/link';
import { Logo } from '@/components/logo';
import { CTAButton } from '@/components/ui/cta-button';
import { ThemeToggle } from '@/components/ui/theme-toggle';
import { GITHUB_REPO_URL, formatStars } from '@/lib/github';

const LINKS = [
  { label: 'Docs', href: 'https://docs.browserlane.com' },
  { label: 'CLI', href: 'https://docs.browserlane.com/cli-reference' },
  { label: 'MCP', href: 'https://docs.browserlane.com/mcp-reference' },
];

function StarIcon() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor" aria-hidden>
      <path d="M12 2l2.9 6.26 6.85.74-5.1 4.63 1.4 6.73L12 16.9l-6.05 3.46 1.4-6.73-5.1-4.63 6.85-.74L12 2z" />
    </svg>
  );
}

export function SiteNav({ stars }: { stars: number | null }) {
  return (
    <header className="fixed inset-x-0 top-0 z-50 border-b border-line bg-canvas/85 backdrop-blur-md">
      <div className="mx-auto flex h-16 max-w-6xl items-center justify-between gap-3 px-4 sm:px-6">
        <Link href="/" className="flex shrink-0 items-center gap-2.5">
          <Logo size={26} />
          <span className="text-[15px] font-medium tracking-tight text-fg">
            browserlane
          </span>
        </Link>
        <nav aria-label="Main" className="hidden items-center gap-6 md:flex">
          {LINKS.map((link) => (
            <a
              key={link.label}
              href={link.href}
              className="text-sm text-muted transition-colors hover:text-fg"
            >
              {link.label}
            </a>
          ))}
          <a
            href={GITHUB_REPO_URL}
            className="flex items-center gap-1.5 text-sm text-muted transition-colors hover:text-fg"
          >
            GitHub
            {stars !== null && (
              <span className="flex items-center gap-1 rounded-full border border-line px-2 py-0.5 font-mono text-[11px] text-dim">
                <span className="text-clay">
                  <StarIcon />
                </span>
                {formatStars(stars)}
              </span>
            )}
          </a>
        </nav>
        <div className="flex items-center gap-2.5">
          <ThemeToggle />
          <CTAButton href="#install" size="sm">
            Install
          </CTAButton>
        </div>
      </div>
    </header>
  );
}

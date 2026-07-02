import Link from 'next/link';
import { Logo } from '@/components/logo';
import { NewsletterForm } from '@/components/newsletter-form';
import { GITHUB_REPO_URL, formatStars } from '@/lib/github';

const COLUMNS = [
  {
    title: 'Product',
    links: [
      { label: 'Documentation', href: 'https://docs.browserlane.com' },
      {
        label: 'CLI reference',
        href: 'https://docs.browserlane.com/cli-reference',
      },
      {
        label: 'MCP reference',
        href: 'https://docs.browserlane.com/mcp-reference',
      },
      {
        label: 'Claude Code skill',
        href: 'https://docs.browserlane.com/skill-reference',
      },
    ],
  },
  {
    title: 'Project',
    links: [
      {
        label: 'Releases',
        href: 'https://github.com/browserlane/browserlane/releases/latest',
      },
      { label: 'Changelog', href: 'https://docs.browserlane.com/changelog' },
      {
        label: 'License (Apache-2.0)',
        href: 'https://github.com/browserlane/browserlane/blob/main/LICENSE',
      },
    ],
  },
];

export function SiteFooter({ stars }: { stars: number | null }) {
  return (
    <footer className="mt-24 border-t border-line md:mt-32">
      {/* newsletter strip */}
      <div className="border-b border-line">
        <div className="mx-auto flex max-w-6xl flex-col justify-between gap-6 px-6 py-12 md:flex-row md:items-center">
          <div className="max-w-md">
            <h2 className="text-xl font-semibold tracking-tight text-fg">
              Follow the releases.
            </h2>
            <p className="mt-2 text-sm leading-relaxed text-dim">
              New commands, MCP tools, and release notes — straight from the
              changelog, no noise. Unsubscribe anytime.
            </p>
          </div>
          <NewsletterForm />
        </div>
      </div>

      <div className="mx-auto max-w-6xl px-6 py-14">
        <div className="flex flex-col justify-between gap-10 md:flex-row">
          <div className="max-w-xs">
            <Link href="/" className="flex items-center gap-2.5">
              <Logo size={24} />
              <span className="text-[15px] font-medium tracking-tight text-fg">
                browserlane
              </span>
            </Link>
            <p className="mt-4 text-sm leading-relaxed text-dim">
              Agentic browser testing and debugging for real web apps. One
              Rust binary — a CLI for humans, an MCP server for agents.
            </p>
          </div>
          <div className="grid grid-cols-2 gap-10 sm:grid-cols-3 sm:gap-16">
            {COLUMNS.map((col) => (
              <nav key={col.title} aria-label={col.title}>
                <p className="font-mono text-xs uppercase tracking-[0.18em] text-dim">
                  {col.title}
                </p>
                <ul className="mt-4 space-y-2.5">
                  {col.links.map((link) => (
                    <li key={link.label}>
                      <a
                        href={link.href}
                        className="text-sm text-muted transition-colors hover:text-fg"
                      >
                        {link.label}
                      </a>
                    </li>
                  ))}
                </ul>
              </nav>
            ))}
            <nav aria-label="Connect">
              <p className="font-mono text-xs uppercase tracking-[0.18em] text-dim">
                Connect
              </p>
              <ul className="mt-4 space-y-2.5">
                <li>
                  <a
                    href={GITHUB_REPO_URL}
                    className="text-sm text-muted transition-colors hover:text-fg"
                  >
                    GitHub{stars !== null ? ` · ★ ${formatStars(stars)}` : ''}
                  </a>
                </li>
                <li>
                  <a
                    href="https://www.linkedin.com/company/browserlane/"
                    className="text-sm text-muted transition-colors hover:text-fg"
                  >
                    LinkedIn
                  </a>
                </li>
                <li>
                  <a
                    href="mailto:bl@browserlane.com"
                    className="text-sm text-muted transition-colors hover:text-fg"
                  >
                    bl@browserlane.com
                  </a>
                </li>
              </ul>
            </nav>
          </div>
        </div>
        <div className="mt-12 flex flex-col justify-between gap-3 border-t border-line pt-6 font-mono text-xs text-faint sm:flex-row">
          <span>© 2026 browserlane · Apache-2.0</span>
          <span>built in Rust · Chrome over WebDriver BiDi</span>
        </div>
      </div>
    </footer>
  );
}

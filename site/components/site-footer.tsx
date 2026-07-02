import Link from 'next/link';
import { Logo } from '@/components/logo';

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
      { label: 'GitHub', href: 'https://github.com/browserlane/browserlane' },
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

export function SiteFooter() {
  return (
    <footer className="mt-24 border-t border-edge/60 md:mt-32">
      <div className="mx-auto max-w-6xl px-6 py-14">
        <div className="flex flex-col justify-between gap-10 md:flex-row">
          <div className="max-w-xs">
            <Link href="/" className="flex items-center gap-2.5">
              <Logo size={24} />
              <span className="text-[15px] font-medium tracking-tight text-ivory-light">
                browserlane
              </span>
            </Link>
            <p className="mt-4 text-sm leading-relaxed text-cloud">
              Agentic browser testing and debugging for real web apps. One Rust
              binary — a CLI for humans, an MCP server for agents.
            </p>
          </div>
          <div className="grid grid-cols-2 gap-10 sm:gap-20">
            {COLUMNS.map((col) => (
              <nav key={col.title} aria-label={col.title}>
                <p className="font-mono text-xs uppercase tracking-[0.18em] text-cloud">
                  {col.title}
                </p>
                <ul className="mt-4 space-y-2.5">
                  {col.links.map((link) => (
                    <li key={link.label}>
                      <a
                        href={link.href}
                        className="text-sm text-cloud-light transition-colors hover:text-ivory-light"
                      >
                        {link.label}
                      </a>
                    </li>
                  ))}
                </ul>
              </nav>
            ))}
          </div>
        </div>
        <div className="mt-12 flex flex-col justify-between gap-3 border-t border-edge/60 pt-6 font-mono text-xs text-cloud-dark sm:flex-row">
          <span>© 2026 browserlane · Apache-2.0</span>
          <span>built in Rust · Chrome over WebDriver BiDi</span>
        </div>
      </div>
    </footer>
  );
}

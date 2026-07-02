import Link from 'next/link';
import { Logo } from '@/components/logo';
import { CTAButton } from '@/components/ui/cta-button';

const LINKS = [
  { label: 'Docs', href: 'https://docs.browserlane.com' },
  { label: 'CLI', href: 'https://docs.browserlane.com/cli-reference' },
  { label: 'MCP', href: 'https://docs.browserlane.com/mcp-reference' },
  { label: 'GitHub', href: 'https://github.com/browserlane/browserlane' },
];

export function SiteNav() {
  return (
    <header className="fixed inset-x-0 top-0 z-50 border-b border-edge/60 bg-ink/85 backdrop-blur-md">
      <div className="mx-auto flex h-16 max-w-6xl items-center justify-between px-6">
        <Link href="/" className="flex items-center gap-2.5">
          <Logo size={26} />
          <span className="text-[15px] font-medium tracking-tight text-ivory-light">
            browserlane
          </span>
        </Link>
        <nav aria-label="Main" className="hidden items-center gap-7 md:flex">
          {LINKS.map((link) => (
            <a
              key={link.label}
              href={link.href}
              className="text-sm text-cloud-light transition-colors hover:text-ivory-light"
            >
              {link.label}
            </a>
          ))}
        </nav>
        <CTAButton href="#install" size="sm">
          Install
        </CTAButton>
      </div>
    </header>
  );
}

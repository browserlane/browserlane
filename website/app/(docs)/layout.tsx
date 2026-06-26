import { source } from '@/lib/source';
import { DocsLayout } from 'fumadocs-ui/layouts/notebook';
import { SidebarCollapseTrigger } from 'fumadocs-ui/layouts/notebook/slots/sidebar';
import type { ReactNode } from 'react';
import { baseOptions } from '@/lib/layout.shared';

/**
 * The sidebar collapse/expand toggle, relocated to sit beside the logo.
 *
 * In `nav.mode: 'top'` Fumadocs renders its built-in collapse trigger over on
 * the right, next to the theme switch — disconnected from the sidebar it
 * controls. We render our own here (fed in via `nav.children`, so it lands just
 * after the logo in the header) and hide the built-in one in global.css. A
 * custom `aria-label` keeps the two distinguishable for that CSS rule, and
 * `props` spread after the component's default label so ours wins.
 */
function SidebarToggle() {
  return (
    <SidebarCollapseTrigger
      aria-label="Toggle navigation sidebar"
      className="ms-1.5 inline-flex size-7 items-center justify-center rounded-md text-fd-muted-foreground transition-colors hover:bg-fd-accent hover:text-fd-accent-foreground max-md:hidden"
    >
      <svg
        xmlns="http://www.w3.org/2000/svg"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth={2}
        strokeLinecap="round"
        strokeLinejoin="round"
        className="size-4.5"
        aria-hidden="true"
      >
        <rect width="18" height="18" x="3" y="3" rx="2" />
        <path d="M9 3v18" />
      </svg>
    </SidebarCollapseTrigger>
  );
}

const isCli = (u: string) => u === '/cli-reference' || u.startsWith('/cli-reference/');
const isMcp = (u: string) => u === '/mcp-reference' || u.startsWith('/mcp-reference/');
const isSkill = (u: string) =>
  u === '/skill-reference' || u.startsWith('/skill-reference/');

export default function Layout({ children }: { children: ReactNode }) {
  const tree = source.getPageTree();
  const base = baseOptions();

  // The cli-reference / mcp-reference / skill-reference folders are their own
  // roots (own sidebar) and are kept out of the Docs sidebar via
  // content/docs/meta.json. The top-level docs root isn't auto-emitted as a
  // tab, so build all four tabs explicitly and bind each to the set of page
  // URLs it owns — derived from the full page list so it's independent of the
  // sidebar tree.
  const cliUrls = new Set<string>();
  const mcpUrls = new Set<string>();
  const skillUrls = new Set<string>();
  const docsUrls = new Set<string>();
  for (const page of source.getPages()) {
    const url = page.url;
    const bucket = isCli(url)
      ? cliUrls
      : isMcp(url)
        ? mcpUrls
        : isSkill(url)
          ? skillUrls
          : docsUrls;
    bucket.add(url);
  }

  return (
    <DocsLayout
      tree={tree}
      // Notebook layout = a real full-width top navbar that hosts the logo +
      // the Docs / CLI / MCP / Skill tabs (Playwright / Claude Code style).
      // CLI is for humans; MCP and Skill are the two agent-facing surfaces.
      tabMode="navbar"
      tabs={[
        { title: 'Docs', url: '/', urls: docsUrls },
        { title: 'CLI', url: '/cli-reference', urls: cliUrls },
        { title: 'MCP', url: '/mcp-reference', urls: mcpUrls },
        { title: 'Skill', url: '/skill-reference', urls: skillUrls },
      ]}
      {...base}
      nav={{ ...base.nav, mode: 'top', children: <SidebarToggle /> }}
    >
      {children}
    </DocsLayout>
  );
}

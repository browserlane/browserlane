import { source } from '@/lib/source';
import { DocsLayout } from 'fumadocs-ui/layouts/notebook';
import type { ReactNode } from 'react';
import { baseOptions } from '@/lib/layout.shared';

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
      nav={{ ...base.nav, mode: 'top' }}
    >
      {children}
    </DocsLayout>
  );
}

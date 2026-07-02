import { SectionHeading } from '@/components/ui/section-heading';
import { TerminalPanel } from '@/components/ui/terminal-panel';

export function TwoSurfaces() {
  return (
    <section
      id="surfaces"
      aria-label="Two surfaces"
      className="mx-auto max-w-6xl px-6 pt-24 md:pt-32"
    >
      <SectionHeading
        eyebrow="Two surfaces"
        title="One binary. Two ways in."
        sub="The same engine answers to a human at a shell and to an agent over MCP. Whatever your agent learns to do, you can reproduce by hand — and vice versa."
      />

      <div className="mt-12 grid grid-cols-1 gap-5 lg:grid-cols-2">
        {/* CLI */}
        <div className="flex flex-col rounded-2xl border border-line bg-card/60 p-6 md:p-7">
          <p className="font-mono text-xs uppercase tracking-[0.2em] text-accent">
            CLI · for humans &amp; scripts
          </p>
          <h3 className="mt-3 text-xl font-semibold tracking-tight text-fg">
            Drive the browser from your shell.
          </h3>
          <p className="mt-2 text-sm leading-relaxed text-muted">
            66 commands with composable output and real exit codes — pipe them,
            script them, put them in CI.
          </p>
          <TerminalPanel
            title="zsh — bl"
            className="mt-5"
            lines={[
              { type: 'cmd', text: 'bl go https://app.example.com' },
              { type: 'ok', text: '200 · Acme — Dashboard' },
              { type: 'cmd', text: 'bl map' },
              { type: 'out', text: '@e1 link "Projects"   @e2 button "New project"' },
              { type: 'cmd', text: 'bl click @e2' },
              { type: 'ok', text: 'clicked' },
              { type: 'cmd', text: 'bl expect url contains "/projects/new"' },
              { type: 'ok', text: 'pass · exit 0' },
            ]}
          />
          <a
            href="https://docs.browserlane.com/cli-reference"
            className="mt-5 inline-block font-mono text-[13px] text-accent transition-colors hover:text-accent-hover"
          >
            CLI reference →
          </a>
        </div>

        {/* MCP */}
        <div className="flex flex-col rounded-2xl border border-line bg-card/60 p-6 md:p-7">
          <p className="font-mono text-xs uppercase tracking-[0.2em] text-accent">
            MCP · for AI agents
          </p>
          <h3 className="mt-3 text-xl font-semibold tracking-tight text-fg">
            Give your agent structured hands.
          </h3>
          <p className="mt-2 text-sm leading-relaxed text-muted">
            86 tools over stdio JSON-RPC. One command registers the server with
            Claude Code, Claude Desktop, Cursor, VS Code, or Codex.
          </p>
          <TerminalPanel
            title="mcp · stdio json-rpc"
            className="mt-5"
            lines={[
              { type: 'out', text: '→ browser_navigate {"url":"https://app.example.com"}' },
              { type: 'out', text: '→ browser_map {}' },
              { type: 'out', text: '←  @e2 button "New project"' },
              { type: 'out', text: '→ browser_click {"selector":"@e2"}' },
              { type: 'ok', text: '←  clicked · 96 ms' },
              { type: 'out', text: '→ browser_expect {"kind":"url","contains":"/projects/new"}' },
              { type: 'ok', text: '←  pass' },
            ]}
          />
          <div className="mt-5 flex flex-wrap items-center justify-between gap-3">
            <code className="rounded-md border border-edge bg-ink px-2.5 py-1.5 font-mono text-xs text-ivory">
              <span className="text-clay">$</span> bl add-mcp claude
            </code>
            <a
              href="https://docs.browserlane.com/mcp-reference"
              className="font-mono text-[13px] text-accent transition-colors hover:text-accent-hover"
            >
              MCP reference →
            </a>
          </div>
        </div>
      </div>

      <p className="mt-6 text-sm text-dim">
        There’s a third, token-light surface too: a Claude Code skill that
        teaches agents the CLI directly —{' '}
        <code className="rounded bg-slate px-1.5 py-0.5 font-mono text-[13px] text-cloud-light">
          bl add-skill
        </code>
        .{' '}
        <a
          href="https://docs.browserlane.com/skill-reference"
          className="text-accent hover:text-accent-hover"
        >
          Skill reference →
        </a>
      </p>
    </section>
  );
}

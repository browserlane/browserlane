import { SectionHeading } from '@/components/ui/section-heading';
import { CopyButton } from '@/components/ui/copy-button';
import { CTAButton } from '@/components/ui/cta-button';

function CommandRow({ command, note }: { command: string; note?: string }) {
  return (
    <div className="flex items-center justify-between gap-3 rounded-lg border border-edge bg-ink px-4 py-3">
      <div className="min-w-0 overflow-x-auto whitespace-nowrap font-mono text-xs text-ivory [scrollbar-width:none] md:text-[13px]">
        <span className="select-none text-clay">$ </span>
        <span className="whitespace-pre">{command}</span>
        {note ? (
          <span className="ml-3 hidden text-cloud-dark sm:inline"># {note}</span>
        ) : null}
      </div>
      <CopyButton text={command} />
    </div>
  );
}

export function Quickstart() {
  return (
    <section
      id="install"
      aria-label="Install browserlane"
      className="mx-auto max-w-6xl scroll-mt-24 px-6 pt-24 md:pt-32"
    >
      <div className="rounded-2xl border border-edge bg-slate/40 p-6 md:p-12">
        <SectionHeading
          eyebrow="Quickstart"
          title="Running in under a minute."
          sub="One line to install, one command to fetch Chrome for Testing, and you’re driving a real browser — from your shell or your agent."
        />

        <div className="mt-10 grid grid-cols-1 gap-8 lg:grid-cols-2">
          <div className="space-y-6">
            <div>
              <p className="mb-2.5 font-mono text-xs uppercase tracking-[0.18em] text-cloud">
                1 · Install — macOS / Linux
              </p>
              <CommandRow command="curl -fsSL https://browserlane.com/install.sh | sh" />
            </div>
            <div>
              <p className="mb-2.5 font-mono text-xs uppercase tracking-[0.18em] text-cloud">
                Windows (PowerShell)
              </p>
              <CommandRow command="irm https://browserlane.com/install.ps1 | iex" />
            </div>
            <div>
              <p className="mb-2.5 font-mono text-xs uppercase tracking-[0.18em] text-cloud">
                2 · Fetch Chrome for Testing
              </p>
              <CommandRow command="bl install" />
            </div>
          </div>

          <div className="space-y-6">
            <div>
              <p className="mb-2.5 font-mono text-xs uppercase tracking-[0.18em] text-cloud">
                3 · First commands
              </p>
              <div className="space-y-2.5">
                <CommandRow command="bl go https://example.com" />
                <CommandRow command="bl screenshot -o page.png" />
              </div>
            </div>
            <div>
              <p className="mb-2.5 font-mono text-xs uppercase tracking-[0.18em] text-cloud">
                For agents · register the MCP server
              </p>
              <CommandRow
                command="bl add-mcp claude"
                note="or cursor · vscode · codex"
              />
            </div>
          </div>
        </div>

        <div className="mt-10 flex flex-wrap items-center gap-3">
          <CTAButton href="https://docs.browserlane.com">
            Read the docs
          </CTAButton>
          <CTAButton
            href="https://github.com/browserlane/browserlane"
            variant="ghost"
          >
            Star on GitHub
          </CTAButton>
          <p className="font-mono text-xs text-cloud">
            checksummed installer · signed &amp; notarized binaries
          </p>
        </div>
      </div>
    </section>
  );
}

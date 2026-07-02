import { BrowserStackVisual } from '@/components/layer-story/browser-stack-visual';
import { CTAButton } from '@/components/ui/cta-button';
import { TerminalPanel } from '@/components/ui/terminal-panel';

export function Hero() {
  return (
    <section className="relative overflow-hidden pt-32 md:pt-40">
      {/* faint dot grid, brand-toned — fades out toward the fold */}
      <div
        aria-hidden
        className="pointer-events-none absolute inset-0"
        style={{
          backgroundImage:
            'radial-gradient(circle, var(--bl-dot) 1px, transparent 1px)',
          backgroundSize: '28px 28px',
          maskImage:
            'linear-gradient(to bottom, black 0%, black 40%, transparent 75%)',
          WebkitMaskImage:
            'linear-gradient(to bottom, black 0%, black 40%, transparent 75%)',
        }}
      />

      <div className="relative mx-auto max-w-6xl px-6 text-center">
        {/* the slogan */}
        <p className="mx-auto max-w-3xl text-balance font-mono text-xs uppercase tracking-[0.22em] text-accent">
          Agentic browser testing and debugging for real web apps
        </p>

        <h1 className="mx-auto mt-6 max-w-4xl text-balance text-4xl font-semibold leading-[1.05] tracking-tight text-fg sm:text-5xl md:text-[4.25rem]">
          A real browser, made legible for agents.
        </h1>

        <p className="mx-auto mt-6 max-w-2xl text-balance text-base leading-relaxed text-muted md:text-lg">
          browserlane gives AI agents and developers a single binary for
          driving Chrome, inspecting web apps, debugging failures, and
          capturing evidence across every run.
        </p>

        <div className="mt-9 flex flex-wrap items-center justify-center gap-3">
          <CTAButton href="#install">Install browserlane</CTAButton>
          <CTAButton href="https://docs.browserlane.com" variant="ghost">
            View docs
          </CTAButton>
        </div>
        <p className="mt-5 inline-flex items-center gap-2 font-mono text-xs text-dim">
          <span className="size-1.5 rounded-full bg-clay" />
          v0.1.3 · one static binary · macOS / Linux / Windows · Apache-2.0
        </p>
      </div>

      {/* product visual: browser + overlapping CLI and MCP panels */}
      <div className="relative mx-auto mt-16 max-w-6xl px-6 md:mt-20">
        <div
          aria-hidden
          className="pointer-events-none absolute inset-x-12 top-8 bottom-0 rounded-[40px] bg-clay/[0.07] blur-3xl"
        />
        <div className="relative mx-auto max-w-3xl md:pb-32 lg:max-w-4xl">
          <BrowserStackVisual layer="hero" className="md:px-16" />

          <TerminalPanel
            title="zsh — bl"
            className="mt-5 shadow-2xl md:absolute md:bottom-10 md:left-0 md:mt-0 md:w-[380px] lg:-left-6"
            lines={[
              { type: 'cmd', text: 'bl go https://app.example.com' },
              { type: 'ok', text: '200 · Acme — Dashboard' },
              { type: 'cmd', text: 'bl map' },
              { type: 'out', text: '@e1 link "Projects"  @e2 button "New project"' },
              { type: 'cmd', text: 'bl click @e2' },
              { type: 'ok', text: 'clicked · url /projects/new' },
            ]}
          />

          <div className="mt-4 overflow-hidden rounded-xl border border-edge bg-slate shadow-2xl md:absolute md:-right-2 md:top-8 md:mt-0 md:w-[290px] lg:-right-8">
            <div className="flex items-center justify-between border-b border-edge/70 px-3.5 py-2.5 font-mono text-[11px]">
              <span className="text-cloud">mcp · tools/call</span>
              <span className="rounded-sm bg-clay/15 px-1.5 py-0.5 text-[10px] text-kraft">
                agent
              </span>
            </div>
            <pre className="overflow-x-auto p-4 font-mono text-xs leading-6 text-cloud-light">
              <span className="text-cloud">{'{'}</span>
              {'\n  '}
              <span className="text-focus">&quot;name&quot;</span>
              <span className="text-cloud">: </span>
              <span className="text-manilla">&quot;browser_click&quot;</span>
              <span className="text-cloud">,</span>
              {'\n  '}
              <span className="text-focus">&quot;arguments&quot;</span>
              <span className="text-cloud">: {'{ '}</span>
              <span className="text-focus">&quot;selector&quot;</span>
              <span className="text-cloud">: </span>
              <span className="text-manilla">&quot;@e2&quot;</span>
              <span className="text-cloud">{' }'}</span>
              {'\n'}
              <span className="text-cloud">{'}'}</span>
              {'\n'}
              <span className="text-kraft">→ clicked · 96 ms</span>
            </pre>
          </div>
        </div>
      </div>
    </section>
  );
}

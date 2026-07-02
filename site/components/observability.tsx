import { SectionHeading } from '@/components/ui/section-heading';

function ArtifactCard({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="rounded-xl border border-edge bg-slate p-5">
      <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-cloud">
        {label}
      </p>
      <div className="mt-3 overflow-x-auto font-mono text-xs leading-6 md:text-[13px]">
        {children}
      </div>
    </div>
  );
}

export function Observability() {
  return (
    <section
      aria-label="Debugging and observability"
      className="mx-auto max-w-6xl px-6 pt-24 md:pt-32"
    >
      <SectionHeading
        eyebrow="Debugging"
        title="Agents shouldn’t click blind."
        sub="Most browser tools tell an agent what the page looks like. browserlane also tells it what happened — assertions with exit codes, structured diffs, and recordings that carry console and network context. Failures become evidence to read, not behavior to guess at."
      />

      <div className="mt-12 grid grid-cols-1 gap-4 md:grid-cols-2">
        <ArtifactCard label="Assertions · real exit codes">
          <div className="whitespace-pre text-ivory">
            <span className="text-clay">$ </span>bl expect text &quot;#total&quot; &quot;$42.00&quot;
          </div>
          <div className="whitespace-pre text-danger">
            ✗ fail · expected &quot;$42.00&quot;, got &quot;NaN&quot;
          </div>
          <div className="whitespace-pre text-cloud">exit 1 — CI stops here, with a reason</div>
        </ArtifactCard>

        <ArtifactCard label="State diffs · between steps">
          <div className="whitespace-pre text-ivory">
            <span className="text-clay">$ </span>bl diff map
          </div>
          <div className="whitespace-pre text-kraft">+ @e9 button &quot;Retry payment&quot;</div>
          <div className="whitespace-pre text-danger">− @e5 spinner &quot;Processing…&quot;</div>
          <div className="whitespace-pre text-cloud">2 changes since last map</div>
        </ArtifactCard>

        <ArtifactCard label="Recordings · replayable runs">
          <div className="whitespace-pre text-ivory">
            <span className="text-clay">$ </span>bl record stop -o run.zip
          </div>
          <div className="whitespace-pre text-cloud-light">
            ✓ saved · 9 steps · screenshots + snapshots
          </div>
          <div className="whitespace-pre text-cloud">
            with console output &amp; network activity
          </div>
        </ArtifactCard>

        <ArtifactCard label="Screenshots · annotated evidence">
          <div className="whitespace-pre text-ivory">
            <span className="text-clay">$ </span>bl screenshot --full-page --annotate
          </div>
          <div className="whitespace-pre text-cloud-light">
            ✓ screenshot.png · interactive elements numbered
          </div>
          <div className="whitespace-pre text-cloud">
            what the agent saw, exactly when it saw it
          </div>
        </ArtifactCard>
      </div>
    </section>
  );
}

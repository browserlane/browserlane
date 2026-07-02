const ITEMS = [
  {
    title: 'One static binary',
    body: 'A single Rust binary for macOS, Linux, and Windows. No Node, no driver daemon, no runtime.',
  },
  {
    title: 'CLI + MCP',
    body: '66 CLI commands for humans and scripts. 86 MCP tools for AI agents. Same engine underneath.',
  },
  {
    title: 'Chrome over WebDriver BiDi',
    body: 'The W3C-standard bidirectional protocol — not a vendor side-channel.',
  },
  {
    title: 'Built for real web apps',
    body: 'Real renders, real input events, real state — and evidence captured on every run.',
  },
];

export function TrustStrip() {
  return (
    <section
      aria-label="Why browserlane"
      className="mt-24 border-y border-edge/60 md:mt-32"
    >
      <div className="mx-auto grid max-w-6xl grid-cols-1 divide-y divide-edge/60 sm:grid-cols-2 sm:divide-x lg:grid-cols-4 lg:divide-y-0">
        {ITEMS.map((item) => (
          <div key={item.title} className="px-6 py-7">
            <h2 className="font-mono text-[13px] font-medium text-ivory-light">
              {item.title}
            </h2>
            <p className="mt-2 text-sm leading-relaxed text-cloud">
              {item.body}
            </p>
          </div>
        ))}
      </div>
    </section>
  );
}

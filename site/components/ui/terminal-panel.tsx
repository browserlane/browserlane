import type { ReactNode } from 'react';

export type TermLine =
  | { type: 'cmd'; text: string }
  | { type: 'out'; text: string }
  | { type: 'ok'; text: string }
  | { type: 'err'; text: string }
  | { type: 'comment'; text: string };

function LineView({ line }: { line: TermLine }) {
  switch (line.type) {
    case 'cmd':
      return (
        <div className="whitespace-pre text-ivory">
          <span className="select-none text-clay">$ </span>
          {line.text}
        </div>
      );
    case 'ok':
      return (
        <div className="whitespace-pre text-cloud-light">
          <span className="text-kraft">✓ </span>
          {line.text}
        </div>
      );
    case 'err':
      return (
        <div className="whitespace-pre text-danger">
          <span>✗ </span>
          {line.text}
        </div>
      );
    case 'comment':
      return <div className="whitespace-pre text-cloud-dark"># {line.text}</div>;
    default:
      return <div className="whitespace-pre text-cloud">{line.text}</div>;
  }
}

export function PanelChrome({
  title,
  children,
  className = '',
}: {
  title: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={`overflow-hidden rounded-xl border border-edge bg-slate ${className}`}
    >
      <div className="flex items-center gap-2 border-b border-edge/70 px-3.5 py-2.5">
        <span aria-hidden className="flex gap-1.5">
          <span className="size-2.5 rounded-full bg-edge" />
          <span className="size-2.5 rounded-full bg-edge" />
          <span className="size-2.5 rounded-full bg-edge" />
        </span>
        <span className="ml-1 font-mono text-[11px] text-cloud">{title}</span>
      </div>
      {children}
    </div>
  );
}

export function TerminalPanel({
  title = 'zsh',
  lines,
  className = '',
}: {
  title?: string;
  lines: TermLine[];
  className?: string;
}) {
  return (
    <PanelChrome title={title} className={className}>
      <div className="overflow-x-auto p-4 font-mono text-xs leading-6 md:text-[13px]">
        {lines.map((line, i) => (
          <LineView key={i} line={line} />
        ))}
      </div>
    </PanelChrome>
  );
}

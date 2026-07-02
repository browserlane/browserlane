'use client';

import { useState } from 'react';

export function CopyButton({ text }: { text: string }) {
  const [copied, setCopied] = useState(false);

  return (
    <button
      type="button"
      aria-label={copied ? 'Copied' : 'Copy command'}
      onClick={() => {
        navigator.clipboard.writeText(text).then(() => {
          setCopied(true);
          setTimeout(() => setCopied(false), 1800);
        });
      }}
      className="shrink-0 rounded-md border border-edge px-2.5 py-1 font-mono text-[11px] text-cloud transition-colors hover:border-cloud hover:text-ivory"
    >
      {copied ? 'copied' : 'copy'}
    </button>
  );
}

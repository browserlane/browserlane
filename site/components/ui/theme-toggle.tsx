'use client';

import { useEffect, useSyncExternalStore } from 'react';

type Mode = 'system' | 'light' | 'dark';

const KEY = 'bl-theme';

/* The stored preference as a tiny external store: localStorage is the
   source of truth, `emit` notifies subscribers after a same-tab write.
   All storage access is guarded — merely touching window.localStorage
   throws in storage-blocked contexts (e.g. Chrome "Block all cookies"),
   and an unguarded throw here would unmount the whole page. */
let listeners: Array<() => void> = [];

function subscribe(listener: () => void) {
  listeners.push(listener);
  return () => {
    listeners = listeners.filter((l) => l !== listener);
  };
}

function getSnapshot(): Mode {
  try {
    const stored = localStorage.getItem(KEY);
    return stored === 'light' || stored === 'dark' ? stored : 'system';
  } catch {
    return 'system';
  }
}

// Server renders no selection; the client snapshot takes over on hydration.
function getServerSnapshot(): Mode | null {
  return null;
}

function apply(mode: Mode) {
  const dark =
    mode === 'dark' ||
    (mode === 'system' &&
      window.matchMedia('(prefers-color-scheme: dark)').matches);
  document.documentElement.classList.toggle('dark', dark);
}

function SystemIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
      <rect x="2" y="4" width="20" height="13" rx="2" />
      <path d="M8 21h8M12 17v4" />
    </svg>
  );
}

function SunIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41" />
    </svg>
  );
}

function MoonIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
      <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
    </svg>
  );
}

const MODES: { mode: Mode; label: string; icon: () => React.ReactNode }[] = [
  { mode: 'system', label: 'System theme', icon: SystemIcon },
  { mode: 'light', label: 'Light theme', icon: SunIcon },
  { mode: 'dark', label: 'Dark theme', icon: MoonIcon },
];

export function ThemeToggle() {
  const mode = useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);

  // Follow OS theme changes live while in system mode.
  useEffect(() => {
    const mq = window.matchMedia('(prefers-color-scheme: dark)');
    const onChange = () => {
      if (getSnapshot() === 'system') apply('system');
    };
    mq.addEventListener('change', onChange);
    return () => mq.removeEventListener('change', onChange);
  }, []);

  function select(next: Mode) {
    try {
      localStorage.setItem(KEY, next);
    } catch {
      // Storage blocked — still apply for this page view.
    }
    apply(next);
    listeners.forEach((l) => l());
  }

  /* Plain toggle buttons with aria-pressed (not role=radio): each button is
     its own tab stop, which matches actual keyboard behavior — radio
     semantics would promise arrow-key navigation we don't implement. */
  return (
    <div
      role="group"
      aria-label="Color theme"
      className="flex items-center rounded-lg border border-line p-0.5"
    >
      {MODES.map(({ mode: m, label, icon: Icon }) => (
        <button
          key={m}
          type="button"
          aria-pressed={mode === m}
          aria-label={label}
          title={label}
          onClick={() => select(m)}
          className={`flex size-7 items-center justify-center rounded-md transition-colors ${
            mode === m ? 'bg-card text-fg' : 'text-faint hover:text-fg'
          }`}
        >
          <Icon />
        </button>
      ))}
    </div>
  );
}

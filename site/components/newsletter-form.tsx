'use client';

import { useState } from 'react';

type Status = 'idle' | 'loading' | 'success' | 'error';

export function NewsletterForm() {
  const [status, setStatus] = useState<Status>('idle');
  const [message, setMessage] = useState('');

  async function onSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (status === 'loading') return; // re-entrancy guard, see button note
    const form = event.currentTarget;
    const data = new FormData(form);
    setStatus('loading');
    setMessage('');

    try {
      const res = await fetch('/api/subscribe', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          email: data.get('email'),
          company: data.get('company'),
        }),
      });
      const json = (await res.json()) as { ok?: boolean; error?: string };
      if (res.ok && json.ok) {
        setStatus('success');
        setMessage('Subscribed — see you in the next update.');
        form.reset();
      } else {
        setStatus('error');
        setMessage(json.error ?? 'Something went wrong — try again.');
      }
    } catch {
      setStatus('error');
      setMessage('Something went wrong — try again.');
    }
  }

  return (
    <form onSubmit={onSubmit} className="w-full max-w-md">
      <div className="flex gap-2">
        <label htmlFor="newsletter-email" className="sr-only">
          Email address
        </label>
        <input
          id="newsletter-email"
          name="email"
          type="email"
          required
          autoComplete="email"
          placeholder="you@example.com"
          className="h-11 min-w-0 flex-1 rounded-lg border border-line bg-canvas px-3.5 font-mono text-[13px] text-fg placeholder:text-faint focus:border-ring focus:outline-none"
        />
        {/* Honeypot — hidden from real users, catches naive bots. */}
        <input
          type="text"
          name="company"
          tabIndex={-1}
          autoComplete="off"
          aria-hidden="true"
          className="hidden"
        />
        {/* aria-disabled (not disabled): disabling the focused button mid-
            submit would drop keyboard focus to <body>; the onSubmit guard
            makes duplicate activations no-ops instead. */}
        <button
          type="submit"
          aria-disabled={status === 'loading'}
          className={`h-11 shrink-0 rounded-lg border border-clay bg-clay px-4 text-[15px] font-medium tracking-tight text-ink transition-colors hover:border-kraft hover:bg-kraft ${
            status === 'loading' ? 'opacity-60' : ''
          }`}
        >
          {status === 'loading' ? 'Subscribing…' : 'Subscribe'}
        </button>
      </div>
      <p
        aria-live="polite"
        className={`mt-2 min-h-5 text-sm ${
          status === 'error' ? 'text-danger' : 'text-dim'
        }`}
      >
        {message}
      </p>
    </form>
  );
}

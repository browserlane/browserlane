import Link from 'next/link';
import type { Metadata } from 'next';

export const metadata: Metadata = {
  title: 'Page not found',
};

export default function NotFound() {
  return (
    <main className="flex flex-1 flex-col items-center justify-center px-6 py-24 text-center">
      <p className="font-mono text-sm font-medium text-fd-primary">404</p>
      <h1 className="mt-4 text-3xl font-bold tracking-tight">
        Page not found
      </h1>
      <p className="mt-3 max-w-md text-fd-muted-foreground">
        We couldn&apos;t find that page. It may have moved, or the link might be
        out of date.
      </p>
      <Link
        href="/"
        className="mt-8 inline-flex items-center rounded-lg bg-fd-primary px-4 py-2 text-sm font-medium text-fd-primary-foreground transition-colors hover:bg-fd-primary/90"
      >
        Back to the docs
      </Link>
    </main>
  );
}

import './global.css';
import { RootProvider } from 'fumadocs-ui/provider/next';
import { GeistSans } from 'geist/font/sans';
import { GeistMono } from 'geist/font/mono';
import type { Metadata } from 'next';
import type { ReactNode } from 'react';

const SITE_URL = 'https://docs.browserlane.com';
const SITE_NAME = 'browserlane';
const tagline =
  'Browser automation for humans and AI agents, via CLI and MCP, built on WebDriver BiDi in Rust.';

export const metadata: Metadata = {
  metadataBase: new URL(SITE_URL),
  title: {
    default: 'browserlane — browser automation for humans and AI agents',
    template: '%s — browserlane',
  },
  description: tagline,
  applicationName: SITE_NAME,
  keywords: [
    'browserlane',
    'browser automation',
    'WebDriver BiDi',
    'MCP',
    'Model Context Protocol',
    'AI agents',
    'Chrome',
    'CLI',
    'Rust',
  ],
  openGraph: {
    type: 'website',
    siteName: SITE_NAME,
    url: SITE_URL,
    title: 'browserlane — browser automation for humans and AI agents',
    description: tagline,
  },
  twitter: {
    card: 'summary_large_image',
    title: 'browserlane — browser automation for humans and AI agents',
    description: tagline,
  },
};

export default function Layout({ children }: { children: ReactNode }) {
  return (
    <html
      lang="en"
      className={`${GeistSans.variable} ${GeistMono.variable}`}
      suppressHydrationWarning
    >
      <body className="flex flex-col min-h-screen">
        <RootProvider>{children}</RootProvider>
      </body>
    </html>
  );
}

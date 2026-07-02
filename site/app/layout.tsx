import type { Metadata } from 'next';
import { GeistSans } from 'geist/font/sans';
import { GeistMono } from 'geist/font/mono';
import './globals.css';

export const metadata: Metadata = {
  metadataBase: new URL('https://browserlane.com'),
  title: 'browserlane — agentic browser testing and debugging for real web apps',
  description:
    'browserlane gives AI agents and developers a single binary for driving Chrome, inspecting web apps, debugging failures, and capturing evidence across every run. A CLI for humans and an MCP server for agents, over WebDriver BiDi.',
  openGraph: {
    title: 'browserlane — agentic browser testing and debugging for real web apps',
    description:
      'One static binary: a CLI for humans and an MCP server for AI agents, driving Chrome over WebDriver BiDi.',
    url: 'https://browserlane.com',
    siteName: 'browserlane',
    type: 'website',
  },
  twitter: {
    card: 'summary',
    title: 'browserlane — agentic browser testing and debugging for real web apps',
    description:
      'One static binary: a CLI for humans and an MCP server for AI agents, driving Chrome over WebDriver BiDi.',
  },
};

/**
 * Applies the saved theme before first paint to avoid a flash.
 * 'system' (or nothing saved) follows prefers-color-scheme; the toggle in
 * the nav writes 'light' | 'dark' | 'system' to localStorage.
 *
 * Deliberately a plain parse-time <script> as the first child of <body> —
 * NOT next/script: in the App Router an inline `beforeInteractive` Script
 * is only queued into `self.__next_s` and executed just before hydration,
 * i.e. AFTER first paint, which reintroduces the light flash. (React logs
 * a dev-only warning about script tags on HMR re-renders; harmless.)
 */
const THEME_INIT = `(function(){try{var t=localStorage.getItem('bl-theme');var d=t==='dark'||((t===null||t==='system')&&window.matchMedia('(prefers-color-scheme: dark)').matches);document.documentElement.classList.toggle('dark',d);}catch(e){}})();`;

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html
      lang="en"
      className={`${GeistSans.variable} ${GeistMono.variable}`}
      suppressHydrationWarning
    >
      <body className="font-sans">
        <script dangerouslySetInnerHTML={{ __html: THEME_INIT }} />
        {children}
      </body>
    </html>
  );
}

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
    title: 'browserlane — a real browser, made legible for agents',
    description:
      'One static binary: a CLI for humans and an MCP server for AI agents, driving Chrome over WebDriver BiDi.',
    url: 'https://browserlane.com',
    siteName: 'browserlane',
    type: 'website',
  },
  twitter: {
    card: 'summary',
    title: 'browserlane — a real browser, made legible for agents',
    description:
      'One static binary: a CLI for humans and an MCP server for AI agents, driving Chrome over WebDriver BiDi.',
  },
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className={`${GeistSans.variable} ${GeistMono.variable}`}>
      <body className="font-sans">{children}</body>
    </html>
  );
}

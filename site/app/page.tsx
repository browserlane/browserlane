import { SiteNav } from '@/components/site-nav';
import { Hero } from '@/components/hero';
import { TrustStrip } from '@/components/trust-strip';
import { LayerScrollStory } from '@/components/layer-story/layer-scroll-story';
import { TwoSurfaces } from '@/components/two-surfaces';
import { Observability } from '@/components/observability';
import { Quickstart } from '@/components/quickstart';
import { SiteFooter } from '@/components/site-footer';

export default function Home() {
  return (
    <>
      <a
        href="#main"
        className="sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-4 focus:z-[60] focus:rounded-md focus:bg-clay focus:px-3 focus:py-2 focus:text-sm focus:text-ink"
      >
        Skip to content
      </a>
      <SiteNav />
      <main id="main">
        <Hero />
        <TrustStrip />
        <LayerScrollStory />
        <TwoSurfaces />
        <Observability />
        <Quickstart />
      </main>
      <SiteFooter />
    </>
  );
}

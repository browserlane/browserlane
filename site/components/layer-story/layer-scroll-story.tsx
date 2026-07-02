'use client';

import { useEffect, useRef, useState } from 'react';
import { SectionHeading } from '@/components/ui/section-heading';
import { BrowserStackVisual } from './browser-stack-visual';
import { LAYERS } from './layers';

/**
 * The "Layer Scroll Story".
 *
 * Desktop (lg+): copy steps scroll on the left while the BrowserStackVisual
 * stays pinned (position: sticky) on the right; the step nearest the viewport
 * center is active and drives the visual's layer.
 *
 * Mobile: no pinning — each step renders its own static visual above the
 * copy, so the story reads as a plain vertical sequence. The page therefore
 * works identically with animations disabled.
 */
export function LayerScrollStory() {
  const [active, setActive] = useState(0);
  const stepRefs = useRef<(HTMLElement | null)[]>([]);

  useEffect(() => {
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            setActive(Number((entry.target as HTMLElement).dataset.index));
          }
        }
      },
      // A thin horizontal band around the viewport center decides the active step.
      { rootMargin: '-45% 0px -45% 0px', threshold: 0 },
    );
    stepRefs.current.forEach((el) => el && observer.observe(el));
    return () => observer.disconnect();
  }, []);

  return (
    <section id="stack" aria-label="The browser stack" className="relative">
      <div className="mx-auto max-w-6xl px-6 pt-24 md:pt-32">
        <SectionHeading
          eyebrow="The browser stack"
          title="A browser isn’t one surface. It’s a stack."
          sub="Shell, tabs, DOM, input, state, signals, environment, evidence. browserlane gives agents and developers a handle at every layer — not just a screenshot of the top one."
        />
      </div>

      <div className="mx-auto max-w-6xl px-6">
        <div className="lg:grid lg:grid-cols-[minmax(0,5fr)_minmax(0,7fr)] lg:gap-16">
          {/* copy column */}
          <div>
            {LAYERS.map((layer, i) => (
              <article
                key={layer.id}
                data-index={i}
                ref={(el) => {
                  stepRefs.current[i] = el;
                }}
                className={`flex flex-col justify-center py-16 transition-opacity duration-300 lg:min-h-[72vh] lg:py-10 ${
                  active === i ? 'lg:opacity-100' : 'lg:opacity-35'
                }`}
              >
                {/* mobile: inline visual per step */}
                <div className="mb-8 lg:hidden">
                  <BrowserStackVisual layer={layer.id} />
                </div>

                <p className="font-mono text-xs uppercase tracking-[0.22em] text-dim">
                  <span className="text-accent">{layer.index}</span> · {layer.name}
                </p>
                <h3 className="mt-3 text-2xl font-semibold tracking-tight text-fg md:text-3xl">
                  {layer.title}
                </h3>
                <p className="mt-3 max-w-md text-[15px] leading-relaxed text-muted">
                  {layer.body}
                </p>
                <div className="mt-5 max-w-md overflow-x-auto rounded-lg border border-edge bg-ink p-4 font-mono text-xs leading-7 md:text-[13px]">
                  {layer.commands.map((cmd) => (
                    <div key={cmd} className="whitespace-pre text-ivory">
                      <span className="select-none text-clay">$ </span>
                      {cmd}
                    </div>
                  ))}
                </div>
              </article>
            ))}
          </div>

          {/* pinned visual column (desktop only) */}
          <div className="hidden lg:block">
            <div className="sticky top-0 flex h-screen flex-col justify-center py-10">
              <div className="mb-5 flex items-center justify-between font-mono text-xs text-dim">
                <span aria-live="polite">
                  <span className="text-accent">{LAYERS[active].index}</span>
                  <span className="text-faint"> / 08</span> —{' '}
                  {LAYERS[active].name.toLowerCase()}
                </span>
                <span className="flex gap-1.5">
                  {LAYERS.map((layer, i) => (
                    <button
                      key={layer.id}
                      type="button"
                      aria-label={`Jump to layer ${layer.index}: ${layer.name}`}
                      onClick={() =>
                        stepRefs.current[i]?.scrollIntoView({
                          block: 'center',
                        })
                      }
                      className={`h-1.5 rounded-full transition-all duration-300 ${
                        active === i
                          ? 'w-6 bg-clay'
                          : 'w-1.5 bg-line hover:bg-faint'
                      }`}
                    />
                  ))}
                </span>
              </div>
              <BrowserStackVisual layer={LAYERS[active].id} />
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

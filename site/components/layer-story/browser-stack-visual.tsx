'use client';

import { AnimatePresence, motion, useReducedMotion } from 'motion/react';
import { LAYERS, type LayerId } from './layers';

/**
 * The pinned "product object" of the scroll story: a stylized browser window
 * rendered entirely from divs/SVG (no images). The `layer` prop selects which
 * capability layer is active; overlays animate in/out per layer.
 *
 * Everything inside is decorative — the container carries a role="img" with a
 * per-layer description, and the internals are aria-hidden.
 */

type VisualLayer = LayerId | 'hero';

const EASE = [0.32, 0.72, 0, 1] as const;

const DESCRIPTIONS: Record<VisualLayer, string> = {
  hero: 'A browser window rendering a web app, driven by the bl command line.',
  shell: 'A browser window materializes and renders a web app after `bl go`.',
  tabs: 'The browser splits into stacked tab planes: dashboard, staging, and an authenticated admin context.',
  dom: 'The page shown as an x-ray: dashed element outlines with selector labels and an accessibility tree.',
  inputs:
    'A cursor path travels to the Submit button while typed text fills a focused email field.',
  state:
    'A state drawer under the page listing session cookie, localStorage, and sessionStorage entries.',
  signals:
    'A console and network panel showing a JavaScript error and a failed POST request with timings.',
  emulation:
    'The viewport morphs into a phone frame with device, timezone, and geolocation badges.',
  observe:
    'The run collapses into a timeline artifact: a screenshot filmstrip with a failure marker.',
};

/* ---------- primitives ---------- */

function Bar({
  w = 'w-10',
  h = 'h-1.5',
  tone,
  xray = false,
}: {
  w?: string;
  h?: string;
  tone?: string;
  xray?: boolean;
}) {
  const fill = xray
    ? 'border border-dashed border-focus/40 bg-transparent'
    : (tone ?? 'bg-edge');
  return <div className={`rounded-full ${w} ${h} ${fill}`} />;
}

function Chip({
  children,
  tone = 'text-cloud-light border-edge bg-ink/80',
}: {
  children: React.ReactNode;
  tone?: string;
}) {
  return (
    <span
      className={`whitespace-nowrap rounded-md border px-1.5 py-0.5 font-mono text-[9px] leading-4 md:text-[10px] ${tone}`}
    >
      {children}
    </span>
  );
}

function overlayMotion(reduce: boolean) {
  return {
    initial: { opacity: 0, y: 10 },
    animate: { opacity: 1, y: 0 },
    exit: { opacity: 0, y: 6 },
    transition: { duration: reduce ? 0 : 0.4, ease: EASE },
  };
}

/* ---------- the mock web app ---------- */

function MockApp({
  xray = false,
  typed = false,
  compact = false,
}: {
  xray?: boolean;
  typed?: boolean;
  compact?: boolean;
}) {
  const edge = xray ? 'border-dashed border-focus/30' : 'border-edge/70';
  return (
    <div className="flex h-full flex-col">
      {/* app header */}
      <div className={`flex h-8 shrink-0 items-center gap-2 border-b px-3 ${edge}`}>
        <div
          className={`size-2.5 rounded ${xray ? 'border border-dashed border-focus/40' : 'bg-clay'}`}
        />
        <Bar w="w-12" xray={xray} />
        <div className="ml-auto flex items-center gap-2">
          {!compact && (
            <>
              <Bar w="w-7" xray={xray} />
              <Bar w="w-7" xray={xray} />
            </>
          )}
          <div
            className={`size-3.5 rounded-full ${xray ? 'border border-dashed border-focus/40' : 'bg-edge'}`}
          />
        </div>
      </div>

      <div className="flex min-h-0 flex-1">
        {/* sidebar */}
        {!compact && (
          <aside className={`w-[22%] shrink-0 space-y-1 border-r p-2.5 ${edge}`}>
            <div
              className={`rounded px-1.5 py-1.5 ${xray ? '' : 'bg-clay/15'}`}
            >
              <Bar w="w-10" tone={xray ? undefined : 'bg-clay/70'} xray={xray} />
            </div>
            <div className="px-1.5 py-1.5">
              <Bar w="w-12" xray={xray} />
            </div>
            <div className="px-1.5 py-1.5">
              <Bar w="w-9" xray={xray} />
            </div>
            <div className="px-1.5 py-1.5">
              <Bar w="w-11" xray={xray} />
            </div>
          </aside>
        )}

        {/* main pane */}
        <main className="min-w-0 flex-1 space-y-2.5 p-3">
          <div className="flex items-center justify-between gap-2">
            <Bar w="w-20" h="h-2" tone="bg-cloud-light/60" xray={xray} />
            <div
              data-mock="new-project-header"
              className={`whitespace-nowrap rounded-md px-2 py-1 text-[9px] font-medium leading-none md:text-[10px] ${
                xray
                  ? 'border border-dashed border-focus/60 text-focus'
                  : 'bg-clay text-ink'
              }`}
            >
              New project
            </div>
          </div>

          {/* stats row */}
          {!compact && (
            <div className="grid grid-cols-3 gap-2">
              {['w-8', 'w-10', 'w-7'].map((w, i) => (
                <div
                  key={i}
                  className={`space-y-1.5 rounded-lg border p-2 ${
                    xray
                      ? 'border-dashed border-focus/25'
                      : 'border-edge/60 bg-slate/40'
                  }`}
                >
                  <Bar w={w} h="h-1" xray={xray} />
                  <Bar
                    w="w-12"
                    h="h-2"
                    tone={i === 0 ? 'bg-kraft/60' : 'bg-cloud-dark/60'}
                    xray={xray}
                  />
                </div>
              ))}
            </div>
          )}

          {/* form card */}
          <div
            className={`space-y-2 rounded-lg border p-2.5 ${
              xray ? 'border-dashed border-focus/40' : 'border-edge/70 bg-slate/60'
            }`}
          >
            <Bar w="w-12" h="h-1" xray={xray} />
            <div
              data-mock="email"
              className={`flex h-6 items-center rounded-md border px-2 font-mono text-[9px] md:h-7 md:text-[10px] ${
                typed
                  ? 'border-focus text-ivory ring-2 ring-focus/30'
                  : xray
                    ? 'border-dashed border-focus/50 text-focus/70'
                    : 'border-edge bg-ink text-cloud-dark'
              }`}
            >
              {typed ? (
                <>
                  sam@acme.dev
                  <span className="bl-caret ml-0.5 inline-block h-3 w-px bg-focus" />
                </>
              ) : xray ? (
                'input#email'
              ) : (
                'email'
              )}
            </div>
            <div className="flex justify-end">
              <div
                data-mock="submit"
                className={`rounded-md px-2 py-1 text-[9px] font-medium leading-none md:text-[10px] ${
                  xray
                    ? 'border border-dashed border-focus/60 text-focus'
                    : 'border border-edge bg-ink text-cloud-light'
                }`}
              >
                Submit
              </div>
            </div>
          </div>

          {/* list rows */}
          <div className="space-y-1">
            {(compact ? [0, 1] : [0, 1, 2, 3]).map((i) => (
              <div
                key={i}
                className={`flex items-center gap-2 rounded-md border px-2 py-1.5 ${
                  xray ? 'border-dashed border-focus/25' : 'border-edge/50'
                }`}
              >
                <div
                  className={`size-2 rounded-full ${xray ? 'border border-dashed border-focus/40' : 'bg-edge'}`}
                />
                <Bar
                  w={['w-16', 'w-12', 'w-20', 'w-14'][i]}
                  h="h-1"
                  xray={xray}
                />
                <div className="ml-auto">
                  <Bar w="w-6" h="h-1" xray={xray} />
                </div>
              </div>
            ))}
          </div>
        </main>
      </div>
    </div>
  );
}

/* ---------- per-layer overlays ---------- */

function XrayLabels({ reduce }: { reduce: boolean }) {
  const m = overlayMotion(reduce);
  return (
    <motion.div {...m} className="pointer-events-none absolute inset-0">
      <div className="absolute left-[3%] top-[16%]">
        <Chip tone="border-focus/50 bg-ink/90 text-focus">nav</Chip>
      </div>
      <div className="absolute right-[2%] top-[20%]">
        <Chip tone="border-focus/50 bg-ink/90 text-focus">
          role=button “New project”
        </Chip>
      </div>
      <div className="absolute left-[32%] top-[48%]">
        <Chip tone="border-focus/50 bg-ink/90 text-focus">input#email → @e4</Chip>
      </div>
      <div className="absolute bottom-[4%] right-[3%] rounded-lg border border-edge bg-ink/95 p-2.5 font-mono text-[9px] leading-4 text-cloud-light shadow-xl md:text-[10px]">
        <div className="mb-0.5 text-cloud">$ bl a11y-tree</div>
        <div>├ navigation “Projects”</div>
        <div>└ main</div>
        <div className="pl-3">├ textbox “Email”</div>
        <div className="pl-3">
          └ button <span className="text-focus">“Submit”</span>
        </div>
      </div>
    </motion.div>
  );
}

function CursorOverlay({ reduce }: { reduce: boolean }) {
  const m = overlayMotion(reduce);
  return (
    <motion.div {...m} className="pointer-events-none absolute inset-0">
      <svg
        className="absolute inset-0 h-full w-full"
        viewBox="0 0 100 100"
        preserveAspectRatio="none"
      >
        <path
          d="M 10 92 C 34 86, 58 76, 72 56"
          fill="none"
          stroke="var(--color-clay)"
          strokeWidth="0.5"
          strokeDasharray="1.6 2"
          opacity="0.9"
        />
      </svg>
      {/* click ripple + pointer near the Submit button */}
      <div className="absolute left-[70%] top-[52%]">
        <span className="motion-safe:animate-ping absolute -left-1.5 -top-1.5 size-5 rounded-full border border-clay/70" />
        <span className="absolute -left-1.5 -top-1.5 size-5 rounded-full border border-clay/40" />
        <svg width="15" height="15" viewBox="0 0 24 24" className="relative">
          <path
            d="M5 3l14 8-6.5 1.5L10 19 5 3z"
            fill="var(--color-ivory-light)"
            stroke="var(--color-ink)"
            strokeWidth="1.4"
          />
        </svg>
      </div>
      <div className="absolute left-[8%] top-[84%] hidden sm:block">
        <Chip tone="border-clay/50 bg-ink/90 text-kraft">
          bl click “button[type=submit]”
        </Chip>
      </div>
    </motion.div>
  );
}

function StateDrawer({ reduce }: { reduce: boolean }) {
  return (
    <motion.div
      initial={{ y: '100%' }}
      animate={{ y: 0 }}
      exit={{ y: '100%' }}
      transition={{ duration: reduce ? 0 : 0.45, ease: EASE }}
      className="absolute inset-x-0 bottom-0 border-t border-edge bg-ink/95 p-3 backdrop-blur"
    >
      <div className="mb-2 flex items-center justify-between font-mono text-[9px] text-cloud md:text-[10px]">
        <span className="flex items-center gap-1.5">
          <svg width="9" height="9" viewBox="0 0 24 24" fill="none">
            <rect
              x="4"
              y="10"
              width="16"
              height="11"
              rx="2"
              fill="var(--color-kraft)"
            />
            <path
              d="M8 10V7a4 4 0 118 0v3"
              stroke="var(--color-kraft)"
              strokeWidth="2.4"
            />
          </svg>
          session state
        </span>
        <span className="text-kraft">✓ state.json · restored next run</span>
      </div>
      <div className="flex flex-wrap gap-1.5">
        <Chip tone="border-kraft/40 bg-slate text-ivory">
          cookie · session=9f2c…7d1
        </Chip>
        <Chip>localStorage · theme=dark</Chip>
        <Chip>sessionStorage · cart=3</Chip>
        <Chip tone="border-edge bg-slate text-cloud">httpOnly · secure</Chip>
      </div>
    </motion.div>
  );
}

function SignalsPanel({ reduce }: { reduce: boolean }) {
  return (
    <motion.div
      initial={{ y: '100%' }}
      animate={{ y: 0 }}
      exit={{ y: '100%' }}
      transition={{ duration: reduce ? 0 : 0.45, ease: EASE }}
      className="absolute inset-x-0 bottom-0 border-t border-edge bg-ink/95 backdrop-blur"
    >
      <div className="flex items-center gap-3 border-b border-edge/70 px-3 py-1.5 font-mono text-[9px] md:text-[10px]">
        <span className="text-ivory">console</span>
        <span className="text-cloud">network</span>
        <span className="ml-auto text-cloud-dark">captured in run.zip</span>
      </div>
      <div className="grid gap-x-4 gap-y-0.5 p-3 font-mono text-[9px] leading-[1.7] md:text-[10px] lg:grid-cols-2">
        <div className="min-w-0">
          <div className="truncate text-cloud-light">
            <span className="text-cloud-dark">[log]</span> cart · 3 items
          </div>
          <div className="truncate text-kraft">
            <span className="text-cloud-dark">[warn]</span> slow response
            /api/cart
          </div>
          <div className="truncate text-danger">
            <span className="text-cloud-dark">[error]</span> TypeError: total is
            undefined
          </div>
          <div className="truncate pl-7 text-cloud-dark">
            at checkout.ts:214
          </div>
        </div>
        <div className="min-w-0">
          <div className="truncate text-cloud-light">
            GET /api/cart <span className="text-kraft">200</span>
            <span className="text-cloud-dark"> · 84 ms</span>
          </div>
          <div className="truncate text-cloud-light">
            GET /assets/app.js <span className="text-kraft">200</span>
            <span className="text-cloud-dark"> · 12 ms</span>
          </div>
          <div className="truncate text-danger">
            POST /api/checkout 500
            <span className="text-cloud-dark"> · 512 ms</span>
          </div>
        </div>
      </div>
    </motion.div>
  );
}

function PhoneOverlay({ reduce }: { reduce: boolean }) {
  const m = overlayMotion(reduce);
  return (
    <motion.div {...m} className="absolute inset-0">
      <div className="absolute inset-0 bg-ink/60" />
      <div className="absolute right-[3%] top-[4%] flex max-w-[55%] flex-wrap justify-end gap-1.5">
        <Chip tone="border-focus/40 bg-ink text-focus">390×844 · dpr 3</Chip>
        <Chip>Asia/Kolkata</Chip>
        <Chip>12.9716, 77.5946</Chip>
        <Chip>prefers-color-scheme: dark</Chip>
      </div>
      <div className="absolute left-[8%] top-1/2 w-[30%] min-w-[112px] -translate-y-1/2 overflow-hidden rounded-xl border border-edge bg-ink shadow-2xl md:left-[12%]">
        <div className="flex h-4 items-center justify-center border-b border-edge/60">
          <div className="h-1 w-8 rounded-full bg-edge" />
        </div>
        <div className="h-40 md:h-44">
          <MockApp compact />
        </div>
      </div>
    </motion.div>
  );
}

function EvidenceTimeline({ reduce }: { reduce: boolean }) {
  const m = overlayMotion(reduce);
  return (
    <motion.div
      {...m}
      className="absolute inset-0 flex flex-col justify-center gap-3 bg-ink p-4"
    >
      <div className="flex flex-wrap items-center gap-1.5 font-mono text-[9px] text-cloud md:text-[10px]">
        <span className="text-ivory">run.zip</span>
        <Chip>filmstrip</Chip>
        <Chip>snapshots</Chip>
        <Chip>console</Chip>
        <Chip>network</Chip>
      </div>
      <div className="flex gap-1.5">
        {[0, 1, 2, 3, 4].map((i) => (
          <div
            key={i}
            className={`flex aspect-[16/11] flex-1 flex-col gap-1 rounded-md border p-1.5 ${
              i === 3 ? 'border-danger/70 bg-danger/10' : 'border-edge bg-slate'
            }`}
          >
            <div className="h-1 w-3/5 rounded-full bg-edge" />
            <div className="h-1 w-4/5 rounded-full bg-edge/70" />
            <div className="mt-auto flex items-center justify-between">
              <div className="h-1 w-2/5 rounded-full bg-edge/70" />
              {i === 3 && (
                <span className="font-mono text-[9px] leading-none text-danger">
                  ✗
                </span>
              )}
            </div>
          </div>
        ))}
      </div>
      <div className="relative h-3">
        <div className="absolute inset-x-0 top-1/2 h-px bg-edge" />
        {[...Array(21)].map((_, i) => (
          <div
            key={i}
            className="absolute top-1/2 h-1 w-px -translate-y-1/2 bg-edge"
            style={{ left: `${i * 5}%` }}
          />
        ))}
        <div className="absolute left-[70%] top-1/2 h-3 w-0.5 -translate-y-1/2 rounded bg-danger" />
      </div>
      <div className="font-mono text-[9px] text-cloud md:text-[10px]">
        0:00 ── 0:12 · step 7/9{' '}
        <span className="text-danger">✗ expect text “#total” · exit 1</span>
      </div>
    </motion.div>
  );
}

/* ---------- browser chrome ---------- */

function TitleBar({ layer, reduce }: { layer: VisualLayer; reduce: boolean }) {
  const tabs = layer === 'tabs';
  return (
    <div className="flex h-9 items-center gap-2 border-b border-edge/70 bg-slate px-3">
      <span className="flex shrink-0 gap-1.5">
        <span className="size-2.5 rounded-full bg-edge" />
        <span className="size-2.5 rounded-full bg-edge" />
        <span className="size-2.5 rounded-full bg-edge" />
      </span>
      <div className="flex min-w-0 items-center gap-1">
        <div className="flex items-center gap-1.5 rounded-md bg-ink px-2 py-1 font-mono text-[9px] text-ivory md:text-[10px]">
          <span className="size-1.5 rounded-full bg-clay" />
          <span className="truncate">dashboard</span>
        </div>
        <AnimatePresence initial={false}>
          {tabs && (
            <motion.div
              key="extra-tabs"
              initial={{ opacity: 0, x: -8 }}
              animate={{ opacity: 1, x: 0 }}
              exit={{ opacity: 0, x: -8 }}
              transition={{ duration: reduce ? 0 : 0.35, ease: EASE }}
              className="flex min-w-0 items-center gap-1"
            >
              <div className="truncate rounded-md px-2 py-1 font-mono text-[9px] text-cloud md:text-[10px]">
                staging
              </div>
              <div className="hidden items-center gap-1 rounded-md px-2 py-1 font-mono text-[9px] text-cloud sm:flex md:text-[10px]">
                admin
                <span className="rounded-sm bg-kraft/20 px-1 text-[8px] text-kraft">
                  authed
                </span>
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
      <div className="ml-auto flex min-w-0 shrink items-center gap-1.5 rounded-md border border-edge/70 bg-ink px-2 py-1 font-mono text-[9px] text-cloud-light md:text-[10px]">
        <svg width="8" height="8" viewBox="0 0 24 24" fill="none" className="shrink-0">
          <rect x="4" y="10" width="16" height="11" rx="2" fill="var(--color-cloud)" />
          <path d="M8 10V7a4 4 0 118 0v3" stroke="var(--color-cloud)" strokeWidth="2.4" />
        </svg>
        <span className="truncate">app.example.com</span>
      </div>
    </div>
  );
}

/* ---------- main component ---------- */

export function BrowserStackVisual({
  layer,
  className = '',
}: {
  layer: VisualLayer;
  className?: string;
}) {
  const reduce = useReducedMotion() ?? false;
  const meta = LAYERS.find((l) => l.id === layer);

  return (
    <div
      role="img"
      aria-label={DESCRIPTIONS[layer]}
      className={`relative ${className}`}
    >
      <div aria-hidden className="relative">
        {/* stacked context planes (tabs layer) */}
        <AnimatePresence initial={false}>
          {layer === 'tabs' && (
            <>
              <motion.div
                key="plane-2"
                initial={{ opacity: 0, x: 0, y: 0 }}
                animate={{ opacity: 0.5, x: 24, y: -20 }}
                exit={{ opacity: 0, x: 0, y: 0 }}
                transition={{ duration: reduce ? 0 : 0.45, ease: EASE }}
                className="absolute inset-0 rounded-2xl border border-edge bg-slate/70"
              />
              <motion.div
                key="plane-1"
                initial={{ opacity: 0, x: 0, y: 0 }}
                animate={{ opacity: 0.8, x: 12, y: -10 }}
                exit={{ opacity: 0, x: 0, y: 0 }}
                transition={{ duration: reduce ? 0 : 0.45, ease: EASE }}
                className="absolute inset-0 rounded-2xl border border-edge bg-slate"
              />
            </>
          )}
        </AnimatePresence>

        {/* the browser frame */}
        <div
          className={`relative overflow-hidden rounded-2xl border bg-slate shadow-[0_32px_80px_-32px_rgba(0,0,0,0.85)] transition-shadow duration-500 ${
            layer === 'shell'
              ? 'border-clay/50 shadow-[0_0_70px_-24px_rgba(204,120,92,0.55),0_32px_80px_-32px_rgba(0,0,0,0.85)]'
              : 'border-edge'
          }`}
        >
          <TitleBar layer={layer} reduce={reduce} />

          <div className="relative aspect-16/10 overflow-hidden bg-ink">
            <motion.div
              className="h-full"
              animate={{
                opacity: layer === 'emulation' ? 0.35 : 1,
                scale: layer === 'emulation' ? 0.98 : 1,
              }}
              transition={{ duration: reduce ? 0 : 0.4, ease: EASE }}
            >
              <MockApp xray={layer === 'dom'} typed={layer === 'inputs'} />
            </motion.div>

            <AnimatePresence initial={false} mode="popLayout">
              {layer === 'dom' && <XrayLabels key="xray" reduce={reduce} />}
              {layer === 'inputs' && (
                <CursorOverlay key="cursor" reduce={reduce} />
              )}
              {layer === 'state' && <StateDrawer key="state" reduce={reduce} />}
              {layer === 'signals' && (
                <SignalsPanel key="signals" reduce={reduce} />
              )}
              {layer === 'emulation' && (
                <PhoneOverlay key="phone" reduce={reduce} />
              )}
              {layer === 'observe' && (
                <EvidenceTimeline key="evidence" reduce={reduce} />
              )}
            </AnimatePresence>
          </div>

          {/* session bar */}
          <div className="flex h-7 items-center justify-between border-t border-edge/70 bg-slate px-3 font-mono text-[9px] text-cloud md:text-[10px]">
            <span className="flex items-center gap-1.5 truncate">
              <span className="size-1.5 shrink-0 rounded-full bg-kraft" />
              chrome for testing
              <span className="hidden sm:inline"> · webdriver bidi</span>
            </span>
            <span className="ml-3 shrink-0 text-cloud-dark">
              {meta ? `${meta.index} · ${meta.name.toLowerCase()}` : 'bl daemon · warm'}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}

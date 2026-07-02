export type LayerId =
  | 'shell'
  | 'tabs'
  | 'dom'
  | 'inputs'
  | 'state'
  | 'signals'
  | 'emulation'
  | 'observe';

export interface Layer {
  id: LayerId;
  index: string;
  name: string;
  title: string;
  body: string;
  /** Real `bl` commands only — every snippet is verified against the CLI. */
  commands: string[];
}

export const LAYERS: Layer[] = [
  {
    id: 'shell',
    index: '01',
    name: 'Browser shell',
    title: 'Start with a real browser.',
    body: 'One command launches Chrome over WebDriver BiDi — visible while you build, headless in CI. Navigate, wait, reload, capture. Every run is a real render of your real app, not a DOM emulation.',
    commands: ['bl go https://app.example.com', 'bl screenshot -o home.png'],
  },
  {
    id: 'tabs',
    index: '02',
    name: 'Tabs & contexts',
    title: 'Every tab is a handle.',
    body: 'Pages are enumerable and addressable — open, list, and switch by index or URL. A warm daemon keeps the session alive between commands, so multi-tab flows stay scriptable.',
    commands: ['bl page new https://app.example.com/admin', 'bl pages', 'bl page switch admin'],
  },
  {
    id: 'dom',
    index: '03',
    name: 'Page & DOM',
    title: 'The page, made legible.',
    body: 'Query by CSS, role, label, text, or XPath. Map every interactive element to a stable @ref an agent can act on, and read the accessibility tree the way assistive tech does.',
    commands: ['bl map', 'bl find role button', 'bl a11y-tree'],
  },
  {
    id: 'inputs',
    index: '04',
    name: 'Human inputs',
    title: 'Input that behaves like a person.',
    body: 'Real pointer and keyboard events with actionability checks built in: click, type, fill, drag, press. If a human can do it in the page, an agent can script it — and the app can’t tell the difference.',
    commands: [
      'bl fill "#email" "sam@acme.dev"',
      'bl click "button[type=submit]"',
      'bl drag "#task-3" "#done"',
    ],
  },
  {
    id: 'state',
    index: '05',
    name: 'State & auth',
    title: 'Sessions you can carry.',
    body: 'Export cookies, localStorage, and sessionStorage as JSON; restore them in the next run. Log in once, then test authenticated flows on every run after — no scripted logins.',
    commands: ['bl storage -o state.json', 'bl storage restore state.json'],
  },
  {
    id: 'signals',
    index: '06',
    name: 'Console & network',
    title: 'See what the app was doing.',
    body: 'Recordings capture console output and network activity alongside snapshots — the failed POST and the stack trace behind it travel with the run, not just the final pixels.',
    commands: ['bl record start --snapshots', 'bl record stop -o run.zip'],
  },
  {
    id: 'emulation',
    index: '07',
    name: 'Emulation',
    title: 'Test the environment too.',
    body: 'Resize to any device, override geolocation, and force CSS media features like dark mode or reduced motion — per session, without touching app code.',
    commands: [
      'bl viewport 390 844 --dpr 3',
      'bl geolocation 12.9716 77.5946',
      'bl media --color-scheme dark',
    ],
  },
  {
    id: 'observe',
    index: '08',
    name: 'Recording & observability',
    title: 'Every run leaves evidence.',
    body: 'Assertions with real exit codes, structured diffs between steps, annotated screenshots, replayable recordings. When a run fails, you open the evidence — you don’t reproduce the failure.',
    commands: ['bl expect url contains "/done"', 'bl diff map', 'bl record stop -o evidence.zip'],
  },
];

# browserlane CLI test plan & coverage tracker

The **standard, ongoing** test plan for the `bl` command-line surface — every
command, subcommand, and argument. It is split into two halves:

- **`scripts/cli-smoke.sh`** — the executable harness. Drives the built binary
  through every command path and asserts the observable contract. **This is the
  source of truth for what is covered**; the matrix below mirrors it.
- **This document** — the human-readable plan: the coverage model, the matrix,
  the manual checklist for what can't be auto-verified, and the process for
  keeping both current as new commands land.

It complements the [`MANUAL-TEST.md`](MANUAL-TEST.md) per-OS, pre-publish
**release sign-off**, it does not replace it. The harness verifies the *CLI
layer* (arg parsing, output shape, exit codes, `--json`, help/errors); the
manual pass covers the visual / interactive items it can't.

---

## How to run

```bash
cargo build --release
bl install                       # one-time Chrome-for-Testing download
bash scripts/cli-smoke.sh        # headless; exit 0 iff all checks + coverage pass
BL_VISIBLE=1 bash scripts/cli-smoke.sh   # watch it drive a real window
```

One section navigates to `example.com` (cookies/storage need an http origin), so
a network connection is required. The harness is bash-3.2 compatible (macOS
system bash). Current status: **112 auto checks green · 9 manual · 110/110 live
command paths tracked**.

---

## Coverage model — the four tiers

Each check is labelled with the depth of verification it actually achieves. This
is deliberate: it prevents a false sense of "100%".

| Tier | Meaning | Example |
|---|---|---|
| **AUTO** | Verified from stdout / exit code / `--json`, deterministically. | `bl title` → `BL Fixture` |
| **PROXY** | Verified indirectly by reading page state back with `bl eval`. Confirms the *mechanism* fired — not the rendered pixels. | `bl click "#cb"` → `eval "…cb.checked"` is `true` |
| **ARTIFACT** | A file was produced and is well-formed for its type (PNG/PDF/zip header, non-trivial size) — not that its *content* is correct. | `bl pdf -o x.pdf` → file starts with `%PDF` |
| **MANUAL** | Cannot be auto-verified here (visual fidelity, interactive/long-running transports, OS window state, or another OS). Listed, executed by a human. | `bl highlight` red outline; `bl serve` |

**Why never 100% automated:** (1) pixel/render fidelity needs eyes, (2) Windows
code paths can't run on macOS, (3) a few commands are interactive/streaming. The
harness reaches ~80% behavioral verification; the rest is the manual checklist below.

---

## The self-coverage gate (how future commands stay tracked)

After running, the harness enumerates the binary's **live visible command tree**
(`bl --help`, recursively) and fails if any path is neither tested nor registered
`MANUAL`:

```
== coverage gate ==
  OK all 109 live command paths are tracked
```

If you add a command under `src/ext/` and don't add a test, the gate prints
`UNTRACKED <command>` and the run fails. **This is the "tracking" guarantee:
the test plan cannot silently fall behind the binary.**

### Process when you add or change a CLI command

1. Add the command in `src/ext/cli.rs` (per [CONTRIBUTING.md](CONTRIBUTING.md)).
2. Add one line to `scripts/cli-smoke.sh` in the matching phase:
   - deterministic output → `check AUTO "<path>" "<expect>" -- bl <path> …`
   - changes page state → `proxy "<path>" "<expect>" "<eval-js>" -- bl <path> …`
   - writes a file → `artifact "<path>" <file> <kind> -- bl <path> …`
   - visual/interactive → `manual "<path>" "<why>"` **and** a row in the manual
     checklist below.
3. Add a row to the matrix here.
4. Run `bash scripts/cli-smoke.sh` — it must end `SMOKE OK`.

---

## Coverage matrix

Ordered as the harness runs (dependency order — the shared daemon session means
order matters). Args/flag variants exercised are noted.

### Phase 1 — meta (no browser)
| Path | Tier | Asserted |
|---|---|---|
| `version` / `--version` | AUTO | `bl v0.1.1` |
| `inspect` *(ext)* | AUTO | JSON `name=browserlane` |
| `paths` | AUTO | exit 0 |
| `is-installed` | AUTO | exit 0 (Chrome present) |
| `--help` / `help` | AUTO | lists `Available Commands` |
| `completion bash\|zsh\|fish\|powershell` | AUTO | shell-specific token present |
| `add-skill` (`--stdout`) | AUTO | non-empty skill text |
| `add-mcp` (`--list`) *(ext)* | AUTO | lists MCP clients (`claude`) |
| `install` | MANUAL | run once; verified via `is-installed` |
| *(error)* `bl clik` | AUTO | `Did you mean … click` (cobra-exact) |
| *(error)* `bl zzzznope` | AUTO | `unknown command`, exit 1 |

### Phase 2 — daemon
| Path | Tier | Asserted |
|---|---|---|
| `daemon start` (`--headless`) | AUTO | exit 0; readiness-waited |
| `daemon status` | AUTO | `running` |
| `daemon status --json` | AUTO | JSON contains `0.1.1` |
| `daemon stop` | AUTO | *(phase 11)* |

### Phase 3 — read-only inspection (on a local fixture)
| Path | Tier | Asserted |
|---|---|---|
| `go` / `url` | AUTO | URL reflects navigation |
| `title` | AUTO | `BL Fixture` |
| `text "#h"` | AUTO | `BL Fixture` |
| `html "#h"` (+`--outer`) | AUTO | inner vs `<h1` outer |
| `attr "#lnk" href` | AUTO | the href |
| `value "#sel"` | AUTO | `one` |
| `count "input"` | AUTO | exit 0 |
| `find` (+`--all --limit`) | AUTO | exit 0 |
| `find role` (+`--name`) / `text` / `label` / `placeholder` / `alt` / `title` / `testid` / `xpath` | AUTO | exit 0 each |
| `is visible` / `enabled` / `checked` | AUTO | `true` / `true` / `false` |
| `is actionable` | AUTO | needs `[url] [selector]`; `actionab…` |
| `a11y-tree` (+`--everything`) | AUTO | exit 0 |
| `map` (+`--selector`) | AUTO | exit 0 |
| `pages` / `frames` / `frame` | AUTO | exit 0 |

### Phase 4 — wait
| Path | Tier | Asserted |
|---|---|---|
| `wait` (`--state --timeout`) / `wait load` / `text` / `url` / `fn` | AUTO | resolves (exit 0) |

### Phase 5 — interaction
| Path | Tier | Asserted |
|---|---|---|
| `click` | PROXY | checkbox `checked=true` |
| `uncheck` / `check` | PROXY | `false` / radio `true` |
| `type` / `fill` | PROXY | input `value` |
| `select` | PROXY | `sel.value=two` |
| `focus` | PROXY | `document.activeElement` matches |
| `hover` / `dblclick` | PROXY | `onmouseover` / `ondblclick` flag set |
| `scroll` (warm-up + `--amount`) | PROXY | `scrollY>0` |
| `scroll into-view` | PROXY | element rect in viewport |
| `press` / `keys` | AUTO | exit 0 |
| `mouse move` / `down` / `up` / `click` (`--button`) | AUTO | exit 0 |
| `highlight` | AUTO | exit 0 *(red-outline visual → MANUAL)* |

### Phase 6 — emulation
| Path | Tier | Asserted |
|---|---|---|
| `viewport` (+`--dpr`) | PROXY | `innerWidth=1280` |
| `window` (+`--state`) | AUTO | exit 0 |
| `media --color-scheme dark` | PROXY | `matchMedia(...).matches` |
| `media` (other flags) | AUTO | exit 0 |
| `geolocation` (+`--accuracy`) | AUTO | exit 0 — negative lon needs `--` |

### Phase 7 — capture
| Path | Tier | Asserted |
|---|---|---|
| `screenshot` (`-o`, `--full-page`, `--annotate`) | ARTIFACT | reported path is a PNG |
| `pdf -o` | ARTIFACT | file starts `%PDF` |

### Phase 8 — storage / state (needs http origin)
| Path | Tier | Asserted |
|---|---|---|
| `cookies <n> <v>` / `cookies` / `cookies clear` | AUTO | exit 0 |
| `storage -o` | ARTIFACT | JSON file |
| `storage restore` | AUTO | exit 0 |
| `diff` / `diff map` | AUTO | exit 0 |

### Phase 9 — stateful (upload / download / record / eval)
| Path | Tier | Asserted |
|---|---|---|
| `upload "#file" <f>` | PROXY | `file.files.length=1` |
| `download` / `download dir` | AUTO | exit 0 |
| `record start` → `record chunk start` → `record chunk stop -o` → `record group start` → `record group stop` → `record stop -o` | AUTO + ARTIFACT | nested; chunk.zip + rec.zip produced |
| `eval` (inline + `--stdin`) | AUTO | `1+1`→2, stdin `3+4`→7 |
| `sleep` | AUTO | exit 0 |

### Phase 10 — content (destructive) · multipage · navigation
| Path | Tier | Asserted |
|---|---|---|
| `content "<h1>NEW</h1>"` | PROXY | h1 text = `NEW` |
| `page new` / `pages` / `page switch` / `page close` | AUTO | exit 0 |
| `back` / `forward` | PROXY | `document.title` history |
| `reload` | AUTO | exit 0 |

### Phase 11 — lifecycle / teardown
| Path | Tier | Asserted |
|---|---|---|
| `start` / `stop` | AUTO | exit 0 |
| `daemon stop` | AUTO | exit 0 |
| *auto-restart* (`bl go` after stop) | AUTO | daemon re-spawns |

### MCP surface
| Path | Tier | Asserted |
|---|---|---|
| `mcp` | AUTO | scripted JSON-RPC `initialize` + `tools/list` returns the tool array |

### MANUAL (registered, not auto-executed) — see checklist below
`launch-test`, `bidi-test`, `ws-test`, `drag`, `dialog accept`, `dialog dismiss`,
`pipe`, `serve`, `install`.

---

## Manual checklist (the irreducible ~20%)

Run these by hand in **visible** mode (`BL_VISIBLE=1`), on **both macOS and
Windows** for the per-OS items. Tick during a release pass.

**Visual fidelity** (auto-run confirms the mechanism; eyes confirm the result):
- [ ] `bl highlight "#h"` — red outline appears for ~3s.
- [ ] `bl screenshot --full-page` — image extends beyond the viewport.
- [ ] `bl screenshot --annotate` — interactive elements have numbered labels.
- [ ] `bl pdf` — opens and renders correctly.
- [ ] `bl hover` / `bl drag` — hover state / drag gesture look right.
- [ ] `bl window --state maximized|minimized|fullscreen` — OS window changes.
- [ ] `bl geolocation 40.7128 -- -74.0060` then a maps site shows NYC.
- [ ] `bl media --color-scheme dark` — a real site flips to dark.

**Interactive / long-running** (not scriptable in the harness):
- [ ] `bl dialog accept` / `dismiss` — trigger an `alert()`/`confirm()` and handle it.
- [ ] `bl drag <src> <dst>` — on a real drag-and-drop page.
- [ ] `bl launch-test` / `bl bidi-test` — print a `ws://` URL / `session.status`.
- [ ] `bl ws-test <url>` — interactive echo; type a line, see it echoed.
- [ ] `bl serve -p 9515` — a client can connect; Ctrl-C stops it.

**Windows-only** (distinct code paths — a macOS run cannot cover these):
- [ ] `bl daemon status --json` shows the named pipe `\\.\pipe\browserlane`.
- [ ] `bl pipe` exits 1 with `pipe mode is not supported on Windows`.
- [ ] `bl install` fetched `chrome.exe` into the `bl paths` cache dir.

---

## Contract findings (discovered while building the harness)

Behaviors the harness had to accommodate. A few may be bugs worth filing:

1. **`screenshot -o` ignores the directory.** It writes `<basename>` into the
   screenshot gallery dir (`~/Pictures/browserlane` on macOS), not the path given —
   while `pdf -o` honors an absolute path. Inconsistent; possible bug.
2. **`geolocation` rejects a negative longitude** unless preceded by `--`
   (`bl geolocation -- 40.7 -74.0`); clap parses `-74` as a flag. Possible
   cobra→clap parity gap (cobra is lenient with negative-number args).
3. **`is actionable` requires both `[url]` and `[selector]`** despite the help
   text showing them as optional.
4. **`record chunk` / `record group` require an active `record start`** — they
   error `no recording in progress` otherwise.
5. **The first `scroll` after a navigation is a no-op**; subsequent scrolls work.
6. **`cookies` / `storage` require an http(s) origin** — they fail on `file://`.

---

## Optional: wire into CI

To enforce the CLI gate on every push, add a job to `.github/workflows/build.yml`
(Linux, headless — Chrome-for-Testing runs headless without a display server):

```yaml
  cli-smoke:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@1.92.0
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --release
      - run: ./target/release/bl install
      - run: bash scripts/cli-smoke.sh
```

(The `example.com` navigation needs network egress, which GitHub runners have.)

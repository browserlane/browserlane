# browserlane

> Browser automation for humans and AI agents, via CLI and MCP, built on WebDriver BiDi in Rust.

[![build](https://github.com/browserlane/browserlane/actions/workflows/build.yml/badge.svg)](https://github.com/browserlane/browserlane/actions/workflows/build.yml)
[![release](https://img.shields.io/github/v/release/browserlane/browserlane?sort=semver)](https://github.com/browserlane/browserlane/releases/latest)
[![license](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)

`bl` is a single-binary browser-automation tool with two surfaces:
- A **CLI** for humans and scripts (navigate, click, screenshot, find, ...)
- An **MCP server** for AI agents (86 tools over stdio JSON-RPC)

It drives Chrome via the WebDriver BiDi protocol, installs its own Chrome for
Testing with one command, and ships as one static binary on macOS, Linux, and
Windows.

## Why browserlane

Driving a browser from code isn't new — [Playwright](https://playwright.dev),
Selenium, Cypress, and WebdriverIO have done it for years. And AI agents now have
their own browser tools: Playwright MCP, browser-use, Stagehand, and more.

The difference is in *how* they drive the browser. The agent-facing tools almost
all sit on the **Chrome DevTools Protocol (CDP)** — Chrome's vendor-specific
side-channel, the same plumbing Playwright and Puppeteer use underneath. The
tools that speak the newer, W3C-standard
**[WebDriver BiDi](https://w3c.github.io/webdriver-bidi/)** protocol — Selenium,
WebdriverIO — are built for human test automation and ship as libraries on a Node
or JVM runtime.

browserlane sits in that gap: an **agent-native** tool — a human CLI *and* an MCP
server — that drives Chrome **exclusively over WebDriver BiDi**, as a **single
Rust binary** with no runtime to install. BiDi is the bidirectional, W3C-standard
protocol the browser vendors are converging toward, so browserlane is built on
the standard rather than a vendor-specific side-channel.

## Install

### One-line install (recommended)

**macOS / Linux**

```bash
curl -fsSL https://browserlane.com/install.sh | sh
```

**Windows** (PowerShell)

```powershell
irm https://browserlane.com/install.ps1 | iex
```

The installer detects your platform, verifies the download's checksum, installs
`bl`, and puts it on your `PATH`. **Re-run any time to update.**

### Manual download

Or grab an archive from the
[latest release](https://github.com/browserlane/browserlane/releases/latest),
extract it, and put `bl` on your `PATH`:

| Platform | Asset |
|---|---|
| macOS (Apple Silicon) | `bl-*-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `bl-*-x86_64-apple-darwin.tar.gz` |
| Linux (x64) | `bl-*-x86_64-unknown-linux-gnu.tar.gz` |
| Windows (x64) | `bl-*-x86_64-pc-windows-msvc.zip` |

### Build from source

Requires Rust stable.

```bash
git clone https://github.com/browserlane/browserlane
cd browserlane
cargo build --release
./target/release/bl --version
```

`bl install` downloads Chrome-for-Testing into the platform cache dir. Run it
once before your first browser command — `bl` doesn't fetch Chrome on demand.

## Quickstart — CLI

```bash
bl install                              # download Chrome for Testing
bl go https://example.com               # open a URL
bl screenshot -o page.png               # capture
bl find role button                     # locate "Submit" buttons by ARIA role
bl click "button[type=submit]"          # interact
bl eval "document.title"                # run JS
bl expect title contains "Example"      # assert page state (exit 0/1)
bl --help                               # all commands
```

The browser is visible by default. Pass `--headless` to hide it.

## Quickstart — MCP (AI agents)

`bl mcp` is an MCP server (JSON-RPC 2.0 over stdio). Wire it into your agent with
one command — `bl` registers itself by **absolute path**, so it works regardless
of `PATH`:

```bash
bl add-mcp claude          # Claude Code
bl add-mcp claude-desktop  # Claude Desktop
bl add-mcp cursor          # Cursor
bl add-mcp vscode          # VS Code (.vscode/mcp.json)
bl add-mcp codex           # OpenAI Codex CLI
```

`bl add-mcp --list` shows every client; add `--stdout` to print the config
instead of writing it. Run **`bl install` once first** — the first browser
action needs Chrome for Testing in the local cache (`bl` doesn't fetch it on
demand).

Then ask your agent things like "open example.com, click the second link,
screenshot the result". The 86 tools cover navigation, interaction, capture,
recording, emulation, assertions, the page clock, and more — see
`bl mcp` → `tools/list` for the full catalog.

## Daemon

`bl daemon start` keeps Chrome warm across CLI invocations (sub-second startup
instead of relaunching per command). `bl daemon status` / `bl daemon stop`.

## Platforms

CI builds + smoke-tests on every push:

| OS | Status |
|---|---|
| macOS (`macos-latest`) | ✅ |
| Linux (`ubuntu-latest`) | ✅ |
| Windows (`windows-latest`) | ✅ |

The daemon IPC uses a Unix-domain socket on macOS/Linux and a named pipe on
Windows (`\\.\pipe\browserlane`).

## Architecture

```
src/            ← the core engine (BiDi transport, browser, API, CLI, MCP server)
src/ext/        ← browserlane-specific commands + MCP tools, behind a stable seam
scripts/        ← cli-smoke.sh (the CLI test harness)
```

Custom commands and MCP tools live in `src/ext/` behind a four-hook seam, so
they're cleanly separated from the core. See [AGENTS.md](AGENTS.md) and
[CONTRIBUTING.md](CONTRIBUTING.md) for the layout and how to add features.

## Status

**v0.1.3** — 66 CLI commands, 86 MCP tools, a persistent
daemon, and a self-checking CLI test harness (`scripts/cli-smoke.sh`). Signed,
notarized prebuilt binaries ship for macOS, Linux, and Windows (via the one-line
installer above); a crates.io release is on the roadmap.

## License

Apache-2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE).

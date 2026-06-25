# AGENTS.md — browserlane maintenance guide

This file orients AI coding agents (and humans) working on browserlane. For
contributor docs see [CONTRIBUTING.md](CONTRIBUTING.md); for the project
overview see [README.md](README.md).

## What browserlane is

A Rust browser-automation binary (`bl`) with two surfaces — a CLI for humans and
an MCP server for AI agents — that drives Chrome exclusively over WebDriver BiDi,
as a single static binary. (See [NOTICE](NOTICE) for third-party attribution.)

## Code layout

```
src/         ← the core engine
  main.rs       CLI entry + command wiring (clap)
  cmd/          one module per CLI command (+ help_text.rs: captured help)
  agent/        MCP server: schema.rs (tool catalog), handlers.rs (dispatch),
                server.rs (stdio JSON-RPC loop)
  api/          the automation engine: router.rs + handlers_* grouped by domain
                (navigation, interaction, capture, network, recording, clock,
                storage, a11y, emulation, …)
  bidi/         WebDriver BiDi protocol layer (the WebSocket conversation w/ Chrome)
  browser/      Chrome-for-Testing installer + launcher
  daemon/       the persistent daemon (keeps Chrome warm) + its IPC
  errors/ log/ paths/ process/   supporting infra
src/ext/     ← browserlane-specific commands + MCP tools, behind a four-hook seam
```

Both surfaces — the CLI (`cmd/`) and the MCP server (`agent/`) — call into the
shared engine in `api/`, which speaks BiDi to Chrome via `bidi/`.

## The ext seam — exactly four hooks

New CLI commands and MCP tools go in `src/ext/`, wired through four one-line
hooks (each tagged `// ext-seam`):

| File | Hook | Purpose |
|---|---|---|
| `src/main.rs` (`build_cli`) | `ext::register_cli(cli)` | add CLI subcommands |
| `src/main.rs` (dispatch `match`) | `ext::dispatch_cli(...)` | dispatch them |
| `src/agent/schema.rs` (`get_tool_schemas`) | `ext::register_mcp_tools(&mut tools)` | add MCP tools |
| `src/agent/handlers.rs` (`dispatch`) | `ext::dispatch_mcp_tool(name, args)` | handle them |

`src/ext/cli.rs` (the `bl add-mcp` command) is a worked example. The seam is a
convenience for cleanly-separated additions — you can also edit the core
directly.

## House rules

- Rust 2021; match the surrounding style.
- Verify before declaring done:
  ```bash
  cargo build --release
  cargo clippy --release -- -D warnings -A clippy::module_inception
  bash scripts/cli-smoke.sh        # drives every command + self-checks coverage
  ```
- macOS, Linux, and Windows all build green. The daemon IPC uses
  `interprocess::local_socket` (Unix-domain socket on unix, named pipe on
  Windows) behind a single `Conn` type, so it's platform-agnostic.
- Paths and identity use the `browserlane` token: cache dir
  `~/.cache | Library/Caches/browserlane`, named pipe `\\.\pipe\browserlane`,
  env vars `BROWSERLANE_*`.

## Product scope

- **Client libraries** (JS/Python/Java/etc.) are out of scope — browserlane
  ships CLI + MCP only. The hidden `pipe` / `serve` library-transport commands
  (`.hide(true)`) exist but aren't a supported surface; don't extend them.

## Helpful files

- `CONTRIBUTING.md` — setup + how to add a feature
- `README.md` — what browserlane is, install, quickstart
- `CLI-TEST-PLAN.md` — the CLI test plan + coverage tracker
- `scripts/cli-smoke.sh` — the CLI test harness (the standard CLI gate)
- `NOTICE` — third-party license attribution
- `.github/workflows/build.yml` — CI: build + clippy + smoke on macOS/Linux/Windows

# Contributing to browserlane

Thanks for your interest! browserlane is a single-binary browser-automation tool
(`bl`) with two surfaces — a CLI and an MCP server — that drives Chrome over
WebDriver BiDi, written in Rust.

The code splits into the **core engine** under `src/` and **browserlane-specific
additions** under `src/ext/`, wired through a small four-hook seam.

## Local setup

```bash
git clone https://github.com/browserlane/browserlane
cd browserlane
cargo build --release
./target/release/bl --version       # bl v0.1.1
```

The first `bl install` (or any browser command) downloads Chrome-for-Testing.

## Add a feature (CLI command, MCP tool, behavior)

New commands and tools go in `src/ext/`, behind the seam:

| Hook | Where | What it does |
|---|---|---|
| `ext::register_cli(cli)` | called from `build_cli` | add clap subcommands |
| `ext::dispatch_cli(...)` | called from `main`'s `match` | dispatch them |
| `ext::register_mcp_tools(&mut tools)` | called from `agent::schema` | add MCP tools |
| `ext::dispatch_mcp_tool(name, args)` | called from `agent::handlers` | handle them |

See `src/ext/cli.rs` and `src/ext/mcp.rs` — the `bl inspect` command is a worked
example. You can also edit the core directly; the seam is just a convenient,
cleanly-separated home for additions.

## Verify your change

```bash
cargo build --release
cargo clippy --release -- -D warnings -A clippy::module_inception   # must pass
bash scripts/cli-smoke.sh        # drives every command + checks coverage
```

`scripts/cli-smoke.sh` is the standard CLI gate: it runs every command path with
tiered assertions and fails if any command isn't covered by a test — so a new
command can't silently escape the plan. See [CLI-TEST-PLAN.md](CLI-TEST-PLAN.md).

## Style

- Rust 2021, edition stable.
- Match the surrounding style.

## Reporting issues

Please include: OS, `bl --version`, the exact command, the observed output, and
what you expected.

## License

By contributing, you agree your work is licensed under Apache-2.0 (the project's
license). See [LICENSE](LICENSE) and [NOTICE](NOTICE).

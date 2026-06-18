# Pre-publish manual test checklist (v0.1.0)

Tick this off **on both macOS and Windows** before flipping the repo public.
Linux is covered by CI's `--version` + `inspect` smoke. The browser is visible
by default; the daemon auto-starts; close it
between sections with `bl daemon stop` if anything misbehaves.

> **Quick start each session:**
> ```bash
> cd ~/Documents/browserlane && cargo build --release
> export PATH="$PWD/target/release:$PATH"    # so `bl` works as a bare command
> bl install                                  # one-time Chrome-for-Testing download
> ```

## Known content TODO before public

- [x] `bl add-skill --stdout` — the embedded skill is fully rebranded to
  browserlane (skill `name:`, all `bl` command examples, install dir
  `~/.claude/skills/browserlane/`). Done.

## Per-OS checklist (run twice — once on Mac, once on Windows)

OS under test: **____________**     `bl --version`: **____________**

### 1. Foundations
- [ ] `bl --version` → `bl v0.1.0`
- [ ] `bl --help` lists ~67 commands, no errors
- [ ] `bl inspect` → JSON with `target_os` and `target_arch` matching this machine
- [ ] `bl paths` prints platform-appropriate cache/screenshot dirs
- [ ] `bl is-installed` → exit 0 (after `bl install`)

### 2. Navigation
- [ ] `bl go https://example.com` opens a visible browser, loads page
- [ ] `bl url` → `https://example.com/`
- [ ] `bl title` → `Example Domain`
- [ ] `bl back` / `bl forward` work on a multi-page session
- [ ] `bl reload` reloads

### 3. Inspection (no clicks)
- [ ] `bl find role link` → returns refs like `@e1`, `@e2`
- [ ] `bl find text "More information"` → returns the link
- [ ] `bl count "a"` → integer count
- [ ] `bl text "h1"` → `Example Domain`
- [ ] `bl attr "a" "href"` → URL
- [ ] `bl a11y-tree` → tree printout
- [ ] `bl content` → HTML body

### 4. Interaction (visible browser)
- [ ] `bl click "a"` clicks the More Information link
- [ ] `bl fill 'input[name="q"]' "hello"` on a search page works
- [ ] `bl type` on a form input works
- [ ] `bl hover "a"` triggers hover state
- [ ] `bl mouse click 100 200` clicks at coords
- [ ] `bl scroll down --amount 3` scrolls
- [ ] `bl press Enter` works
- [ ] `bl focus "input"` focuses

### 5. Capture
- [ ] `bl screenshot -o /tmp/test.png` saves a PNG you can open
- [ ] `bl pdf -o /tmp/test.pdf` saves a PDF
- [ ] `bl screenshot -o /tmp/full.png --full-page` captures beyond the viewport

### 6. Emulation
- [ ] `bl viewport 1280 720` resizes
- [ ] `bl window 1920 1080` resizes
- [ ] `bl geolocation 40.7128 -74.0060` then a maps page shows NYC
- [ ] `bl media --color-scheme dark` flips a site to dark mode

### 7. Storage
- [ ] `bl cookies` lists cookies
- [ ] `bl cookies session abc123` sets one
- [ ] `bl cookies clear` clears all
- [ ] `bl storage -o /tmp/state.json` exports session state

### 8. Page state
- [ ] `bl pages` lists open tabs
- [ ] `bl page new https://www.google.com` opens a new tab
- [ ] `bl page switch 0` switches back
- [ ] `bl page close 1` closes the second tab
- [ ] `bl frames` works on a page with iframes

### 9. Daemon
- [ ] `bl daemon start` → "Daemon started (pid …)"
- [ ] `bl daemon status` → running, version 0.1.0
- [ ] `bl daemon status --json` → valid JSON
- [ ] After `bl daemon stop`, `bl daemon status` → "Daemon is not running"
- [ ] Second `bl go ...` after the daemon stops works (auto-starts)

### 10. MCP (the AI-agent surface)
- [ ] `bl mcp` runs and accepts a JSON-RPC pair without crashing. Test:
  ```bash
  printf '%s\n' \
    '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}' \
    '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
    | bl mcp 2>/dev/null | head -2
  ```
  Expected: 2 JSON-RPC responses; the second has 85 tools.
- [ ] `claude mcp add browserlane -- bl mcp` then in Claude: "open example.com,
  screenshot the page, then click the More Information link". Verify the agent
  drives the browser end-to-end.

### 11. Completion + extras
- [ ] `bl completion bash | head -5` → bash completion script
- [ ] `bl completion zsh|fish|powershell` → each emits a script (exit 0)
- [ ] `bl clik` → suggests `click`
- [ ] `bl add-skill --stdout | head -10` → SKILL content (see Known content
  TODO above)

### 12. Windows-only spot checks
- [ ] Daemon uses the named pipe (`bl daemon status --json` shows
  `\\\\.\\pipe\\browserlane`)
- [ ] Pre-built Chrome.exe was downloaded by `bl install` (check the
  platform cache dir from `bl paths`)
- [ ] `bl pipe` exits 1 with `pipe mode is not supported on Windows`
  (expected — pipe is the hidden library transport)

## Sign-off

- [ ] **macOS** — all sections green; signed-off by: ______________
- [ ] **Windows** — all sections green; signed-off by: ______________
- [ ] **Known content TODO** decided (rebrand SKILL or accept)
- [ ] Ready to flip `browserlane/browserlane` visibility to **public**

> When both are ticked, ask me to run `gh repo edit browserlane/browserlane --visibility public`.

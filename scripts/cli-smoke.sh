#!/usr/bin/env bash
#
# cli-smoke.sh — the standard, self-maintaining smoke test for the browserlane
# CLI (`bl`). It drives the *built binary* through every command path and
# asserts the observable contract (stdout / exit code / artifact), labelling
# each check with the depth of verification it actually achieves:
#
#   AUTO      fully verified from stdout / exit code (deterministic).
#   PROXY     verified indirectly by reading page state back with `bl eval`
#             (confirms the mechanism fired — not the rendered pixels).
#   ARTIFACT  a file was produced and is well-formed for its type (PNG/PDF/zip
#             header, non-trivial size) — not that its *content* is correct.
#   MANUAL    cannot be auto-verified here (visual fidelity, interactive /
#             long-running transports, OS window state, or another OS). Listed,
#             not executed — see CLI-TEST-PLAN.md for the human checklist.
#
# COVERAGE GATE (the "tracking" part): after the run, the harness enumerates
# the binary's *live* visible command tree and fails if any command is neither
# tested nor explicitly registered MANUAL. New commands added under src/ext/
# therefore cannot silently escape the test plan.
#
# Usage:
#   bash scripts/cli-smoke.sh                 # headless (CI-friendly)
#   BL_VISIBLE=1 bash scripts/cli-smoke.sh    # watch it drive a real window
#   BL_BIN=/path/to/bl bash scripts/cli-smoke.sh
#
# Requires: a built `bl` (defaults to target/release/bl) and a one-time
# `bl install` (Chrome for Testing). One section navigates to example.com, so a
# network connection is needed for the cookies/storage checks. Exit 0 iff every
# AUTO/PROXY/ARTIFACT check passed AND coverage holds.
#
# Compatible with bash 3.2 (macOS system bash) — no associative arrays.
set -o pipefail

# ---- locate the binary ------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BL_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BL_BIN="${BL_BIN:-$BL_ROOT/target/release/bl}"
[ -x "$BL_BIN" ] || BL_BIN="$(command -v bl 2>/dev/null)"
if [ -z "$BL_BIN" ] || [ ! -x "$BL_BIN" ]; then
  echo "FATAL: bl binary not found. Build it: cargo build --release" >&2
  exit 2
fi
bl() { "$BL_BIN" "$@"; }

# headless unless BL_VISIBLE is set; passed to the daemon-starting command. Note:
# --headless is a per-command flag (not global), so it must come AFTER the
# subcommand — `bl daemon start --headless`, not `bl --headless daemon start`.
G="--headless"; [ -n "${BL_VISIBLE:-}" ] && G=""

# ---- colours / bookkeeping --------------------------------------------------
if [ -t 1 ]; then R=$'\033[31m'; GRN=$'\033[32m'; YEL=$'\033[33m'; DIM=$'\033[2m'; Z=$'\033[0m'; else R=; GRN=; YEL=; DIM=; Z=; fi
pass=0; fail=0; manual=0
COV=""                       # newline-delimited list of tracked command paths
FAILURES=""

add_cov() { COV="${COV}${1}"$'\n'; }
ok()   { pass=$((pass+1));   printf "  ${GRN}PASS${Z} ${DIM}%-8s${Z} %s\n" "$1" "$2"; }
no()   { fail=$((fail+1)); FAILURES="${FAILURES}    - [$1] $2 — $3"$'\n'; printf "  ${R}FAIL${Z} ${DIM}%-8s${Z} %s ${DIM}— %s${Z}\n" "$1" "$2" "$3"; }
phase(){ printf "\n${YEL}== %s ==${Z}\n" "$1"; }

# check TIER PATH "expected-substr (empty = just exit 0)" -- cmd args...
check() {
  local tier="$1" path="$2" want="$3"; shift 3; [ "$1" = "--" ] && shift
  add_cov "$path"
  local out rc; out="$("$@" 2>&1)"; rc=$?
  if [ -n "$want" ]; then
    # here-string (not a pipe): avoids pipefail false-fails when grep -q exits
    # early on large output and SIGPIPEs the upstream writer.
    if grep -qiF -- "$want" <<<"$out"; then ok "$tier" "$path"
    else no "$tier" "$path" "want '$want', got '$(printf '%s' "$out" | head -1 | cut -c1-60)'"; fi
  else
    if [ $rc -eq 0 ]; then ok "$tier" "$path"
    else no "$tier" "$path" "exit $rc: $(printf '%s' "$out" | head -1 | cut -c1-60)"; fi
  fi
}

# proxy PATH "expected" "eval-js" -- cmd args...   (run cmd, then read state back)
proxy() {
  local path="$1" want="$2" js="$3"; shift 3; [ "$1" = "--" ] && shift
  add_cov "$path"
  "$@" >/dev/null 2>&1
  local got; got="$(bl eval "$js" 2>&1)"
  if grep -qiF -- "$want" <<<"$got"; then ok PROXY "$path"
  else no PROXY "$path" "eval => '$(printf '%s' "$got" | head -1 | cut -c1-40)' want '$want'"; fi
}

# artifact PATH file kind -- cmd args...   (kind: pdf|zip|json|any). For
# screenshots use shot() — `bl screenshot` ignores -o's dir and reports its path.
artifact() {
  local path="$1" file="$2" kind="$3"; shift 3; [ "$1" = "--" ] && shift
  add_cov "$path"
  rm -f "$file"; "$@" >/dev/null 2>&1
  if [ ! -s "$file" ]; then no ARTIFACT "$path" "no file at $file"; return; fi
  case "$kind" in
    pdf)  head -c4 "$file" | grep -q 'PDF' && ok ARTIFACT "$path" || no ARTIFACT "$path" "not a PDF" ;;
    zip)  head -c2 "$file" | grep -q 'PK'  && ok ARTIFACT "$path" || no ARTIFACT "$path" "not a zip" ;;
    json) head -c1 "$file" | grep -q '[{[]' && ok ARTIFACT "$path" || no ARTIFACT "$path" "not JSON" ;;
    *)    ok ARTIFACT "$path" ;;
  esac
}

# shot PATH -- cmd...   (screenshot: parse the "saved to <path>" it prints)
shot() {
  local path="$1"; shift; [ "$1" = "--" ] && shift
  add_cov "$path"
  local out p; out="$("$@" 2>&1)"
  p="$(printf '%s' "$out" | sed -n 's/.*saved to //p' | tr -d '\r')"
  if [ -n "$p" ] && [ -s "$p" ] && file "$p" | grep -qi png; then ok ARTIFACT "$path"
  else no ARTIFACT "$path" "no PNG (out: $(printf '%s' "$out" | head -1 | cut -c1-50))"; fi
}

# register a path as MANUAL (tracked, but verified by a human elsewhere).
manual() { add_cov "$1"; manual=$((manual+1)); printf "  ${YEL}MANUAL${Z}   %-20s ${DIM}%s${Z}\n" "$1" "$2"; }

# ---- fixture ----------------------------------------------------------------
FIX="${TMPDIR:-/tmp}/bl-fixture.html"; F="file://$FIX"
cat > "$FIX" <<'HTML'
<!doctype html><meta charset=utf-8><title>BL Fixture</title>
<h1 id=h ondblclick="this.dataset.dc='1'">BL Fixture</h1>
<a id=lnk href="https://example.com/next" onmouseover="this.dataset.hov='1'">More information</a>
<input type=text name=q placeholder="search" aria-label="Search">
<input type=checkbox id=cb> <input type=radio name=r id=rb>
<select id=sel><option>one<option>two</select>
<button id=btn onclick="this.dataset.k='1'">Go</button>
<input type=file id=file>
<img src="data:image/gif;base64,R0lGODlhAQABAAAAACw=" alt="logo">
<div data-testid=tid role=note title="tt">tagged</div>
<iframe srcdoc="<p id=inner>inside</p>" name=f1></iframe>
<div style="height:3000px">tall</div>
HTML
OUT="${TMPDIR:-/tmp}/bl-smoke"; mkdir -p "$OUT"
goto() { bl go "$F" >/dev/null 2>&1; }

echo "browserlane CLI smoke — binary: $BL_BIN  mode: $([ -n "$G" ] && echo headless || echo visible)"

# ===========================================================================
phase "1 · meta (no browser)"
check AUTO version  "v"                  -- bl version
check AUTO version  '{"version"'         -- bl version --json       # --json variant
check AUTO paths    ""                   -- bl paths
check AUTO paths    '"cache_dir"'        -- bl paths --json         # --json variant
check AUTO is-installed ""               -- bl is-installed
check AUTO is-installed '"installed"'    -- bl is-installed --json  # --json variant
# Help is clap-native now; exact help bytes are pinned by the `cargo test` insta
# snapshots (the help safety net), so here we only assert `--help` runs (exit 0)
# and prints the program — not its precise layout.
check AUTO help     "bl"                 -- bl --help
check AUTO "completion bash"       "complete"                   -- bl completion bash
check AUTO "completion zsh"        "compdef"                    -- bl completion zsh
check AUTO "completion fish"       "complete"                   -- bl completion fish
check AUTO "completion powershell" "Register-ArgumentCompleter" -- bl completion powershell
check AUTO add-skill ""                  -- bl add-skill --stdout
check AUTO add-mcp   "claude"            -- bl add-mcp --list   # ext: lists MCP clients
# Error handling is clap-native now: a near-miss suggests the closest command and
# an unknown one is reported as "unrecognized subcommand"; both exit 2 (clap's
# usage-error code) where cobra used 1. Assert the message AND the exit code.
err_exit2() {  # err_exit2 PATH want-substr -- cmd...   (assert substr in output + exit 2)
  local path="$1" want="$2"; shift 2; [ "$1" = "--" ] && shift
  add_cov "$path"
  local out rc; out="$("$@" 2>&1)"; rc=$?
  if grep -qiF -- "$want" <<<"$out" && [ "$rc" -eq 2 ]; then ok AUTO "$path"
  else no AUTO "$path" "want '$want' & exit 2, got exit $rc: '$(printf '%s' "$out" | head -1 | cut -c1-50)'"; fi
}
err_exit2 _err:suggest "similar subcommand" -- bl clik       # clap suggests 'click'
err_exit2 _err:unknown "unrecognized subcommand" -- bl zzzznope  # clap unknown cmd
manual install "run once; state verified by is-installed"

# ===========================================================================
phase "2 · daemon (headless start → reused as the session)"
bl daemon stop >/dev/null 2>&1; sleep 1   # let any prior daemon fully exit
check AUTO "daemon start"  "" -- bl daemon start $G
# `daemon start` can return before its socket is serving — wait for readiness
# before asserting status (otherwise the first status races a half-open daemon).
for i in $(seq 1 20); do dstat="$(bl daemon status 2>&1)"; grep -qi running <<<"$dstat" && break; sleep 0.3; done
check AUTO "daemon status" "running" -- bl daemon status
check AUTO "daemon status" "0.1.2"   -- bl daemon status --json   # --json variant

# ===========================================================================
phase "3 · session + read-only inspection"
goto
check AUTO go     "bl-fixture" -- bl url
check AUTO url    "bl-fixture" -- bl url
check AUTO title  "BL Fixture" -- bl title
check AUTO text   "BL Fixture" -- bl text "#h"
check AUTO html   "BL Fixture" -- bl html "#h"
check AUTO html   "<h1"        -- bl html "#h" --outer       # --outer variant
check AUTO attr   "example.com/next" -- bl attr "#lnk" href
check AUTO value  "one"        -- bl value "#sel"
check AUTO count  ""           -- bl count "input"
check AUTO find   ""           -- bl find "a"
check AUTO find   ""           -- bl find "input" --all --limit 2   # --all/--limit
check AUTO "find role"        "" -- bl find role button --name Go
check AUTO "find text"        "" -- bl find text "More information"
check AUTO "find label"       "" -- bl find label "Search"
check AUTO "find placeholder" "" -- bl find placeholder search
check AUTO "find alt"         "" -- bl find alt logo
check AUTO "find title"       "" -- bl find title tt
check AUTO "find testid"      "" -- bl find testid tid
check AUTO "find xpath"       "" -- bl find xpath "//h1"
check AUTO "is visible"    "true"  -- bl is visible "#h"
check AUTO "is enabled"    "true"  -- bl is enabled "#btn"
check AUTO "is checked"    "false" -- bl is checked "#cb"
check AUTO "is actionable" "ctionab" -- bl is actionable "$F" "#btn"   # needs url + selector
check AUTO a11y-tree "" -- bl a11y-tree
check AUTO a11y-tree "" -- bl a11y-tree --everything
check AUTO map "" -- bl map
check AUTO map "" -- bl map --selector body
check AUTO pages "" -- bl pages
check AUTO frames "" -- bl frames
check AUTO frame "" -- bl frame f1

# ===========================================================================
phase "4 · wait"
check AUTO wait        "" -- bl wait "#h" --state visible --timeout 5000
check AUTO "wait load" "" -- bl wait load --timeout 5000
check AUTO "wait text" "" -- bl wait text "Fixture" --timeout 5000
check AUTO "wait url"  "" -- bl wait url "*bl-fixture*" --timeout 5000
check AUTO "wait fn"   "" -- bl wait fn "document.title.length>0" --timeout 5000

# ===========================================================================
phase "5 · interaction (PROXY = read state back)"
goto
proxy click   "true"  "document.getElementById('cb').checked"            -- bl click "#cb"
proxy uncheck "false" "document.getElementById('cb').checked"            -- bl uncheck "#cb"
proxy check   "true"  "document.getElementById('rb').checked"            -- bl check "#rb"
proxy type    "hello" "document.querySelector('input[name=q]').value"    -- bl type 'input[name=q]' hello
proxy fill    "world" "document.querySelector('input[name=q]').value"    -- bl fill 'input[name=q]' world
proxy select  "two"   "document.getElementById('sel').value"             -- bl select "#sel" two
proxy focus   "true"  "document.activeElement===document.querySelector('input[name=q]')" -- bl focus 'input[name=q]'
proxy hover   "1"     "document.getElementById('lnk').dataset.hov||''"   -- bl hover "#lnk"
proxy dblclick "1"    "document.getElementById('h').dataset.dc||''"      -- bl dblclick "#h"
# scroll: the first scroll after a load can be a no-op, so warm up then assert.
add_cov "scroll"; bl scroll down >/dev/null 2>&1; bl scroll down --amount 15 >/dev/null 2>&1
sy="$(bl eval 'window.scrollY' 2>/dev/null)"; sy="${sy%%.*}"
{ [ -n "$sy" ] && [ "$sy" -gt 0 ] 2>/dev/null; } && ok PROXY scroll || no PROXY scroll "scrollY=$sy (want >0)"
proxy "scroll into-view" "true" "(function(){var r=document.getElementById('btn').getBoundingClientRect();return r.top<innerHeight})()" -- bl scroll into-view "#btn"
check AUTO press "" -- bl press a "#h"
check AUTO keys  "" -- bl keys "Control+a"
check AUTO "mouse move"  "" -- bl mouse move 40 40
check AUTO "mouse down"  "" -- bl mouse down --button 0
check AUTO "mouse up"    "" -- bl mouse up --button 0
check AUTO "mouse click" "" -- bl mouse click 40 40
check AUTO highlight "" -- bl highlight "#h"   # runs; red-outline visual is MANUAL

# ===========================================================================
phase "6 · emulation"
proxy viewport "1280" "innerWidth" -- bl viewport 1280 720
check AUTO viewport "" -- bl viewport 1024 768 --dpr 2   # --dpr variant
check AUTO window "" -- bl window 1024 768
check AUTO window "" -- bl window --state normal         # --state variant
proxy media "true" "matchMedia('(prefers-color-scheme: dark)').matches" -- bl media --color-scheme dark
check AUTO media "" -- bl media --reduced-motion reduce --forced-colors active --contrast more --media screen
# negative longitude must follow `--` (clap parses -74 as a flag otherwise).
check AUTO geolocation "" -- bl geolocation --accuracy 5 -- 40.7128 -74.0060

# ===========================================================================
phase "7 · capture (ARTIFACT = produced + well-formed)"
goto
shot screenshot -- bl screenshot -o s.png
shot screenshot -- bl screenshot -o full.png --full-page
shot screenshot -- bl screenshot -o ann.png --annotate
artifact pdf "$OUT/p.pdf" pdf -- bl pdf -o "$OUT/p.pdf"

# ===========================================================================
phase "8 · storage / state (needs an http origin → example.com)"
bl go https://example.com >/dev/null 2>&1
check AUTO cookies "" -- bl cookies session abc123
check AUTO cookies "" -- bl cookies
check AUTO "cookies clear" "" -- bl cookies clear
artifact storage "$OUT/state.json" json -- bl storage -o "$OUT/state.json"
check AUTO "storage restore" "" -- bl storage restore "$OUT/state.json"
# `bl diff` is a subcommand-required parent now (bare → clap usage error, exit 2);
# its behaviour is exercised via `diff map`, which also covers the `diff` path.
check AUTO "diff map" "" -- bl diff map

# ===========================================================================
phase "9 · stateful (upload / download / record / eval)"
goto
proxy upload "1" "document.getElementById('file').files.length" -- bl upload "#file" "$FIX"
# `bl download` is a subcommand-required parent now (bare → clap usage error,
# exit 2); `download dir` exercises it and covers the `download` path.
check AUTO "download dir" "" -- bl download dir "$OUT"
# chunks and groups nest INSIDE an active recording; record stop closes it.
check AUTO "record start" "" -- bl record start --name r1 --format png --screenshots
check AUTO "record chunk start" "" -- bl record chunk start --name c1
bl click "#cb" >/dev/null 2>&1
artifact "record chunk stop" "$OUT/chunk.zip" zip -- bl record chunk stop -o "$OUT/chunk.zip"
check AUTO "record group start" "" -- bl record group start grp1
bl click "#cb" >/dev/null 2>&1
check AUTO "record group stop" "" -- bl record group stop
artifact "record stop" "$OUT/rec.zip" zip -- bl record stop -o "$OUT/rec.zip"
check AUTO eval "2" -- bl eval "1+1"
add_cov "eval"; estdin="$(printf '3+4' | bl eval --stdin 2>&1)"
grep -q 7 <<<"$estdin" && ok AUTO "eval (stdin)" || no AUTO "eval (stdin)" "got '$estdin'"
check AUTO sleep "" -- bl sleep 100

# ===========================================================================
phase "10 · content (destructive) · multipage · navigation"
goto
proxy content "NEW" "document.querySelector('h1').textContent" -- bl content "<h1 id=h>NEW</h1>"
check AUTO "page new" "" -- bl page new "data:text/html,<title>T2</title>"
check AUTO pages "" -- bl pages
check AUTO "page switch" "" -- bl page switch 0
check AUTO "page close" "" -- bl page close 1
bl go "data:text/html,<title>AAA</title>" >/dev/null 2>&1
bl go "data:text/html,<title>BBB</title>" >/dev/null 2>&1
proxy back    "AAA" "document.title" -- bl back
proxy forward "BBB" "document.title" -- bl forward
check AUTO reload "" -- bl reload

# ===========================================================================
phase "11 · session lifecycle + teardown"
check AUTO start "" -- bl start "data:text/html,<title>S</title>"
check AUTO stop  "" -- bl stop
check AUTO "daemon stop" "" -- bl daemon stop
sleep 1   # let the socket/pidfile clear before testing auto-start
add_cov "_auto-restart"
if bl go "data:text/html,<title>R</title>" >/dev/null 2>&1; then ok AUTO "_auto-restart (daemon re-spawns)"; else no AUTO "_auto-restart" "auto-start failed"; fi
bl daemon stop >/dev/null 2>&1

# ===========================================================================
phase "MANUAL — cannot be auto-verified here (see CLI-TEST-PLAN.md)"
manual launch-test     "launches browser + prints ws:// URL; leaves processes"
manual bidi-test       "low-level BiDi diagnostic"
manual ws-test         "interactive WebSocket echo loop"
manual drag            "needs a drop-target fixture; gesture is visual"
manual "dialog accept" "alert() blocks the page; dialog handling is interactive"
manual "dialog dismiss" "interactive dialog handling"
manual pipe            "hidden library transport (stdin BiDi); errors on Windows"
manual serve           "hidden long-running WebDriver-ish server"

# mcp is a server but IS scriptable — verify the JSON-RPC handshake.
phase "MCP handshake (scripted)"
add_cov "mcp"
MCP_OUT="$(printf '%s\n' \
 '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}' \
 '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | bl mcp 2>/dev/null | head -5)"
NTOOLS="$(printf '%s' "$MCP_OUT" | grep -o '"name"' | wc -l | tr -d ' ')"
if printf '%s' "$MCP_OUT" | grep -q '"tools"'; then ok AUTO "mcp (tools/list: ~$NTOOLS tools)"; else no AUTO mcp "no tools/list response"; fi

# ===========================================================================
# COVERAGE GATE — enumerate the binary's live visible command tree; every path
# must be tested or registered MANUAL. This is what tracks future commands.
phase "coverage gate"
is_covered() {  # covered if exactly tested, or a parent of a tested leaf
  local p="$1" c
  while IFS= read -r c; do
    [ -z "$c" ] && continue
    [ "$c" = "$p" ] && return 0
    case "$c" in "$p "*) return 0;; esac
  done <<EOF
$COV
EOF
  return 1
}
# Enumerate a command's subcommands from its --help. The root and clap-native
# subcommand help share the same headings: a "Commands:" block closed by the next
# section ("Arguments:" or "Options:"). The root additionally interleaves category
# headers (Navigation:, Interaction: … at column 0) and blank lines between the
# command rows; those are tolerated because a "  <lowercase>" command row is the
# only thing we print. Start the list at "Commands:", end it at a known closer.
subcmds() {
  bl $1 --help 2>&1 | awk '
    /^Commands:/                 { f=1; next }
    f && /^(Options|Arguments):/ { f=0 }   # block closers
    f && /^  [a-z]/              { print $1 }
  '
}
Q=(""); ALLPATHS=()
while [ ${#Q[@]} -gt 0 ]; do
  cur="${Q[0]}"; Q=("${Q[@]:1}")
  [ -n "$cur" ] && ALLPATHS+=("$cur")
  while IFS= read -r s; do
    [ -z "$s" ] && continue; [ "$s" = "help" ] && continue
    if [ -z "$cur" ]; then Q+=("$s"); else Q+=("$cur $s"); fi
  done < <(subcmds "$cur")
done
untracked=0
for p in "${ALLPATHS[@]}"; do
  [ "$p" = "help" ] && continue
  if ! is_covered "$p"; then printf "  ${R}UNTRACKED${Z} %s — add a test or register MANUAL\n" "$p"; untracked=$((untracked+1)); fi
done
[ $untracked -eq 0 ] && printf "  ${GRN}OK${Z} all %d live command paths are tracked\n" "${#ALLPATHS[@]}"

# ---- summary ----------------------------------------------------------------
ntracked="$(printf '%s' "$COV" | grep -c .)"
phase "summary"
printf "  tracked paths : %s   live paths: %d\n" "$ntracked" "${#ALLPATHS[@]}"
printf "  ${GRN}passed${Z} %d   ${R}failed${Z} %d   ${YEL}manual${Z} %d\n" "$pass" "$fail" "$manual"
[ -n "$FAILURES" ] && { echo "  failures:"; printf '%s' "$FAILURES"; }
[ $fail -eq 0 ] && [ $untracked -eq 0 ] && { echo "${GRN}SMOKE OK${Z}"; exit 0; }
echo "${R}SMOKE FAILED${Z} ($fail check(s), $untracked untracked)"; exit 1

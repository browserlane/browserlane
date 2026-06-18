//! Cobra-formatted `--help` text for every command. Each prog-path token
//! (Usage / footer / root's `help for` & `version for`) is the
//! `__BROWSERLANE_PROG_NAME_PLACEHOLDER__` sentinel, substituted with the
//! program name at runtime (see help.rs). Hand-maintained.

/// Placeholder that stands in for the program name (base of argv0).
pub const PROG_SENTINEL: &str = "__BROWSERLANE_PROG_NAME_PLACEHOLDER__";

/// Every valid command path (space-joined; "" is the root).
pub const COMMAND_PATHS: &[&str] = &["", "a11y-tree", "add-skill", "attr", "back", "bidi-test", "check", "click", "completion", "completion bash", "completion fish", "completion powershell", "completion zsh", "content", "cookies", "cookies clear", "count", "daemon", "daemon start", "daemon status", "daemon stop", "dblclick", "dialog", "dialog accept", "dialog dismiss", "diff", "diff map", "download", "download dir", "drag", "eval", "fill", "find", "find alt", "find label", "find placeholder", "find role", "find testid", "find text", "find title", "find xpath", "focus", "forward", "frame", "frames", "geolocation", "go", "help", "highlight", "hover", "html", "install", "is", "is actionable", "is checked", "is enabled", "is visible", "is-installed", "keys", "launch-test", "map", "mcp", "media", "mouse", "mouse click", "mouse down", "mouse move", "mouse up", "page", "page close", "page new", "page switch", "pages", "paths", "pdf", "pipe", "press", "record", "record chunk", "record chunk start", "record chunk stop", "record group", "record group start", "record group stop", "record start", "record stop", "reload", "screenshot", "scroll", "scroll into-view", "select", "serve", "sleep", "start", "stop", "storage", "storage restore", "text", "title", "type", "uncheck", "upload", "url", "value", "version", "viewport", "wait", "wait fn", "wait load", "wait text", "wait url", "window", "ws-test"];

/// Returns the cobra `--help` text for a command path ("" = root), or None.
pub fn help_text(path: &str) -> Option<&'static str> {
    let t = match path {
        "" => r#"Browser automation for AI agents and humans

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ [flags]
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ [command]

Available Commands:
  a11y-tree    Get the accessibility tree of the current page
  add-skill    Install browserlane browser skill for Claude Code
  attr         Get an HTML attribute value from an element
  back         Navigate back in browser history
  bidi-test    Launch browser, connect via BiDi, send session.status
  check        Check a checkbox or radio button
  click        Click an element (optionally navigate to URL first)
  completion   Generate the autocompletion script for the specified shell
  content      Replace the page HTML content
  cookies      Manage browser cookies
  count        Count matching elements
  daemon       Manage the browserlane daemon (background browser process)
  dblclick     Double-click an element
  dialog       Handle browser dialogs (alert, confirm, prompt)
  diff         Compare current state vs previous
  download     Manage browser downloads
  drag         Drag from one element to another
  eval         Evaluate a JavaScript expression (optionally navigate to URL first)
  fill         Clear an input field and type new text
  find         Find elements by CSS selector or semantic locator
  focus        Focus an element
  forward      Navigate forward in browser history
  frame        Find a frame by name or URL substring
  frames       List all child frames (iframes) on the page
  geolocation  Override the browser geolocation
  go           Go to a URL and print page info
  help         Help about any command
  highlight    Highlight an element with a red outline for 3 seconds
  hover        Hover over an element by CSS selector
  html         Get HTML content of the page or an element
  install      Download Chrome for Testing and chromedriver
  is           Check element state (visible, enabled, checked, actionable)
  is-installed Check if Chrome and chromedriver are installed (exit 0 = yes, exit 1 = no)
  keys         Press a key or key combination
  launch-test  Launch browser via chromedriver and print BiDi WebSocket URL
  map          Map interactive page elements with @refs
  mcp          Start MCP server (stdio JSON-RPC for LLM agents)
  media        Override CSS media features
  mouse        Mouse control (click, move, down, up)
  page         Manage browser pages (new, close, switch)
  pages        List all open browser pages
  paths        Print browser and cache paths
  pdf          Save page as PDF
  press        Press a key on a specific element or the focused element
  record       Record browser sessions (screenshots and snapshots)
  reload       Reload the current page
  screenshot   Capture a screenshot (optionally navigate to URL first)
  scroll       Scroll the page or an element
  select       Select an option in a <select> element
  sleep        Pause execution for a number of milliseconds
  start        Start a browser session
  stop         Stop the browser session
  storage      Export or restore browser state (cookies, localStorage, sessionStorage)
  text         Get text content of the page or an element
  title        Get the current page title
  type         Type text into an element (optionally navigate to URL first)
  uncheck      Uncheck a checkbox
  upload       Set files on an input[type=file] element
  url          Get the current page URL
  value        Get the current value of a form element
  version      Print the version number
  viewport     Get or set the browser viewport size
  wait         Wait for an element, URL, text, page load, or JS condition
  window       Get or set the OS browser window size, position, or state
  ws-test      Test WebSocket connection (type messages, see echoes)

Flags:
      --headless   Hide browser window (visible by default)
  -h, --help       help for __BROWSERLANE_PROG_NAME_PLACEHOLDER__
      --json       Output as JSON
  -v, --verbose    Enable debug logging
      --version    version for __BROWSERLANE_PROG_NAME_PLACEHOLDER__

Use "__BROWSERLANE_PROG_NAME_PLACEHOLDER__ [command] --help" for more information about a command.
"#,
        "a11y-tree" => r#"Get the accessibility tree of the current page

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ a11y-tree [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ a11y-tree
  # Print the accessibility tree (interesting nodes only)

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ a11y-tree --everything
  # Include all nodes (generic containers, etc.)

Flags:
      --everything   Show all nodes including generic containers
  -h, --help         help for a11y-tree

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "add-skill" => r#"Install browserlane browser skill for Claude Code

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ add-skill [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ add-skill
  # Installs skill to ~/.claude/skills/vibe-check/

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ add-skill --stdout
  # Print skill content to stdout

Flags:
  -h, --help     help for add-skill
      --stdout   Print skill content to stdout instead of installing

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "attr" => r#"Get an HTML attribute value from an element

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ attr [selector] [attribute] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ attr "a" "href"
  # Get the href of the first link

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ attr "img" "src"
  # Get the image source URL

Flags:
  -h, --help   help for attr

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "back" => r#"Navigate back in browser history

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ back [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ back
  # Go back one page (like clicking the back button)

Flags:
  -h, --help   help for back

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "bidi-test" => r#"Launch browser, connect via BiDi, send session.status

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ bidi-test [flags]

Flags:
  -h, --help   help for bidi-test

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "check" => r#"Check a checkbox or radio button

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ check [selector] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ check "input[name=agree]"
  # Check the "agree" checkbox (idempotent)

Flags:
  -h, --help   help for check

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "click" => r#"Click an element (optionally navigate to URL first)

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ click [url] [selector] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ click "a"
  # Clicks on current page

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ click https://example.com "a"
  # Navigates to URL first, then clicks

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ click https://example.com "a" --timeout 5s
  # Custom timeout for actionability checks

Flags:
  -h, --help               help for click
      --timeout duration   Timeout for actionability checks (e.g., 5s, 30s) (default 30s)

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "completion" => r#"Generate the autocompletion script for __BROWSERLANE_PROG_NAME_PLACEHOLDER__ for the specified shell.
See each sub-command's help for details on how to use the generated script.

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ completion [command]

Available Commands:
  bash        Generate the autocompletion script for bash
  fish        Generate the autocompletion script for fish
  powershell  Generate the autocompletion script for powershell
  zsh         Generate the autocompletion script for zsh

Flags:
  -h, --help   help for completion

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging

Use "__BROWSERLANE_PROG_NAME_PLACEHOLDER__ completion [command] --help" for more information about a command.
"#,
        "completion bash" => r#"Generate the autocompletion script for the bash shell.

This script depends on the 'bash-completion' package.
If it is not installed already, you can install it via your OS's package manager.

To load completions in your current shell session:

	source <(__BROWSERLANE_PROG_NAME_PLACEHOLDER__ completion bash)

To load completions for every new session, execute once:

#### Linux:

	__BROWSERLANE_PROG_NAME_PLACEHOLDER__ completion bash > /etc/bash_completion.d/__BROWSERLANE_PROG_NAME_PLACEHOLDER__

#### macOS:

	__BROWSERLANE_PROG_NAME_PLACEHOLDER__ completion bash > $(brew --prefix)/etc/bash_completion.d/__BROWSERLANE_PROG_NAME_PLACEHOLDER__

You will need to start a new shell for this setup to take effect.

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ completion bash

Flags:
  -h, --help              help for bash
      --no-descriptions   disable completion descriptions

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "completion fish" => r#"Generate the autocompletion script for the fish shell.

To load completions in your current shell session:

	__BROWSERLANE_PROG_NAME_PLACEHOLDER__ completion fish | source

To load completions for every new session, execute once:

	__BROWSERLANE_PROG_NAME_PLACEHOLDER__ completion fish > ~/.config/fish/completions/__BROWSERLANE_PROG_NAME_PLACEHOLDER__.fish

You will need to start a new shell for this setup to take effect.

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ completion fish [flags]

Flags:
  -h, --help              help for fish
      --no-descriptions   disable completion descriptions

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "completion powershell" => r#"Generate the autocompletion script for powershell.

To load completions in your current shell session:

	__BROWSERLANE_PROG_NAME_PLACEHOLDER__ completion powershell | Out-String | Invoke-Expression

To load completions for every new session, add the output of the above command
to your powershell profile.

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ completion powershell [flags]

Flags:
  -h, --help              help for powershell
      --no-descriptions   disable completion descriptions

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "completion zsh" => r#"Generate the autocompletion script for the zsh shell.

If shell completion is not already enabled in your environment you will need
to enable it.  You can execute the following once:

	echo "autoload -U compinit; compinit" >> ~/.zshrc

To load completions in your current shell session:

	source <(__BROWSERLANE_PROG_NAME_PLACEHOLDER__ completion zsh)

To load completions for every new session, execute once:

#### Linux:

	__BROWSERLANE_PROG_NAME_PLACEHOLDER__ completion zsh > "${fpath[1]}/___BROWSERLANE_PROG_NAME_PLACEHOLDER__"

#### macOS:

	__BROWSERLANE_PROG_NAME_PLACEHOLDER__ completion zsh > $(brew --prefix)/share/zsh/site-functions/___BROWSERLANE_PROG_NAME_PLACEHOLDER__

You will need to start a new shell for this setup to take effect.

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ completion zsh [flags]

Flags:
  -h, --help              help for zsh
      --no-descriptions   disable completion descriptions

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "content" => r#"Replace the page HTML content

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ content [html] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ content "<h1>Hello World</h1>"
  # Set page content directly

  echo "<h1>Hello</h1>" | __BROWSERLANE_PROG_NAME_PLACEHOLDER__ content --stdin
  # Set page content from stdin

Flags:
  -h, --help    help for content
      --stdin   Read HTML from stdin

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "cookies" => r#"Manage browser cookies

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ cookies [name] [value] [flags]
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ cookies [command]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ cookies
  # List all cookies

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ cookies "session" "abc123"
  # Set a cookie with name and value

Available Commands:
  clear       Clear all cookies

Flags:
  -h, --help   help for cookies

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging

Use "__BROWSERLANE_PROG_NAME_PLACEHOLDER__ cookies [command] --help" for more information about a command.
"#,
        "cookies clear" => r#"Clear all cookies

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ cookies clear [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ cookies clear
  # Delete all cookies

Flags:
  -h, --help   help for clear

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "count" => r#"Count matching elements

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ count [selector] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ count "a"
  # Print number of links on the page

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ count "li.item"
  # Count list items

Flags:
  -h, --help   help for count

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "daemon" => r#"Manage the browserlane daemon (background browser process)

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ daemon [flags]
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ daemon [command]

Available Commands:
  start       Start the browserlane daemon
  status      Show daemon status
  stop        Stop the browserlane daemon

Flags:
  -h, --help   help for daemon

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging

Use "__BROWSERLANE_PROG_NAME_PLACEHOLDER__ daemon [command] --help" for more information about a command.
"#,
        "daemon start" => r#"Start the browserlane daemon

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ daemon start [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ daemon start
  # Starts daemon in background

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ daemon start --foreground
  # Starts daemon in foreground (for debugging)

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ daemon start --idle-timeout 30m
  # Auto-shutdown after 30 minutes of inactivity

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ daemon start --connect ws://remote:9515/session
  # Connect to a remote browser instead of launching a local one

Flags:
      --connect string               Connect to a remote BiDi WebSocket URL instead of launching a local browser
      --connect-header stringArray   HTTP header for WebSocket connect (repeatable, format: "Key: Value")
      --foreground                   Run daemon in foreground (for debugging)
  -h, --help                         help for start
      --idle-timeout duration        Shutdown after this duration of inactivity (0 to disable) (default 30m0s)

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "daemon status" => r#"Show daemon status

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ daemon status [flags]

Flags:
  -h, --help   help for status

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "daemon stop" => r#"Stop the browserlane daemon

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ daemon stop [flags]

Flags:
  -h, --help   help for stop

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "dblclick" => r#"Double-click an element

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ dblclick [selector] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ dblclick "td.cell"
  # Double-click to edit a table cell

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ dblclick @e2
  # Double-click element from map

Flags:
  -h, --help   help for dblclick

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "dialog" => r#"Handle browser dialogs (alert, confirm, prompt)

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ dialog [flags]
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ dialog [command]

Available Commands:
  accept      Accept a dialog (optionally with prompt text)
  dismiss     Dismiss a dialog

Flags:
  -h, --help   help for dialog

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging

Use "__BROWSERLANE_PROG_NAME_PLACEHOLDER__ dialog [command] --help" for more information about a command.
"#,
        "dialog accept" => r#"Accept a dialog (optionally with prompt text)

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ dialog accept [text] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ dialog accept
  # Accept an alert or confirm dialog

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ dialog accept "my input"
  # Accept a prompt dialog with text

Flags:
  -h, --help   help for accept

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "dialog dismiss" => r#"Dismiss a dialog

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ dialog dismiss [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ dialog dismiss
  # Dismiss/cancel a dialog

Flags:
  -h, --help   help for dismiss

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "diff" => r#"Compare current state vs previous

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ diff [flags]
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ diff [command]

Available Commands:
  map         Compare current page elements vs last map

Flags:
  -h, --help   help for diff

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging

Use "__BROWSERLANE_PROG_NAME_PLACEHOLDER__ diff [command] --help" for more information about a command.
"#,
        "diff map" => r#"Compare current page elements vs last map

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ diff map [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ map           # take initial snapshot
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ click @e3     # interact with page
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ diff map      # see what changed

Flags:
  -h, --help   help for map

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "download" => r#"Manage browser downloads

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ download [flags]
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ download [command]

Available Commands:
  dir         Set the download directory

Flags:
  -h, --help   help for download

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging

Use "__BROWSERLANE_PROG_NAME_PLACEHOLDER__ download [command] --help" for more information about a command.
"#,
        "download dir" => r#"Set the download directory

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ download dir [path] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ download dir ./downloads
  # Set download directory to ./downloads

Flags:
  -h, --help   help for dir

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "drag" => r#"Drag from one element to another

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ drag [source] [target] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ drag ".draggable" ".dropzone"
  # Drag element to drop target

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ drag @e1 @e3
  # Drag using map refs

Flags:
  -h, --help   help for drag

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "eval" => r#"Evaluate a JavaScript expression (optionally navigate to URL first)

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ eval [url] [expression] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ eval "document.title"
  # Evaluates on current page

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ eval https://example.com "document.title"
  # Navigates to URL first, then evaluates

  echo 'document.title' | __BROWSERLANE_PROG_NAME_PLACEHOLDER__ eval --stdin
  # Read expression from stdin (avoids shell quoting issues)

Flags:
  -h, --help    help for eval
      --stdin   Read expression from stdin

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "fill" => r##"Clear an input field and type new text

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ fill [selector] [text] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ fill "input[name=email]" "user@example.com"
  # Clear the field and type new value

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ fill "#search" "browserlane"
  # Replace search field contents

Flags:
  -h, --help   help for fill

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"##,
        "find" => r#"Find elements by CSS selector or semantic locator

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find [selector] [flags]
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find [command]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find "a"
  # → @e1 [a] "More information..."

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find "a" --all
  # → @e1 [a] "Home"  @e2 [a] "About"  ...

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find text "Sign In"
  # → @e1 [button] "Sign In"

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find role button
  # → @e1 [button] "Submit"

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find role heading --name "Example"
  # Find heading with accessible name "Example"

Available Commands:
  alt         Find element by alt attribute
  label       Find input by associated label text
  placeholder Find element by placeholder attribute
  role        Find element by ARIA role
  testid      Find element by data-testid attribute
  text        Find element by text content
  title       Find element by title attribute
  xpath       Find element by XPath expression

Flags:
      --all         Find all matching elements
  -h, --help        help for find
      --limit int   Maximum number of elements to return (with --all) (default 10)

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging

Use "__BROWSERLANE_PROG_NAME_PLACEHOLDER__ find [command] --help" for more information about a command.
"#,
        "find alt" => r#"Find element by alt attribute

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find alt [alt] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find alt "Logo"

Flags:
  -h, --help   help for alt

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "find label" => r#"Find input by associated label text

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find label [label] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find label "Email"
  # → @e1 [input type="email"] placeholder="Email"

Flags:
  -h, --help   help for label

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "find placeholder" => r#"Find element by placeholder attribute

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find placeholder [placeholder] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find placeholder "Search..."
  # → @e1 [input] placeholder="Search..."

Flags:
  -h, --help   help for placeholder

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "find role" => r#"Find element by ARIA role

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find role [role] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find role button
  # → @e1 [button] "Submit"

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find role heading --name "Example"
  # Find heading with accessible name "Example"

Flags:
  -h, --help          help for role
      --name string   Accessible name filter

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "find testid" => r#"Find element by data-testid attribute

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find testid [testid] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find testid "submit-btn"
  # → @e1 [button] data-testid="submit-btn"

Flags:
  -h, --help   help for testid

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "find text" => r#"Find element by text content

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find text [text] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find text "Sign In"
  # → @e1 [button] "Sign In"

Flags:
  -h, --help   help for text

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "find title" => r#"Find element by title attribute

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find title [title] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find title "Close"

Flags:
  -h, --help   help for title

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "find xpath" => r#"Find element by XPath expression

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find xpath [expression] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ find xpath "//div[@class='main']"
  # → @e1 [div.main] ...

Flags:
  -h, --help   help for xpath

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "focus" => r#"Focus an element

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ focus [selector] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ focus "input[name=email]"
  # Focus the email input

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ focus @e1
  # Focus element from map

Flags:
  -h, --help   help for focus

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "forward" => r#"Navigate forward in browser history

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ forward [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ forward
  # Go forward one page (like clicking the forward button)

Flags:
  -h, --help   help for forward

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "frame" => r#"Find a frame by name or URL substring

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ frame [nameOrUrl] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ frame "myIframe"
  # Find frame by name

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ frame "example.com"
  # Find frame by URL substring

Flags:
  -h, --help   help for frame

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "frames" => r#"List all child frames (iframes) on the page

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ frames [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ frames
  # [{"context":"abc","url":"https://example.com/frame","name":"myFrame"}]

Flags:
  -h, --help   help for frames

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "geolocation" => r#"Override the browser geolocation

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ geolocation [latitude] [longitude] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ geolocation 40.7128 -74.006
  # Set location to New York City

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ geolocation 51.5074 -0.1278 --accuracy 10
  # Set location to London with 10m accuracy

Flags:
      --accuracy float   Accuracy in meters (default: 1)
  -h, --help             help for geolocation

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "go" => r#"Go to a URL and print page info

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ go [url] [flags]

Flags:
  -h, --help   help for go

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "help" => r#"Help provides help for any command in the application.
Simply type __BROWSERLANE_PROG_NAME_PLACEHOLDER__ help [path to command] for full details.

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ help [command] [flags]

Flags:
  -h, --help   help for help

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "highlight" => r#"Highlight an element with a red outline for 3 seconds

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ highlight [selector] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ highlight "h1"
  # Highlights the first h1 element

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ highlight @e1
  # Highlights the element from map

Flags:
  -h, --help   help for highlight

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "hover" => r#"Hover over an element by CSS selector

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ hover [selector] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ hover "a"
  # Hover over first link

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ hover https://example.com "a"
  # Navigate then hover

Flags:
  -h, --help   help for hover

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "html" => r#"Get HTML content of the page or an element

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ html [selector] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ html
  # Get full page HTML

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ html "div.content"
  # Get innerHTML of a specific element

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ html "div.content" --outer
  # Get outerHTML of a specific element

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ html https://example.com "h1"
  # Navigate then get element HTML

Flags:
  -h, --help    help for html
      --outer   Return outerHTML instead of innerHTML

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "install" => r#"Download Chrome for Testing and chromedriver

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ install [flags]

Flags:
  -h, --help   help for install

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "is" => r#"Check element state (visible, enabled, checked, actionable)

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ is [flags]
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ is [command]

Available Commands:
  actionable  Check actionability of an element (Visible, Stable, ReceivesEvents, Enabled, Editable)
  checked     Check if a checkbox or radio is checked
  enabled     Check if an element is enabled
  visible     Check if an element is visible on the page

Flags:
  -h, --help   help for is

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging

Use "__BROWSERLANE_PROG_NAME_PLACEHOLDER__ is [command] --help" for more information about a command.
"#,
        "is actionable" => r#"Check actionability of an element (Visible, Stable, ReceivesEvents, Enabled, Editable)

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ is actionable [url] [selector] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ is actionable https://example.com "a"
  # Output:
  # Checking actionability for selector: a
  # ✓ Visible: true
  # ✓ Stable: true
  # ✓ ReceivesEvents: true
  # ✓ Enabled: true
  # ✗ Editable: false

Flags:
  -h, --help   help for actionable

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "is checked" => r#"Check if a checkbox or radio is checked

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ is checked [selector] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ is checked "input[type=checkbox]"
  # Prints true or false

Flags:
  -h, --help   help for checked

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "is enabled" => r#"Check if an element is enabled

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ is enabled [selector] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ is enabled "button[type=submit]"
  # Prints true or false

Flags:
  -h, --help   help for enabled

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "is visible" => r#"Check if an element is visible on the page

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ is visible [selector] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ is visible "h1"
  # Prints true or false

Flags:
  -h, --help   help for visible

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "is-installed" => r#"Check if Chrome and chromedriver are installed (exit 0 = yes, exit 1 = no)

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ is-installed [flags]

Flags:
  -h, --help   help for is-installed

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "keys" => r#"Press a key or key combination

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ keys [keys] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ keys Enter
  # Press Enter

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ keys "Control+a"
  # Select all

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ keys "Shift+Tab"
  # Shift+Tab to previous field

Flags:
  -h, --help   help for keys

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "launch-test" => r#"Launch browser via chromedriver and print BiDi WebSocket URL

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ launch-test [flags]

Flags:
  -h, --help   help for launch-test

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "map" => r#"Map interactive page elements with @refs

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ map [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ map
  # Lists interactive elements with refs like @e1, @e2
  # Use refs with other commands: __BROWSERLANE_PROG_NAME_PLACEHOLDER__ click @e1

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ map --selector "nav"
  # Only map elements inside the <nav> element

Flags:
  -h, --help              help for map
      --selector string   Scope to elements within this CSS selector

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "mcp" => r#"Start the Model Context Protocol (MCP) server.

This runs a JSON-RPC 2.0 server over stdin/stdout, designed for integration
with LLM agents like Claude Code.

The server provides browser automation tools:
  - browser_start: Start a browser session
  - browser_navigate: Go to a URL
  - browser_click: Click an element
  - browser_type: Type into an element
  - browser_screenshot: Capture the page
  - browser_find: Find element info
  - browser_evaluate: Execute JavaScript
  - browser_stop: Stop the browser
  - browser_get_text: Get page/element text
  - browser_get_url: Get current URL
  - browser_get_title: Get page title
  - browser_get_html: Get page/element HTML
  - browser_find_all: Find all matching elements
  - browser_wait: Wait for element state
  - browser_hover: Hover over an element
  - browser_select: Select a dropdown option
  - browser_scroll: Scroll the page
  - browser_keys: Press keys
  - browser_new_page: Open a new page
  - browser_list_pages: List open pages
  - browser_switch_page: Switch pages
  - browser_close_page: Close a page

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ mcp [flags]

Examples:
  # Run directly (for testing)
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ mcp

  # Configure in Claude Code
  claude mcp add __BROWSERLANE_PROG_NAME_PLACEHOLDER__ -- __BROWSERLANE_PROG_NAME_PLACEHOLDER__ mcp

  # Custom screenshot directory
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ mcp --screenshot-dir ./screenshots

  # Disable screenshot file saving (inline only)
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ mcp --screenshot-dir ""

  # Test with echo
  echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}' | __BROWSERLANE_PROG_NAME_PLACEHOLDER__ mcp

Flags:
  -h, --help                    help for mcp
      --screenshot-dir string   Directory for saving screenshots (default: ~/Pictures/browserlane, use "" to disable)

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "media" => r#"Override CSS media features

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ media [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ media --color-scheme dark
  # Enable dark mode

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ media --reduced-motion reduce
  # Reduce motion

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ media --color-scheme light --forced-colors active
  # Override multiple features

Flags:
      --color-scheme string     Color scheme: light, dark, no-preference
      --contrast string         Contrast: more, less, no-preference
      --forced-colors string    Forced colors: active, none
  -h, --help                    help for media
      --media string            Media type: screen, print
      --reduced-motion string   Reduced motion: reduce, no-preference

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "mouse" => r#"Mouse control (click, move, down, up)

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ mouse [flags]
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ mouse [command]

Available Commands:
  click       Click at coordinates or current position
  down        Press a mouse button down
  move        Move the mouse to coordinates
  up          Release a mouse button

Flags:
  -h, --help   help for mouse

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging

Use "__BROWSERLANE_PROG_NAME_PLACEHOLDER__ mouse [command] --help" for more information about a command.
"#,
        "mouse click" => r#"Click at coordinates or current position

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ mouse click [x] [y] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ mouse click 100 200
  # Left click at (100, 200)

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ mouse click 100 200 --button 2
  # Right click at (100, 200)

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ mouse click
  # Left click at current position

Flags:
      --button int   Mouse button (0=left, 1=middle, 2=right)
  -h, --help         help for click

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "mouse down" => r#"Press a mouse button down

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ mouse down [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ mouse down
  # Press left mouse button

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ mouse down --button 2
  # Press right mouse button

Flags:
      --button int   Mouse button (0=left, 1=middle, 2=right)
  -h, --help         help for down

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "mouse move" => r#"Move the mouse to coordinates

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ mouse move [x] [y] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ mouse move 100 200
  # Move mouse to position (100, 200)

Flags:
  -h, --help   help for move

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "mouse up" => r#"Release a mouse button

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ mouse up [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ mouse up
  # Release left mouse button

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ mouse up --button 2
  # Release right mouse button

Flags:
      --button int   Mouse button (0=left, 1=middle, 2=right)
  -h, --help         help for up

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "page" => r#"Manage browser pages (new, close, switch)

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ page [flags]
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ page [command]

Available Commands:
  close       Close a browser page by index (default: current page)
  new         Open a new browser page
  switch      Switch to a browser page by index or URL substring

Flags:
  -h, --help   help for page

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging

Use "__BROWSERLANE_PROG_NAME_PLACEHOLDER__ page [command] --help" for more information about a command.
"#,
        "page close" => r#"Close a browser page by index (default: current page)

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ page close [index] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ page close
  # Close current page (index 0)

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ page close 1
  # Close page at index 1

Flags:
  -h, --help   help for close

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "page new" => r#"Open a new browser page

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ page new [url] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ page new
  # Open a blank new page

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ page new https://example.com
  # Open a new page and navigate to URL

Flags:
  -h, --help   help for new

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "page switch" => r#"Switch to a browser page by index or URL substring

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ page switch [index or url] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ page switch 1
  # Switch to page at index 1

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ page switch google.com
  # Switch to page containing "google.com" in URL

Flags:
  -h, --help   help for switch

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "pages" => r#"List all open browser pages

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ pages [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ pages
  # [0] https://example.com
  # [1] https://google.com

Flags:
  -h, --help   help for pages

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "paths" => r#"Print browser and cache paths

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ paths [flags]

Flags:
  -h, --help   help for paths

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "pdf" => r#"Save page as PDF

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ pdf [url] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ pdf -o page.pdf
  # Save current page as PDF

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ pdf https://example.com -o page.pdf
  # Navigate to URL first, then save as PDF

Flags:
  -h, --help            help for pdf
  -o, --output string   Output file path (default "page.pdf")

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "pipe" => r#"Start browserlane in pipe mode where protocol messages are exchanged
over stdin (commands) and stdout (responses/events) as newline-delimited JSON.
Diagnostic output goes to stderr. This mode is used by client libraries.

Use --connect to proxy to a remote BiDi endpoint instead of launching a local browser.

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ pipe [flags]

Examples:
  echo '{"id":1,"method":"browserlane:browser.page","params":{}}' | __BROWSERLANE_PROG_NAME_PLACEHOLDER__ pipe --headless

  # Connect to a remote browser
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ pipe --connect ws://remote:9515

  # Connect with auth header
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ pipe --connect wss://cloud.example.com/bidi --connect-header "Authorization: Bearer token"

Flags:
      --connect string               Connect to a remote BiDi WebSocket URL instead of launching a local browser
      --connect-header stringArray   HTTP header for WebSocket connect (repeatable, format: "Key: Value")
  -h, --help                         help for pipe

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "press" => r#"Press a key on a specific element or the focused element

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ press [key] [selector] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ press Enter
  # Press Enter on the currently focused element

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ press Enter "input[name=search]"
  # Click to focus the input, then press Enter

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ press "Control+a"
  # Select all

Flags:
  -h, --help   help for press

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "record" => r#"Record browser sessions (screenshots and snapshots)

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record [flags]
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record [command]

Available Commands:
  chunk       Manage recording chunks
  group       Manage recording groups
  start       Start a recording
  stop        Stop recording and save

Flags:
  -h, --help   help for record

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging

Use "__BROWSERLANE_PROG_NAME_PLACEHOLDER__ record [command] --help" for more information about a command.
"#,
        "record chunk" => r#"Manage recording chunks

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record chunk [flags]
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record chunk [command]

Available Commands:
  start       Start a new chunk within the current recording
  stop        Package current chunk into a ZIP file (recording stays active)

Flags:
  -h, --help   help for chunk

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging

Use "__BROWSERLANE_PROG_NAME_PLACEHOLDER__ record chunk [command] --help" for more information about a command.
"#,
        "record chunk start" => r#"Start a new chunk within the current recording

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record chunk start [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record chunk start
  # Start a new chunk (for splitting long recordings)

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record chunk start --name "part2" --title "Checkout Flow"

Flags:
  -h, --help           help for start
      --name string    Name for the chunk
      --title string   Title shown in trace viewer

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "record chunk stop" => r#"Package current chunk into a ZIP file (recording stays active)

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record chunk stop [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record chunk stop
  # Save chunk to chunk.zip

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record chunk stop -o part1.zip

Flags:
  -h, --help            help for stop
  -o, --output string   Output file path (default: chunk.zip)

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "record group" => r#"Manage recording groups

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record group [flags]
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record group [command]

Available Commands:
  start       Start a named group in the recording
  stop        End the current recording group

Flags:
  -h, --help   help for group

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging

Use "__BROWSERLANE_PROG_NAME_PLACEHOLDER__ record group [command] --help" for more information about a command.
"#,
        "record group start" => r#"Start a named group in the recording

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record group start <name> [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record group start "Login"
  # Groups nest actions in the trace viewer

Flags:
  -h, --help   help for start

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "record group stop" => r#"End the current recording group

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record group stop [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record group stop

Flags:
  -h, --help   help for stop

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "record start" => r#"Start a recording

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record start [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record start
  # Start recording with screenshots (default)

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record start --screenshots=false
  # Record without screenshots

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record start --snapshots
  # Record with screenshots and HTML snapshots

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record start --format png
  # Use PNG format instead of JPEG (larger files, lossless)

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record start --quality 0.1
  # Lower JPEG quality for smaller recording files

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record start --title "Login Flow"
  # Set a title shown in the trace viewer

Flags:
      --bidi            Record raw BiDi commands in the recording
      --format string   Screenshot format: jpeg or png (default "jpeg")
  -h, --help            help for start
      --name string     Name for the recording
      --quality float   JPEG quality 0.0-1.0 (ignored for png) (default 0.5)
      --screenshots     Capture screenshots after each action (default true)
      --snapshots       Capture HTML snapshots
      --sources         Include source information
      --title string    Title shown in trace viewer (defaults to name)

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "record stop" => r#"Stop recording and save

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record stop [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record stop
  # Save recording to record.zip

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ record stop -o my-recording.zip
  # Save recording to custom path

Flags:
  -h, --help            help for stop
  -o, --output string   Output file path (default: record.zip)

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "reload" => r#"Reload the current page

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ reload [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ reload
  # Reload the current page

Flags:
  -h, --help   help for reload

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "screenshot" => r#"Capture a screenshot (optionally navigate to URL first)

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ screenshot [url] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ screenshot -o shot.png
  # Screenshots the current page

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ screenshot https://example.com -o shot.png
  # Navigates to URL first, then screenshots

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ screenshot -o full.png --full-page
  # Capture the entire page (not just the viewport)

Flags:
      --annotate        Annotate interactive elements with numbered labels
      --full-page       Capture the full page instead of just the viewport
  -h, --help            help for screenshot
  -o, --output string   Output file path (default "screenshot.png")

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "scroll" => r#"Scroll the page or an element

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ scroll [direction] [flags]
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ scroll [command]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ scroll
  # Scroll down by default

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ scroll up
  # Scroll up

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ scroll down --amount 5
  # Scroll down 5 increments

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ scroll down --selector "div.content"
  # Scroll within a specific element

Available Commands:
  into-view   Scroll an element into view

Flags:
      --amount int        Number of scroll increments (default 3)
  -h, --help              help for scroll
      --selector string   CSS selector for element to scroll to

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging

Use "__BROWSERLANE_PROG_NAME_PLACEHOLDER__ scroll [command] --help" for more information about a command.
"#,
        "scroll into-view" => r##"Scroll an element into view

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ scroll into-view [selector] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ scroll into-view "#footer"
  # Scroll the footer element into view (centered on screen)

Flags:
  -h, --help   help for into-view

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"##,
        "select" => r#"Select an option in a <select> element

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ select [selector] [value] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ select "select#color" "blue"
  # Select "blue" in the color dropdown

Flags:
  -h, --help   help for select

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "serve" => r#"Start WebSocket proxy server for browser automation

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ serve [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ serve
  # Starts server on default port 9515, visible browser

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ serve --port 8080
  # Starts server on port 8080

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ serve --headless
  # Starts server with headless browser

Flags:
  -h, --help       help for serve
  -p, --port int   Port to listen on (default 9515)

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "sleep" => r#"Pause execution for a number of milliseconds

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ sleep [ms] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ sleep 1000
  # Wait 1 second

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ sleep 500
  # Wait 500ms

Flags:
  -h, --help   help for sleep

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "start" => r#"Start a browser session. Without arguments, launches a local browser.
With a URL argument, connects to a remote BiDi WebSocket endpoint.

If no URL is given, checks BROWSERLANE_CONNECT_URL env var before falling
back to a local browser launch.

Set BROWSERLANE_CONNECT_API_KEY to send an Authorization: Bearer header.

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ start [url] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ start
  # Start with a local browser

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ start ws://remote:9515/session
  # Connect to a remote browser

  export BROWSERLANE_CONNECT_URL=wss://cloud.example.com/session
  export BROWSERLANE_CONNECT_API_KEY=my-api-key
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ start
  # Connect using env vars

Flags:
  -h, --help   help for start

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "stop" => r#"Stop the browser session

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ stop [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ stop
  # Stop the browser and daemon

Flags:
  -h, --help   help for stop

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "storage" => r#"Export or restore browser state (cookies, localStorage, sessionStorage)

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ storage [flags]
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ storage [command]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ storage
  # Print state as JSON

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ storage -o state.json
  # Save state to file

Available Commands:
  restore     Restore browser state from a JSON file

Flags:
  -h, --help            help for storage
  -o, --output string   Output file path

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging

Use "__BROWSERLANE_PROG_NAME_PLACEHOLDER__ storage [command] --help" for more information about a command.
"#,
        "storage restore" => r#"Restore browser state from a JSON file

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ storage restore [path] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ storage restore state.json
  # Restore cookies and storage from saved state

Flags:
  -h, --help   help for restore

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "text" => r#"Get text content of the page or an element

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ text [selector] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ text
  # Get all page text

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ text "h1"
  # Get text of a specific element

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ text https://example.com
  # Navigate then get all page text

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ text https://example.com "h1"
  # Navigate then get element text

Flags:
  -h, --help   help for text

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "title" => r#"Get the current page title

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ title [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ title
  # Prints: Example Domain

Flags:
  -h, --help   help for title

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "type" => r#"Type text into an element (optionally navigate to URL first)

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ type [url] [selector] [text] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ type "input" "12345"
  # Types on current page

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ type https://the-internet.herokuapp.com/inputs "input" "12345"
  # Navigates to URL first, then types

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ type https://the-internet.herokuapp.com/inputs "input" "12345" --timeout 5s
  # Custom timeout for actionability checks

Flags:
  -h, --help               help for type
      --timeout duration   Timeout for actionability checks (e.g., 5s, 30s) (default 30s)

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "uncheck" => r#"Uncheck a checkbox

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ uncheck [selector] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ uncheck "input[name=agree]"
  # Uncheck the "agree" checkbox (idempotent)

Flags:
  -h, --help   help for uncheck

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "upload" => r##"Set files on an input[type=file] element

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ upload [selector] [files...] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ upload "input[type=file]" ./photo.jpg
  # Upload a single file

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ upload "#file-input" ./photo.jpg ./doc.pdf
  # Upload multiple files

Flags:
  -h, --help   help for upload

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"##,
        "url" => r#"Get the current page URL

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ url [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ url
  # Prints: https://example.com

Flags:
  -h, --help   help for url

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "value" => r#"Get the current value of a form element

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ value [selector] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ value "input[name=email]"
  # Print the current value of the email input

Flags:
  -h, --help   help for value

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "version" => r#"Print the version number

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ version [flags]

Flags:
  -h, --help   help for version

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "viewport" => r#"Get or set the browser viewport size

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ viewport [width] [height] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ viewport
  # {"width":1280,"height":720,"devicePixelRatio":1}

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ viewport 1280 720
  # Set viewport to 1280x720

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ viewport 375 812 --dpr 3
  # Simulate iPhone X viewport

Flags:
      --dpr float   Device pixel ratio (e.g., 2 for Retina)
  -h, --help        help for viewport

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "wait" => r#"Wait for an element, URL, text, page load, or JS condition

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ wait [selector] [flags]
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ wait [command]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ wait "div.loaded"
  # Wait for element to exist in DOM

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ wait "div.loaded" --state visible
  # Wait for element to be visible

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ wait "div.spinner" --state hidden --timeout 5000
  # Wait for spinner to disappear

Available Commands:
  fn          Wait until a JS expression returns truthy
  load        Wait until the page is fully loaded
  text        Wait until text appears on the page
  url         Wait until the page URL contains a substring

Flags:
  -h, --help           help for wait
      --state string   State to wait for: attached, visible, hidden (default "attached")
      --timeout int    Timeout in milliseconds (default 30000)

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging

Use "__BROWSERLANE_PROG_NAME_PLACEHOLDER__ wait [command] --help" for more information about a command.
"#,
        "wait fn" => r#"Wait until a JS expression returns truthy

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ wait fn [expression] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ wait fn "document.readyState === 'complete'"
  # Wait for page to be fully loaded

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ wait fn "window.ready === true" --timeout 10000
  # Wait for custom condition with timeout

Flags:
  -h, --help            help for fn
      --timeout float   Timeout in milliseconds (default 30000)

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "wait load" => r#"Wait until the page is fully loaded

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ wait load [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ wait load
  # Wait until document.readyState is "complete"

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ wait load --timeout 10000
  # Wait up to 10 seconds

Flags:
  -h, --help          help for load
      --timeout int   Timeout in milliseconds (default 30000)

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "wait text" => r#"Wait until text appears on the page

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ wait text [text] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ wait text "Welcome"
  # Waits until "Welcome" appears on the page

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ wait text "Success" --timeout 10000
  # Wait with custom timeout (10 seconds)

Flags:
  -h, --help            help for text
      --timeout float   Timeout in milliseconds (default 30000)

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "wait url" => r#"Wait until the page URL contains a substring

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ wait url [pattern] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ wait url "/dashboard"
  # Wait until URL contains "/dashboard"

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ wait url "success" --timeout 10000
  # Wait up to 10 seconds

Flags:
  -h, --help          help for url
      --timeout int   Timeout in milliseconds (default 30000)

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "window" => r#"Get or set the OS browser window size, position, or state

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ window [width] [height] [x] [y] [flags]

Examples:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ window
  # {"state":"normal","x":0,"y":25,"width":1280,"height":720}

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ window 1920 1080
  # Set window to 1920x1080

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ window 1920 1080 0 0
  # Set window to 1920x1080 at position (0, 0)

  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ window --state maximized
  # Maximize the window

Flags:
  -h, --help           help for window
      --state string   Window state: normal, maximized, minimized, fullscreen

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        "ws-test" => r#"Test WebSocket connection (type messages, see echoes)

Usage:
  __BROWSERLANE_PROG_NAME_PLACEHOLDER__ ws-test [url] [flags]

Flags:
  -h, --help   help for ws-test

Global Flags:
      --headless   Hide browser window (visible by default)
      --json       Output as JSON
  -v, --verbose    Enable debug logging
"#,
        _ => return None,
    };
    Some(t)
}

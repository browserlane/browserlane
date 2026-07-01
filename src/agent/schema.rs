//! The MCP tool-schema catalog: GetToolSchemas mirrored as a Vec<Tool> with
//! each tool's JSON Schema built via json!. Faithful to the Go source (pure
//! static data); verified byte-equivalent to the Go binary's tools/list.

use serde_json::json;

use super::server::Tool;

/// Returns the list of available MCP tools with their schemas.
pub fn get_tool_schemas() -> Vec<Tool> {
    let mut tools: Vec<Tool> = vec![
        Tool {
            name: "browser_start".to_string(),
            description: "Start a browser session".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "headless": {
                        "type": "boolean",
                        "description": "Run browser in headless mode (no visible window)",
                        "default": false
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_navigate".to_string(),
            description: "Navigate to a URL in the browser".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to navigate to"
                    }
                },
                "required": ["url"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_click".to_string(),
            description: "Click an element by CSS selector. Waits for element to be visible, stable, and enabled.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for the element to click"
                    }
                },
                "required": ["selector"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_type".to_string(),
            description: "Type text into an element by CSS selector. Waits for element to be visible, stable, enabled, and editable.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for the element to type into"
                    },
                    "text": {
                        "type": "string",
                        "description": "The text to type"
                    }
                },
                "required": ["selector", "text"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_screenshot".to_string(),
            description: "Capture a screenshot of the current page".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "annotate": {
                        "type": "boolean",
                        "description": "Annotate interactive elements with numbered labels (default: false)",
                        "default": false
                    },
                    "filename": {
                        "type": "string",
                        "description": "Optional filename to save the screenshot (e.g., screenshot.png)"
                    },
                    "fullPage": {
                        "type": "boolean",
                        "description": "Capture the full page (entire document) instead of just the viewport (default: false)",
                        "default": false
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_find".to_string(),
            description: "Find an element and return its info (tag, text, bounding box). Use a CSS selector or a semantic locator (role, text, label, placeholder, testid, xpath, alt, title). Combine role with text or other locators to narrow results.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "alt": {
                        "type": "string",
                        "description": "Find element by alt attribute"
                    },
                    "label": {
                        "type": "string",
                        "description": "Find input by associated label text or aria-label"
                    },
                    "placeholder": {
                        "type": "string",
                        "description": "Find element by placeholder attribute"
                    },
                    "role": {
                        "type": "string",
                        "description": "ARIA role to match (e.g., \"button\", \"link\", \"textbox\", \"heading\", \"checkbox\")"
                    },
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for the element to find"
                    },
                    "testid": {
                        "type": "string",
                        "description": "Find element by data-testid attribute"
                    },
                    "text": {
                        "type": "string",
                        "description": "Find element containing this text"
                    },
                    "title": {
                        "type": "string",
                        "description": "Find element by title attribute"
                    },
                    "xpath": {
                        "type": "string",
                        "description": "Find element by XPath expression"
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_evaluate".to_string(),
            description: "Execute JavaScript in the browser to extract data, query the DOM, or inspect page state. Returns the evaluated result. Use this to get text content, attributes, element data, or any information from the page.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "expression": {
                        "type": "string",
                        "description": "JavaScript expression to evaluate"
                    }
                },
                "required": ["expression"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_stop".to_string(),
            description: "Stop the browser session".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_get_html".to_string(),
            description: "Get the HTML content of the page or a specific element".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "outer": {
                        "type": "boolean",
                        "description": "Return outerHTML instead of innerHTML (default: false)",
                        "default": false
                    },
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for a specific element (optional, defaults to full page HTML)"
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_find_all".to_string(),
            description: "Find all elements matching a CSS selector and return their info (tag, text, bounding box)".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "number",
                        "description": "Maximum number of elements to return (default: 10)",
                        "default": 10
                    },
                    "selector": {
                        "type": "string",
                        "description": "CSS selector to match elements"
                    }
                },
                "required": ["selector"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_wait".to_string(),
            description: "Wait for an element to reach a specified state (attached, visible, or hidden)".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for the element to wait for"
                    },
                    "state": {
                        "type": "string",
                        "description": "State to wait for: \"attached\" (exists in DOM), \"visible\" (visible on page), or \"hidden\" (not found or not visible)",
                        "enum": ["attached", "visible", "hidden"],
                        "default": "attached"
                    },
                    "timeout": {
                        "type": "number",
                        "description": "Timeout in milliseconds (default: 30000)",
                        "default": 30000
                    }
                },
                "required": ["selector"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_new_page".to_string(),
            description: "Open a new browser page, optionally navigating to a URL".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "URL to navigate to in the new page (optional)"
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_list_pages".to_string(),
            description: "List all open browser pages with their URLs".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_switch_page".to_string(),
            description: "Switch to a browser page by index or URL substring".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "index": {
                        "type": "number",
                        "description": "Page index (0-based) from browser_list_pages"
                    },
                    "url": {
                        "type": "string",
                        "description": "URL substring to match (alternative to index)"
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_close_page".to_string(),
            description: "Close a browser page by index (default: current page)".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "index": {
                        "type": "number",
                        "description": "Page index to close (default: 0, the current page)",
                        "default": 0
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_hover".to_string(),
            description: "Hover over an element by CSS selector".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for the element to hover over"
                    }
                },
                "required": ["selector"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_select".to_string(),
            description: "Select an option in a <select> element by value".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for the <select> element"
                    },
                    "value": {
                        "type": "string",
                        "description": "The value to select"
                    }
                },
                "required": ["selector", "value"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_scroll".to_string(),
            description: "Scroll the page or a specific element".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "amount": {
                        "type": "number",
                        "description": "Number of scroll increments (default: 3)",
                        "default": 3
                    },
                    "direction": {
                        "type": "string",
                        "description": "Scroll direction: up, down, left, right (default: down)",
                        "enum": ["up", "down", "left", "right"],
                        "default": "down"
                    },
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for element to scroll to (optional, defaults to viewport center)"
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_keys".to_string(),
            description: "Press a key or key combination (e.g., \"Enter\", \"Control+a\", \"Shift+Tab\")".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "keys": {
                        "type": "string",
                        "description": "Key or key combination to press (e.g., \"Enter\", \"Control+a\", \"Shift+ArrowDown\")"
                    }
                },
                "required": ["keys"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_get_text".to_string(),
            description: "Get the text content of the page or a specific element".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for a specific element (optional, defaults to full page text)"
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_get_url".to_string(),
            description: "Get the current page URL".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_get_title".to_string(),
            description: "Get the current page title".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_a11y_tree".to_string(),
            description: "Get the accessibility tree of the current page. Returns a tree of ARIA roles, names, and states — useful for understanding page structure without visual rendering.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "everything": {
                        "type": "boolean",
                        "description": "Show all nodes including generic containers. Default: false",
                        "default": false
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "page_clock_install".to_string(),
            description: "Install a fake clock on the page, overriding Date, setTimeout, setInterval, requestAnimationFrame, and performance.now".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "time": {
                        "type": "number",
                        "description": "Initial time as epoch milliseconds (optional)"
                    },
                    "timezone": {
                        "type": "string",
                        "description": "IANA timezone ID to override (e.g. 'America/New_York', 'Europe/London')"
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "page_clock_fast_forward".to_string(),
            description: "Jump the fake clock forward by N milliseconds, firing each due timer at most once".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "ticks": {
                        "type": "number",
                        "description": "Number of milliseconds to fast-forward"
                    }
                },
                "required": ["ticks"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "page_clock_run_for".to_string(),
            description: "Advance the fake clock by N milliseconds, firing all time-related callbacks systematically".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "ticks": {
                        "type": "number",
                        "description": "Number of milliseconds to advance"
                    }
                },
                "required": ["ticks"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "page_clock_pause_at".to_string(),
            description: "Jump the fake clock to a specific time and pause — no timers fire until resumed or advanced".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "time": {
                        "type": "number",
                        "description": "Time as epoch milliseconds to pause at"
                    }
                },
                "required": ["time"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "page_clock_resume".to_string(),
            description: "Resume real-time progression from the current fake clock time".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        Tool {
            name: "page_clock_set_fixed_time".to_string(),
            description: "Freeze Date.now() at a specific value permanently. Timers still run.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "time": {
                        "type": "number",
                        "description": "Time as epoch milliseconds to freeze at"
                    }
                },
                "required": ["time"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "page_clock_set_system_time".to_string(),
            description: "Set Date.now() to a specific value without triggering any timers".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "time": {
                        "type": "number",
                        "description": "Time as epoch milliseconds to set"
                    }
                },
                "required": ["time"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "page_clock_set_timezone".to_string(),
            description: "Override the browser timezone. Pass an IANA timezone ID (e.g. 'America/New_York'), or empty string to reset to system default".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "timezone": {
                        "type": "string",
                        "description": "IANA timezone ID (e.g. 'America/New_York', 'Europe/London', 'Asia/Tokyo'). Empty string resets to system default."
                    }
                },
                "required": ["timezone"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_fill".to_string(),
            description: "Clear an input field and type new text. Waits for element to be editable, clears existing value, then types. Use this instead of browser_type when you want to replace the field contents.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for the input element"
                    },
                    "text": {
                        "type": "string",
                        "description": "The text to fill in"
                    }
                },
                "required": ["selector", "text"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_press".to_string(),
            description: "Press a key or key combination on a specific element or the focused element. If selector is given, clicks the element first to focus it, then presses the key.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "key": {
                        "type": "string",
                        "description": "Key or key combination to press (e.g., \"Enter\", \"Control+a\", \"Escape\")"
                    },
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for the element to focus before pressing (optional, defaults to currently focused element)"
                    }
                },
                "required": ["key"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_back".to_string(),
            description: "Navigate back in browser history (like clicking the back button)".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_forward".to_string(),
            description: "Navigate forward in browser history (like clicking the forward button)".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_reload".to_string(),
            description: "Reload the current page. Waits for the page to fully load.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_get_value".to_string(),
            description: "Get the current value of an input, textarea, or select element".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for the form element"
                    }
                },
                "required": ["selector"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_get_attribute".to_string(),
            description: "Get the value of an HTML attribute on an element".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "attribute": {
                        "type": "string",
                        "description": "Attribute name to retrieve (e.g., \"href\", \"src\", \"class\", \"data-id\")"
                    },
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for the element"
                    }
                },
                "required": ["selector", "attribute"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_is_visible".to_string(),
            description: "Check if an element is visible on the page. Returns true/false without throwing errors.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for the element"
                    }
                },
                "required": ["selector"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_check".to_string(),
            description: "Check a checkbox or radio button. Idempotent — does nothing if already checked.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for the checkbox or radio button"
                    }
                },
                "required": ["selector"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_uncheck".to_string(),
            description: "Uncheck a checkbox. Idempotent — does nothing if already unchecked.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for the checkbox"
                    }
                },
                "required": ["selector"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_scroll_into_view".to_string(),
            description: "Scroll an element into view, centering it on screen".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for the element to scroll into view"
                    }
                },
                "required": ["selector"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_wait_for_url".to_string(),
            description: "Wait until the page URL contains a given substring".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Substring to match in the URL"
                    },
                    "timeout": {
                        "type": "number",
                        "description": "Timeout in milliseconds (default: 30000)",
                        "default": 30000
                    }
                },
                "required": ["pattern"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_wait_for_load".to_string(),
            description: "Wait until the page reaches the \"complete\" ready state (all resources loaded)".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "timeout": {
                        "type": "number",
                        "description": "Timeout in milliseconds (default: 30000)",
                        "default": 30000
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_sleep".to_string(),
            description: "Pause execution for a specified number of milliseconds. Use sparingly — prefer browser_wait or browser_wait_for_url when possible.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "ms": {
                        "type": "number",
                        "description": "Number of milliseconds to sleep (max 30000)"
                    }
                },
                "required": ["ms"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_map".to_string(),
            description: "Map interactive page elements with @refs for targeting. Returns a list of interactive elements (buttons, links, inputs, etc.) each with a short @ref like @e1, @e2. Use these refs as selectors in other commands (click, fill, etc.).".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector to scope element discovery to a subtree (e.g. \"nav\", \"#sidebar\")"
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_diff_map".to_string(),
            description: "Compare current page state vs last map. Shows additions (+) and removals (-) since the last browser_map call.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_pdf".to_string(),
            description: "Save the current page as a PDF file".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "filename": {
                        "type": "string",
                        "description": "Output filename for the PDF (e.g., page.pdf)"
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_highlight".to_string(),
            description: "Highlight an element with a red outline for 3 seconds. Useful for visual debugging.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector or @ref for the element to highlight"
                    }
                },
                "required": ["selector"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_dblclick".to_string(),
            description: "Double-click an element by CSS selector or @ref".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector or @ref for the element to double-click"
                    }
                },
                "required": ["selector"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_focus".to_string(),
            description: "Focus an element by CSS selector or @ref".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector or @ref for the element to focus"
                    }
                },
                "required": ["selector"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_count".to_string(),
            description: "Count the number of elements matching a CSS selector".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector to count matches for"
                    }
                },
                "required": ["selector"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_is_enabled".to_string(),
            description: "Check if an element is enabled (not disabled). Returns true/false.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector or @ref for the element"
                    }
                },
                "required": ["selector"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_is_checked".to_string(),
            description: "Check if a checkbox or radio button is checked. Returns true/false.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector or @ref for the checkbox/radio element"
                    }
                },
                "required": ["selector"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_wait_for_text".to_string(),
            description: "Wait until specific text appears on the page".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "Text to wait for on the page"
                    },
                    "timeout": {
                        "type": "number",
                        "description": "Timeout in milliseconds (default: 30000)",
                        "default": 30000
                    }
                },
                "required": ["text"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_wait_for_fn".to_string(),
            description: "Wait until a JavaScript expression returns a truthy value".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "expression": {
                        "type": "string",
                        "description": "JavaScript expression to evaluate (e.g., \"window.ready === true\")"
                    },
                    "timeout": {
                        "type": "number",
                        "description": "Timeout in milliseconds (default: 30000)",
                        "default": 30000
                    }
                },
                "required": ["expression"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_dialog_accept".to_string(),
            description: "Accept a dialog (alert, confirm, prompt). Optionally provide text for prompt dialogs.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "text": {
                        "type": "string",
                        "description": "Text to enter in the prompt dialog (optional)"
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_dialog_dismiss".to_string(),
            description: "Dismiss a dialog (cancel/close)".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_get_cookies".to_string(),
            description: "List all cookies for the current page".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_set_cookie".to_string(),
            description: "Set a cookie".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "domain": {
                        "type": "string",
                        "description": "Cookie domain (optional, defaults to current page domain)"
                    },
                    "name": {
                        "type": "string",
                        "description": "Cookie name"
                    },
                    "path": {
                        "type": "string",
                        "description": "Cookie path (optional, defaults to /)"
                    },
                    "value": {
                        "type": "string",
                        "description": "Cookie value"
                    }
                },
                "required": ["name", "value"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_delete_cookies".to_string(),
            description: "Delete cookies. If name is given, deletes that cookie. Otherwise deletes all cookies.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Cookie name to delete (optional, omit to delete all)"
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_mouse_move".to_string(),
            description: "Move the mouse to specific coordinates".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "x": {
                        "type": "number",
                        "description": "X coordinate"
                    },
                    "y": {
                        "type": "number",
                        "description": "Y coordinate"
                    }
                },
                "required": ["x", "y"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_mouse_down".to_string(),
            description: "Press a mouse button down at the current position".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "button": {
                        "type": "number",
                        "description": "Mouse button (0=left, 1=middle, 2=right). Default: 0",
                        "default": 0
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_mouse_up".to_string(),
            description: "Release a mouse button at the current position".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "button": {
                        "type": "number",
                        "description": "Mouse button (0=left, 1=middle, 2=right). Default: 0",
                        "default": 0
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_mouse_click".to_string(),
            description: "Click at coordinates or at the current mouse position. If x and y are provided, moves the mouse there first.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "button": {
                        "type": "number",
                        "description": "Mouse button (0=left, 1=middle, 2=right). Default: 0",
                        "default": 0
                    },
                    "x": {
                        "type": "number",
                        "description": "X coordinate to click at"
                    },
                    "y": {
                        "type": "number",
                        "description": "Y coordinate to click at"
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_drag".to_string(),
            description: "Drag from one element to another".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "source": {
                        "type": "string",
                        "description": "CSS selector or @ref for the source element"
                    },
                    "target": {
                        "type": "string",
                        "description": "CSS selector or @ref for the target element"
                    }
                },
                "required": ["source", "target"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_set_viewport".to_string(),
            description: "Set the browser viewport size".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "devicePixelRatio": {
                        "type": "number",
                        "description": "Device pixel ratio (optional, e.g., 2 for Retina)"
                    },
                    "height": {
                        "type": "number",
                        "description": "Viewport height in pixels"
                    },
                    "width": {
                        "type": "number",
                        "description": "Viewport width in pixels"
                    }
                },
                "required": ["width", "height"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_get_viewport".to_string(),
            description: "Get the current viewport dimensions".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_get_window".to_string(),
            description: "Get the OS browser window dimensions and state".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_set_window".to_string(),
            description: "Set the OS browser window size, position, or state".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "height": {
                        "type": "number",
                        "description": "Window height in pixels"
                    },
                    "state": {
                        "type": "string",
                        "description": "Window state: normal, maximized, minimized, or fullscreen",
                        "enum": ["normal", "maximized", "minimized", "fullscreen"]
                    },
                    "width": {
                        "type": "number",
                        "description": "Window width in pixels"
                    },
                    "x": {
                        "type": "number",
                        "description": "Window x position in pixels"
                    },
                    "y": {
                        "type": "number",
                        "description": "Window y position in pixels"
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_emulate_media".to_string(),
            description: "Override CSS media features (color scheme, reduced motion, etc.)".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "colorScheme": {
                        "type": "string",
                        "description": "Color scheme: \"light\", \"dark\", or \"no-preference\"",
                        "enum": ["light", "dark", "no-preference"]
                    },
                    "contrast": {
                        "type": "string",
                        "description": "Contrast preference: \"more\", \"less\", or \"no-preference\"",
                        "enum": ["more", "less", "no-preference"]
                    },
                    "forcedColors": {
                        "type": "string",
                        "description": "Forced colors: \"active\" or \"none\"",
                        "enum": ["active", "none"]
                    },
                    "media": {
                        "type": "string",
                        "description": "Media type: \"screen\" or \"print\"",
                        "enum": ["screen", "print"]
                    },
                    "reducedMotion": {
                        "type": "string",
                        "description": "Reduced motion: \"reduce\" or \"no-preference\"",
                        "enum": ["reduce", "no-preference"]
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_set_geolocation".to_string(),
            description: "Override the browser geolocation".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "accuracy": {
                        "type": "number",
                        "description": "Accuracy in meters (default: 1)",
                        "default": 1
                    },
                    "latitude": {
                        "type": "number",
                        "description": "Latitude (-90 to 90)"
                    },
                    "longitude": {
                        "type": "number",
                        "description": "Longitude (-180 to 180)"
                    }
                },
                "required": ["latitude", "longitude"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_set_content".to_string(),
            description: "Replace the page HTML content".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "html": {
                        "type": "string",
                        "description": "HTML content to set"
                    }
                },
                "required": ["html"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_frames".to_string(),
            description: "List all child frames (iframes) on the current page".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_frame".to_string(),
            description: "Find a frame by name (exact match) or URL (substring match)".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "nameOrUrl": {
                        "type": "string",
                        "description": "Frame name (exact match) or URL substring to find"
                    }
                },
                "required": ["nameOrUrl"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_upload".to_string(),
            description: "Set files on an input[type=file] element".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "files": {
                        "type": "array",
                        "description": "Array of absolute file paths to upload",
                        "items": {
                            "type": "string"
                        }
                    },
                    "selector": {
                        "type": "string",
                        "description": "CSS selector for the file input element"
                    }
                },
                "required": ["selector", "files"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_record_start".to_string(),
            description: "Start a browser recording (screenshots and/or HTML snapshots). Output is Playwright trace viewer compatible.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "bidi": {
                        "type": "boolean",
                        "description": "Record raw BiDi commands in the recording (default: false)",
                        "default": false
                    },
                    "format": {
                        "type": "string",
                        "description": "Screenshot format: \"jpeg\" or \"png\" (default: \"jpeg\")",
                        "enum": ["jpeg", "png"],
                        "default": "jpeg"
                    },
                    "name": {
                        "type": "string",
                        "description": "Name for the recording (default: \"record\")"
                    },
                    "quality": {
                        "type": "number",
                        "description": "JPEG quality 0.0-1.0 (default: 0.5, ignored for png)",
                        "default": 0.5
                    },
                    "screenshots": {
                        "type": "boolean",
                        "description": "Capture screenshots after each action (default: true)",
                        "default": true
                    },
                    "snapshots": {
                        "type": "boolean",
                        "description": "Capture HTML snapshots (default: false)",
                        "default": false
                    },
                    "sources": {
                        "type": "boolean",
                        "description": "Include source information (default: false)",
                        "default": false
                    },
                    "title": {
                        "type": "string",
                        "description": "Title shown in trace viewer (defaults to name)"
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_record_stop".to_string(),
            description: "Stop recording and save to a Playwright-compatible trace ZIP file".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Output file path (default: record.zip)"
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_record_start_group".to_string(),
            description: "Start a named group in the recording (groups nest actions in the trace viewer)".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name for the group"
                    }
                },
                "required": ["name"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_record_stop_group".to_string(),
            description: "End the current recording group".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_record_start_chunk".to_string(),
            description: "Start a new chunk within the current recording (for splitting long recordings)".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name for the chunk"
                    },
                    "title": {
                        "type": "string",
                        "description": "Title shown in trace viewer (defaults to name)"
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_record_stop_chunk".to_string(),
            description: "Package the current recording chunk into a ZIP file (recording remains active)".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Output file path (default: chunk.zip)"
                    }
                },
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_storage_state".to_string(),
            description: "Export cookies, localStorage, and sessionStorage as JSON".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_restore_storage".to_string(),
            description: "Restore cookies and storage from a JSON state file".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the JSON state file"
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_download_set_dir".to_string(),
            description: "Set the download directory for the browser".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory path for downloads"
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
        Tool {
            name: "browser_expect".to_string(),
            description: "Assert a condition about the current page (URL, title, text, element state, count, or a JS expression). Returns a PASS message when the assertion holds; fails with the actual value otherwise. Use this to verify a browser flow reached the expected state.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "expected": {
                        "type": ["string", "number", "boolean"],
                        "description": "Expected value: the string/substring for url, title, text, and value targets, or the element count for the count target"
                    },
                    "expression": {
                        "type": "string",
                        "description": "JavaScript expression for the \"js\" target; the assertion passes when it evaluates to a truthy value (false, null, empty string, 0, and errors fail)"
                    },
                    "negate": {
                        "type": "boolean",
                        "description": "Invert the assertion (e.g. target \"checked\" with negate asserts the box is unchecked). Default: false",
                        "default": false
                    },
                    "operator": {
                        "type": "string",
                        "description": "How to compare the actual value with \"expected\" for the url, title, text, and value targets (default: \"contains\")",
                        "enum": ["contains", "equals"]
                    },
                    "selector": {
                        "type": "string",
                        "description": "CSS selector or @ref: required for visible, hidden, enabled, checked, value, and count targets; optional for text (defaults to full page text)"
                    },
                    "target": {
                        "type": "string",
                        "description": "What to assert on: \"url\", \"title\", \"text\" (page or element text), \"visible\", \"hidden\" (absent or not visible), \"enabled\", \"checked\" (element state), \"value\" (form element value), \"count\" (matching-element count), or \"js\" (expression truthiness)",
                        "enum": ["url", "title", "text", "visible", "hidden", "enabled", "checked", "value", "count", "js"]
                    }
                },
                "required": ["target"],
                "additionalProperties": false
            }),
        },
    ];
    // ext-seam (browserlane extension hook)
    crate::ext::register_mcp_tools(&mut tools);
    tools
}

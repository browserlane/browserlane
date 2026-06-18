//! Phase 3 interaction cluster: only the key-name resolver is ported here (used
//! by PressKey). The input.performActions builders/helpers are exercised through
//! the api layer and are ported with the input cluster as needed.

/// Resolves a key name to its WebDriver codepoint. If the name is not found in
/// the key map, it's returned as-is.
pub fn resolve_key(name: &str) -> String {
    match name {
        "Enter" => "\u{E006}",
        "Tab" => "\u{E004}",
        "Escape" => "\u{E00C}",
        "Backspace" => "\u{E003}",
        "Delete" => "\u{E017}",
        "ArrowUp" => "\u{E013}",
        "ArrowDown" => "\u{E015}",
        "ArrowLeft" => "\u{E012}",
        "ArrowRight" => "\u{E014}",
        "Home" => "\u{E011}",
        "End" => "\u{E010}",
        "PageUp" => "\u{E00E}",
        "PageDown" => "\u{E00F}",
        "Insert" => "\u{E016}",
        "Space" => " ",
        "Control" => "\u{E009}",
        "Shift" => "\u{E008}",
        "Alt" => "\u{E00A}",
        "Meta" => "\u{E03D}",
        "F1" => "\u{E031}",
        "F2" => "\u{E032}",
        "F3" => "\u{E033}",
        "F4" => "\u{E034}",
        "F5" => "\u{E035}",
        "F6" => "\u{E036}",
        "F7" => "\u{E037}",
        "F8" => "\u{E038}",
        "F9" => "\u{E039}",
        "F10" => "\u{E03A}",
        "F11" => "\u{E03B}",
        "F12" => "\u{E03C}",
        other => return other.to_string(),
    }
    .to_string()
}

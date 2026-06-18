/// Prints a ✓/✗ status line for a named boolean check (used by `is actionable`).
pub fn print_check(name: &str, passed: bool) {
    if passed {
        println!("✓ {name}: true");
    } else {
        println!("✗ {name}: false");
    }
}

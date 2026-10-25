// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `teamcode guard <roots-file>`: the PreToolUse hook of a protected workspace, answered before any Tauri setup
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("guard") {
        let roots = args.get(2).map(String::as_str).unwrap_or("");
        std::process::exit(teamcode_core::workspace::guard::run_guard(
            &mut std::io::stdin(),
            &mut std::io::stdout(),
            std::path::Path::new(roots),
        ));
    }
    teamcode_lib::run()
}

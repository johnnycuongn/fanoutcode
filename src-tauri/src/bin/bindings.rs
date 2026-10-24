//! `npm run bindings`: regenerates `src/lib/bindings.ts` (run from `src-tauri`).

fn main() {
    teamcode_lib::specta::export(std::path::Path::new("../src/lib/bindings.ts")).unwrap()
}

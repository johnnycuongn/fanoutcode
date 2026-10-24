# ADR 0001: Generated IPC bindings

- **Status:** accepted 2026-10-24

## Context

LiveSpec mirrors its Rust IPC types in TypeScript by hand and keeps a list of the mirrors in `AGENTS.md`. Team Code's
IPC surface (projects, workspaces, tasks, sessions, views, settings, git, tests, memory, Jira, cost, windows, and a
dozen events) is several times larger (spec §15.1), so hand mirrors would drift.

## Decision

tauri-specta generates `src/lib/bindings.ts` from the Rust commands and event structs. The file is committed,
`npm run bindings` regenerates it, and CI runs `npm run bindings:check`, which fails when it is stale. ESLint
(`teamcode/no-manual-ipc`) forbids importing `invoke` or `listen` from `@tauri-apps/api` anywhere but the generated
file.

## Consequences

- Every command carries `#[specta::specta]` and is registered in `specta::builder()`; every event derives
  `tauri_specta::Event`.
- `u64` and `i64` export as `number` (`BigIntExportBehavior::Number`); values above 2^53 are not expected.
- `teamcode-core` depends on `specta` (types only, no Tauri) so its domain types derive `specta::Type`.
- Pinned versions: see the table below, updated when they change.

| Crate | Version |
|---|---|
| specta | =2.0.0-rc.22 (features derive, chrono) |
| specta-typescript | =0.0.9 |
| tauri-specta | (recorded in Plan 01 Task 12) |

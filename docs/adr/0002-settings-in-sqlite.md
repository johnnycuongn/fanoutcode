# ADR 0002: Settings live in SQLite

- **Status:** accepted 2026-10-24 (contracts amendment A13)

## Context

Spec §14 put settings in JSON under `<app_config>`. Settings resolve along a scope chain, app → workspace → project →
task, most specific wins (§6.8, §13). That needs per-scope rows that Plan 03's `settings::resolve` reads, and the app
already opens `<app_data>/teamcode.db` for everything else.

## Decision

Settings live in the SQLite `settings` table, one row per `(scope_kind, scope_id, key)` with the value as JSON
(`scope_id` is `''` for the app scope). Nothing is written under app config. Secrets stay in the OS keychain.

## Consequences

- One store and one file to back up; settings migrate with the schema.
- A user cannot hand-edit settings in a text editor.
- The spec's §14 line "settings JSON in `<app_config>`" is superseded by this ADR.

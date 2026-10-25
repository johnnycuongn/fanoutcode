---
paths: ["src/**"]
---

# Webview rules

- Compose the primitives in `src/components/ui/`. Raw `<input>`, `<select>`, `<textarea>`, `<button>` and `<dialog>`
  appear only there; ESLint (`teamcode/no-raw-controls`) fails the build otherwise.
- Semantic tokens only: `bg-page`, `bg-bar`, `bg-well`, `bg-card`, `border-line`, `text-ink`, `text-mute`,
  `text-accent`, `text-ok`, `text-warn`, `text-bad`, `*-stage-{spec,plan,build,review,test,commit,done}`,
  `*-swatch-<name>` (via `swatchClass` in `src/lib/theme.ts`). No hex colours or inline colour styles outside
  `src/styles.css`.
- Animate with Motion, using `src/lib/motion.ts`. Honour `prefers-reduced-motion`: springs become 150 ms eases and
  loops stop.
- Every interactive element is reachable by keyboard, shows a `:focus-visible` ring and has an accessible name.
- State lives in module stores built with `createStore` (`src/lib/stores/createStore.ts`) and read with `use()` or
  `useSelector()`. Never add a state library.
- Agent and model output renders only through `MiniMarkdown` or xterm, never `innerHTML` or
  `dangerouslySetInnerHTML`.
- IPC only through `src/lib/bindings.ts` (`commands`, `events`). From `@tauri-apps/api/core` import only `Channel`
  (to receive a generated command's stream); nothing from `@tauri-apps/api/event` (`teamcode/no-manual-ipc`).
  Window and webview handles (`@tauri-apps/api/window`, `/webview`) are allowed for window-local work.
- A feature imports another feature only through `@/features/<name>` (its `index.ts`).
- A new ⌘ chord goes into `KEYMAP` in `src/lib/keyboard.ts`; the native menu (`src-tauri/src/menu.rs`) must not
  claim it (its test reads the keymap).
- Budgets: a component under about 200 lines, a store under 300.
- Nothing a user triggers takes longer than 250 ms to respond; show progress for anything slower.
- To add a primitive: `src/components/ui/<Name>.tsx` + `<Name>.test.tsx`, export it from `index.ts`, add it to
  `PRIMITIVES`, add `src/dev/primitives/pages/<Name>Page.tsx` and register it in `src/dev/primitives/registry.ts`;
  `registry.test.tsx` fails until all four exist.

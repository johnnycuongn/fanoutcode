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
- IPC only through `src/lib/bindings.ts` (`commands`, `events`); never import from `@tauri-apps/api/core` or
  `@tauri-apps/api/event`.
- A feature imports another feature only through `@/features/<name>` (its `index.ts`).
- Budgets: a component under about 200 lines, a store under 300.
- Nothing a user triggers takes longer than 250 ms to respond; show progress for anything slower.

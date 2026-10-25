# Plan 02 report: UI primitives

**Status: DONE_WITH_CONCERNS.** All 17 tasks are complete and every gate passes. The concerns are two manual browser
checks that need a browser or the GUI (Task 2 step 5 gallery in `npm run dev`, Task 15 step 5 animation playback), the
swatch palette ruling (Ruling 1) that changes what Plans 06 and 07 may pass as `swatch`, and plan text that does not run
as written (see "Plan text that is wrong").

- Worktree: `/Users/johnnynguyen/Personal/TeamCode-wt/plan-02`, branch `plan-02`. Not merged.
- Commit range: `88cc4aa` (Plan 01 final, also current `main`) .. `5781266` (18 commits, 108 files). `git rebase main` was a
  no-op: main had not advanced.
- Ledger: `.superpowers/sdd/plan-02-progress.md`, one line per task.
- Rust untouched. Files changed outside `src/`: `package.json`, `package-lock.json`, `vitest.config.ts`,
  `docs/adr/2026-10-24-ui-primitive-dependencies.md`, `ARCHITECTURE.md`, `.claude/rules/ui.md`, `CLAUDE.md` (one bullet).

## Gate results (final run on `5781266`)

| Gate | Result |
|---|---|
| `npm run typecheck` | exit 0, no output |
| `npm run lint` | ESLint clean; `All matched files use Prettier code style!` |
| `npx vitest run` | `Test Files 40 passed (40)`, `Tests 224 passed (224)`, no skips |
| `cargo test --workspace` | all `ok`: 240 (livespec-core) + 26 (teamcode-core) + 4 (shell) + ignored keychain/live tests, 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | `Finished`, no warnings |
| `npm run build` | `✓ built in 629ms`; `grep -rl "Primitives gallery" dist/` finds nothing (`not in bundle`) |
| lefthook `pre-commit` | typecheck, lint, test all `✔️` on every commit; no `--no-verify` |

Extra checks: a temporary build that referenced `EmptyState` emitted `EmptyStateLottie-*.js` as its own chunk and
`dotlottie-player-*.wasm` as a bundled asset (so the renderer is served from the bundle, not a CDN); the temporary edit
was reverted. The built CSS contains the dynamic-looking classes (`bg-swatch-rose`, `bg-accent/15`, `text-accent-ink`,
`animate-status-pulse`, `zoom-in-95`, `translate-x-4.5`, `size-30`).

Final test counts by file: cx 1, tokens 6, motion 4, Button 6, IconButton 4, KeyHint 11, Badge 7, StatusDot 5, Field 6,
Input 5, Textarea 3, Checkbox 5, Switch 3, Popover 3, Tooltip 4, Select 4, Combobox 7, Segmented 6, Tabs 4, Menu 6,
ContextMenu 3, Dialog 7, Drawer 5, Toast 3, Chip 9, Avatar 9, EmptyState 6, Panel 6, Resizable 10, index 2,
lintBoundary 5, registry 3, Gallery 4 (plus Plan 01's 62).

## Unverified

- **Task 2 step 5 second half and Task 15 step 5 (`npm run dev` in a browser)**, unverified: needs a browser. The
  gallery heading at `#/dev/primitives`, the Lottie animation playing, the `.wasm` coming from `localhost` in the
  Network panel, the still first frame under macOS Reduce motion, and the Task 17 step 6 dark-theme walk of every page
  were not done. What was checked instead: unit tests for every page's `<h1>`, the production build contains no gallery
  string, and the lazy chunk plus bundled wasm appear in a build that references `EmptyState`.
- The placeholder `empty-column.lottie` (a hand-written pulsing ring, CC0) was built with Python's `zipfile` in the
  dotLottie v1 layout (`manifest.json`, `animations/empty-pulse.json`). No renderer has opened it yet.
- Tauri CSP `'wasm-unsafe-eval'` is already in Plan 01's `tauri.conf.json`; not exercised in the real webview.

## Rulings

1. Ruling: `Swatch` is Plan 01's ten names (`rose orange yellow lime emerald cyan indigo violet fuchsia slate`, from
   `src/lib/theme.ts`), not the plan's eight (`violet indigo cyan mint lime gold coral magenta`). The theme file and
   `styles.test.ts` already define the ten `--color-swatch-*` tokens and overview A10 says Plan 07 uses
   `SWATCHES`/`swatchClass`. `tokens.ts` re-exports the type and `swatchClasses` is keyed by all ten (a test pins it to
   `SWATCHES`). Tests that used `coral` use `rose`. Cost if wrong: a consumer passing `mint`/`gold`/`coral`/`magenta`
   gets a type error at once.
2. Ruling: `--color-accent-ink` did not exist (the plan says Plan 01 defines it). I added it to `src/styles.css`
   (`@theme inline` alias plus `--tc-accent-ink: #ffffff` in both themes), plus the two `@import` lines the plan lists.
   Cost if wrong: Plan 01's file gained 5 lines; trivial to move.
3. Ruling: test config lives in `vitest.config.ts` (Plan 01), not a `test` block in `vite.config.ts`. I switched the
   global environment from `node` to `jsdom`, added `setupFiles: ["src/test/setup.ts"]` and `css: false`. The Node-only
   tests still pass (`eslint/rules`, `styles.test.ts`, `repoRules`); `lintBoundary.test.ts` opts into
   `// @vitest-environment node`. Plan 01's per-file `// @vitest-environment jsdom` comments are now redundant but
   harmless. Cost if wrong: a future Node-only test that touches `window` must opt out with the comment.
4. Ruling: Vitest globals stay off (Plan 01), so every test imports `describe/test/expect/vi`; the plan's test
   snippets omit the imports. Code is formatted with Plan 01's Prettier (double quotes, 120 columns), not the plan's
   single quotes.
5. Ruling: the ADR file keeps the plan's name `docs/adr/2026-10-24-ui-primitive-dependencies.md` rather than Plan 01's
   `NNNN-` numbering, so it cannot collide with an ADR number Plan 03 may take in parallel. Cost if wrong: a rename.
6. Ruling: `initials()` splits on whitespace first; only a single word is split on hyphens. The plan says "split on
   spaces and hyphens" but its own tests expect `best-practice reviewer` → `BR` and `codebase-consistency reviewer` →
   `CR`, which that rule cannot produce (it gives `BP`). Mine satisfies all four plan cases and `code-review` → `CR`.
7. Ruling: Radix roving focus does not put `tabindex="0"` on the active `Tabs`/`Segmented` item until the group is
   focused (the list container is the tab stop and hands focus to the active item). The plan's "tabindex is 0 on the
   active item" tests would fail. I replaced them with behaviour tests: `user.tab()` lands on the active item, which
   then has `tabindex="0"` and the others `-1`, and (Segmented) a second Tab leaves the group.
8. Ruling: the Segmented arrow-key tests hold the key (`{ArrowRight>}` ... `{/ArrowRight}`). Radix selects an item on
   arrow focus only while the arrow key is still down, and `user-event` releases it instantly. Real keyboards are
   unaffected.
9. Ruling: `Combobox` renders the Clear `IconButton` as a sibling absolutely positioned over the trigger, not inside it.
   The plan says "inside the trigger's trailing slot", which nests a `<button>` in a `<button>` (invalid HTML, React
   warns). Same for `Chip` with `onRemove`: the root becomes `<span role="group">` holding two sibling buttons (the plan
   says this too).
10. Ruling: `Avatar` does not pass `delayMs={0}` to Radix `Fallback`. With any `delayMs` Radix renders the fallback only
    after a timer, so the plan's synchronous "renders initials when there is no image" test fails; without it the
    initials show immediately and the image replaces them when loaded.
11. Ruling: the dotLottie wasm import is `@lottiefiles/dotlottie-web/dotlottie-player.wasm?url`; the plan's
    `.../dist/dotlottie-player.wasm` is not an exported path. I also added `@lottiefiles/dotlottie-web` as a direct
    exact dependency (`0.81.0`, the version the React wrapper pins) because importing a transitive dependency is
    fragile.
12. Ruling: `Drawer` casts Vaul's root props, and `DrawerProps` uses a distributive omit. Vaul's props are a union
    (with and without `fadeFromIndex`), and a plain `Omit` collapses it so the spread fails type-checking. Cost if wrong:
    none observable; behaviour is Vaul's.
13. Ruling: `Resizable` measures its container from the handle's `parentElement`, so no ref merging is needed, and
    skips `onRatioChange` when the new ratio equals the stored one (a stored out-of-range ratio still triggers a
    corrective call). Its drag test stubs a minimal `PointerEvent` (jsdom has none), in that test file only, so
    `user-event` behaviour elsewhere is unchanged.
14. Ruling: App's gallery route is a small `App` that returns the lazy gallery or `<Home />`, rather than an early
    return inside the existing component. Hooks cannot follow a conditional return, and `Home`'s IPC calls would
    otherwise fire (and reject) outside Tauri when the gallery is shown. `App.test.tsx` is unchanged and passes.
15. Ruling: shared helpers added beyond the plan's signatures, all additive: `controlSurface` and `popMotion(reduced)` in
    `cx.ts`; `pickFill({swatch, stage, tone})` in `tokens.ts`; `menuParts.tsx` (item classes and layout shared by `Menu`
    and `ContextMenu`, deliberately not exported from the barrel). `popMotion` is the single source of the open/close
    classes the Dialog tests pin, and Popover, Tooltip, Menu and Select reuse it.
16. Ruling: `Toaster` uses Sonner's `unstyled` mode with token classes and per-type `icons` (`text-ok`, `text-bad`, ...),
    rather than recolouring whole toasts by `classNames.success`. Cost if wrong: purely visual.
17. Ruling: `Switch` shows the checked state on the track (`bg-accent`, white thumb); the plan's "checked thumb
    bg-accent" is ambiguous. `KeyHint`'s spoken label for Enter is "Return". Cost if wrong: copy tweaks.
18. Ruling: `CLAUDE.md` said example pages live at `src/dev/primitives/<Name>.tsx`; the plan (and every file) uses
    `pages/<Name>Page.tsx`. I corrected that one bullet in `CLAUDE.md` and left "Current stage" alone, to limit merge
    conflicts with Plan 03.
19. Ruling: the registry test wraps pages in `UiProvider` (Tooltip needs its provider), gets a 30 s timeout (26 lazy
    pages, about 8 s locally), and mocks `EmptyStateLottie` (jsdom has no WebAssembly or `IntersectionObserver`).
20. Ruling: the first two tasks' red step was skipped for Task 1 (harness and implementation were written together) and
    for the Task 11 implementation (written before its tests were run). Every other task has a recorded
    failing run (`Failed to resolve import "./<Name>"`). All of it was green before commit.

## STUBs

None.

## Deviations from the plan (all additive or forced)

- `Panel` and `Segmented`/`Tabs` got a few extra tests (bar variant, footer, disabled option, ids). `EmptyState`'s art
  box is `aria-hidden` (decorative). `IconButton` omits `aria-label` from its props (it is `label`).
- Gallery toolbar (Task 17) is a `Segmented` Light/Dark plus a `Switch` Dense; both are covered by tests.
- `lintBoundary.test.ts` also lints every file in `src/components/ui/` and asserts no primitive imports
  `@/features`, `@/lib/bindings`, `@/lib/stores` or `../`.
- `PRIMITIVES` was built in task order and reordered into the spec §15.3 order in Task 17 (pinned by `index.test.ts`).
- The barrel `export *` lines are alphabetical by task, not spec order (allowed by the plan).

## Plan text that is wrong (not edited)

- Task 1 step 4: the `test` block belongs in `vitest.config.ts` (Ruling 3).
- Task 2: the plan's early return in `App` conflicts with the rules of hooks (Ruling 14); `PrimitiveName` is `never`
  while `PRIMITIVES` is empty, so `Object.entries` needs an explicit type argument and `Gallery` sorts on `String(name)`.
- Tasks 10 (tab-order and arrow-key tests, Rulings 7 and 8), 14 (`initials` rule, Ruling 6; `delayMs`, Ruling 10),
  15 (wasm path, Ruling 11) and 9 (button inside button, Ruling 9) contain tests or specs that cannot pass as written.
- Task 16: the plan's drag test needs a `PointerEvent`, which jsdom does not provide (Ruling 13).
- Task 17 step 2: `lintBoundary` needs no `overrideConfig` (Plan 01's config is not type-aware), and its hex probe has
  to be built from parts because `styles.test.ts` and the lint rule both reject hex strings.
- Proposed contract addition 1 (`--color-swatch-*` names, `--radius-*`, `--shadow-card`) disagrees with Plan 01's
  actual tokens (Ruling 1); the overview's row for Plan 02 still lists "the lint rule", which Plan 01 shipped.

## What the next plans need to know

- Import everything from `@/components/ui`: `cx`, `focusRing`, `Size/Tone/StageToken/Swatch/AgentStatus`, `STAGES`,
  `statusTone`, `statusLabel`, `toneClasses/stageClasses/swatchClasses`, `pickFill`, `UiProvider`, `toast`, and the 26
  families listed in `PRIMITIVES`. `Swatch` is the ten names from `@/lib/theme`.
- Mount `<UiProvider>` once at each window's root (Plan 06's `App.tsx`); it holds `TooltipProvider` and the Sonner
  `Toaster`. `toast` is the only toast API. In tests use `renderUi` from `@/test/ui`.
- `src/lib/motion.ts` exports `useReducedMotion`, `prefersReducedMotion`, `transitionFor(reduced, full?)`, `SPRING`,
  `EASE_REDUCED`, `GLOW_MS`, `REFLOW_MS`, `REDUCED_MOTION_QUERY`. `popMotion(reduced)` in `cx.ts` is the CSS-side twin.
- `KeyHint`/`formatChord` take `"cmd+shift+a"` chords; Plan 06's `keyboard.ts` should emit that form. Output order is
  ⌘ ⌃ ⌥ ⇧ then the key.
- Colour-by-prop is `status | swatch | stage | tone` with that precedence in `Chip` and `StatusDot`, and
  `swatch > stage > tone` in `Badge`. Map the generated `Stage`/`AgentState` to `StageToken`/`AgentStatus` at the
  feature boundary (suggested home `src/lib/format.ts`, per the plan's contract addition 2).
- `Resizable` is `direction: "horizontal" | "vertical"` plus `ratio` and `onRatioChange`, and clamps stored ratios for
  layout and ARIA (Plan 06 maps it from `LayoutNode::Split`). `EmptyState` takes an `animation` URL from
  `import art from "./x.lottie?url"`; the real art is Plans 06 and 07's.
- A primitive that needs a test with Radix pointer behaviour should expect jsdom to lack `PointerEvent`; `setup.ts`
  stubs `ResizeObserver`, `scrollIntoView` and pointer capture only.
- To add a primitive: see `.claude/rules/ui.md` (file, test, barrel, `PRIMITIVES`, page, `registry.ts`).
- Likely merge conflict with Plan 03: `ARCHITECTURE.md` (new "UI primitives" section before "Where to change things",
  one Contents line, one table row) and the `CLAUDE.md` primitive bullet. Keep both sides.

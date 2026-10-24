# Team Code 02 — UI Primitives Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `src/components/ui/`, the only place raw HTML controls appear: the twenty-six primitives of spec §15.3 as thin, token-styled, keyboard-complete wrappers over shadcn/ui and Radix, each with render tests and a dev-only example page.

**Architecture:** Every primitive is one `PascalCase.tsx` file under `src/components/ui/` that wraps a Radix primitive (or Vaul, Sonner, cmdk, dotLottie) with the semantic Tailwind tokens, a shared `cx` helper and a `className` passthrough, and exports exact prop types; nothing in the folder imports from `src/features/`, `src/lib/bindings.ts` or a store. A `tokens.ts` module holds the shared unions (`Size`, `Tone`, `StageToken`, `Swatch`, `AgentStatus`) and the literal class maps Tailwind needs to see, and `src/lib/motion.ts` holds the one reduced-motion hook every animated primitive reads. A barrel `index.ts` exports everything plus a `PRIMITIVES` record, and a dev-only gallery under `src/dev/primitives/` renders one example page per primitive; a test pins that the two agree.

**Tech Stack:** React 19, TypeScript 5 strict, Tailwind v4 (`@tailwindcss/vite`), `radix-ui` (unified package, as current shadcn/ui), `cmdk`, `vaul`, `sonner`, `motion`, `@lottiefiles/dotlottie-react`, `clsx` + `tailwind-merge` + `class-variance-authority`, `lucide-react`, `tw-animate-css`; Vitest + Testing Library (`@testing-library/react`, `user-event`, `jest-dom`) on jsdom.

**Spec:** `docs/superpowers/specs/2026-10-24-team-code-design.md` §15.3 (primitives), §11.1 (tokens), §11.3 (motion), §3 (UI kit), §10 (consumers). Contracts: `docs/superpowers/plans/2026-10-24-team-code-00-overview-and-contracts.md`.

## Global Constraints

- Tauri 2; Rust stable; Node 22; React 19; TypeScript 5.x strict; Vite; Tailwind v4 via `@tailwindcss/vite`, no `tailwind.config` (§3, LiveSpec convention).
- Three layers: `crates/teamcode-core` never depends on Tauri; `src-tauri` stays thin; the webview has no state library and uses module stores read with `useSyncExternalStore` (§3, §14).
- Raw `<input>`, `<select>`, `<textarea>`, `<button>`, `<dialog>` appear only under `src/components/ui/`; hex colours and inline colour styles appear only in the theme file. ESLint fails the build otherwise (§15.3).
- Semantic Tailwind tokens only: `bg-page`, `bg-bar`, `bg-well`, `bg-card`, `border-line`, `text-ink`, `text-mute`, `text-accent`, `text-ok`, `text-warn`, `text-bad`, plus `stage-spec|plan|build|review|test|commit|done` (§11.1).
- Fonts bundled from Fontsource, never fetched: Inter, JetBrains Mono, Literata. CSP `default-src 'self'` (§3, §14).
- Nothing a user triggers takes longer than 250 ms to respond; `prefers-reduced-motion` turns springs into 150 ms eases and stops loops (§11.3).
- File size budgets: component under about 200 lines, store under 300, Rust module under 500 (§15.1).
- Naming: components `PascalCase.tsx`, lib and stores `camelCase.ts`, Rust `snake_case`; IPC commands `domain_verb` in snake_case; events `kebab-case` (§15.1, this document).
- Commits follow Conventional Commits (§15.4). Every task ends with the quality gates in §15.4 passing for the code it touched.

Plan-specific rules, applied to every task below:

- A primitive has **no business logic**: it never imports `src/lib/bindings.ts`, a store, or anything under `src/features/`. It takes plain unions (`AgentStatus`, `StageToken`), never `AgentState` or `Stage` from the bindings.
- React 19: `ref` is an ordinary prop carried by `ComponentProps<'x'>`; do not use `forwardRef`. Return type is `ReactElement` from `react` (the global `JSX` namespace is gone).
- Radix is imported from the unified package: `import { Dialog as DialogPrimitive } from 'radix-ui'`.
- Every component accepts `className?: string` merged last with `cx`, and spreads the rest of its props onto the root element.
- Keyboard and focus behaviour is Radix's; a primitive never adds a key handler Radix already provides, and never removes focus-visible styling (`focusRing` from `cx.ts` on every focusable element).
- Tailwind only sees literal class strings, so colour-by-prop goes through the maps in `tokens.ts`, never through template strings.

## Review Focus

1. **Long labels** — a *Replied* chip carries the first line of an agent message, which can be hundreds of characters; `Chip` and `Badge` must truncate with an ellipsis and expose the full text in `title`, never wrap and break a two-line card. (Test added to Task 14.)
2. **Out-of-range persisted ratio** — a view layout restored from SQLite may carry a `ratio` outside `[min, 1 − min]` (min changed, or a corrupt row); `Resizable` must clamp for layout and ARIA and step from the clamped value, not from the stored one. (Test added to Task 16.)
3. **Nested dialogs** — a confirm sheet opens over a settings dialog (archive while working, §6.7); Escape must close only the inner one and focus must return to the inner trigger. (Test added to Task 12.)
4. **Value not among options** — a `Combobox` bound to a project list whose project was removed receives a `value` with no matching option; it must show the placeholder, not a blank trigger or a crash. (Test added to Task 9.)
5. **Tooltip on a disabled control** — disabled buttons fire no pointer or focus events, so the hint explaining *why* a control is disabled (greyed worktree toggle, §9) would never show; `Tooltip` must wrap a disabled child in a focusable span. (Test added to Task 7.)

---

## File map

```
src/components/ui/
  cx.ts tokens.ts primitives.css index.ts
  Button.tsx IconButton.tsx KeyHint.tsx Badge.tsx StatusDot.tsx
  Field.tsx Input.tsx Textarea.tsx Checkbox.tsx Switch.tsx
  Popover.tsx Tooltip.tsx UiProvider.tsx Select.tsx Combobox.tsx
  Segmented.tsx Tabs.tsx Menu.tsx ContextMenu.tsx Dialog.tsx Drawer.tsx Toast.tsx
  Chip.tsx Avatar.tsx EmptyState.tsx EmptyStateLottie.tsx Panel.tsx Resizable.tsx
  <Name>.test.tsx beside each, lintBoundary.test.ts
src/lib/motion.ts (+ motion.test.ts)
src/test/setup.ts src/test/ui.ts
src/dev/primitives/index.tsx Gallery.tsx registry.ts registry.test.tsx pages/<Name>Page.tsx
docs/adr/2026-10-24-ui-primitive-dependencies.md
```

Each primitive task follows the same shape: write the test file, run it (fails: module missing), implement to the stated signature, run it (passes), add the example page and the `PRIMITIVES` entry, run the registry test, commit. Test commands are `npx vitest run <file>`; a first run fails with `Error: Failed to resolve import "./<Name>"`, a passing run prints `Test Files  1 passed (1)`.

---

### Task 1: Toolkit — dependencies, `cx`, tokens, `lib/motion.ts`, test harness

**Files:**
- Modify: `package.json` (dependencies), `vite.config.ts` (`test` block), `src/styles.css` (two `@import` lines)
- Create: `src/components/ui/cx.ts`, `src/components/ui/cx.test.ts`, `src/components/ui/tokens.ts`, `src/components/ui/tokens.test.ts`, `src/components/ui/primitives.css`, `src/lib/motion.ts`, `src/lib/motion.test.ts`, `src/test/setup.ts`, `src/test/ui.ts`, `docs/adr/2026-10-24-ui-primitive-dependencies.md`

**Interfaces:**
- Consumes: Plan 01's `src/styles.css`, which defines the §11.1 tokens as Tailwind v4 `@theme` colours (`--color-page … --color-bad`, `--color-stage-spec … --color-stage-done`) switched by `data-theme` on `:root`, so every token works with any colour utility prefix and opacity modifier (`bg-accent/15`, `ring-accent`, `text-stage-plan`); the Vitest setup from Plan 01.
- Produces:
  - `cx(...inputs: ClassValue[]): string` (clsx + twMerge); `focusRing: string`; `controlHeight: Record<Size, string>` (`sm: 'h-6'`, `md: 'h-7'`, `lg: 'h-8'`); `controlText: Record<Size, string>` (`sm: 'text-xs'`, `md: 'text-sm'`, `lg: 'text-sm'`).
  - `tokens.ts`: `type Size = 'sm' | 'md' | 'lg'`; `type Tone = 'neutral' | 'accent' | 'ok' | 'warn' | 'bad'`; `type StageToken = 'spec' | 'plan' | 'build' | 'review' | 'test' | 'commit' | 'done'`; `type Swatch = 'violet' | 'indigo' | 'cyan' | 'mint' | 'lime' | 'gold' | 'coral' | 'magenta'`; `type AgentStatus = 'working' | 'needs-you' | 'idle'`; `const STAGES: readonly StageToken[]`; `type FillClasses = { soft: string; solid: string; text: string }`; `toneClasses: Record<Tone, FillClasses>`; `stageClasses: Record<StageToken, FillClasses>`; `swatchClasses: Record<Swatch, FillClasses>`; `statusTone(status: AgentStatus): Tone`; `statusLabel(status: AgentStatus): string`.
  - `src/lib/motion.ts`: `REDUCED_MOTION_QUERY = '(prefers-reduced-motion: reduce)'`; `prefersReducedMotion(): boolean`; `useReducedMotion(): boolean`; `SPRING: Transition` (`{ type: 'spring', stiffness: 420, damping: 32 }`); `EASE_REDUCED: Transition` (`{ duration: 0.15, ease: 'easeOut' }`); `transitionFor(reduced: boolean, full?: Transition): Transition`; `GLOW_MS = 400`; `REFLOW_MS = 300`.
  - `src/test/ui.ts`: `mockMatchMedia(reduced: boolean): { set(reduced: boolean): void }`; `setupUser(): UserEvent` (`userEvent.setup({ pointerEventsCheck: 0 })`).

- [ ] **Step 1: Install dependencies**

Run: `npm i radix-ui cmdk sonner vaul motion @lottiefiles/dotlottie-react clsx tailwind-merge class-variance-authority lucide-react tw-animate-css && npm i -D @testing-library/react @testing-library/user-event @testing-library/jest-dom jsdom`
Expected: `package.json` lists each; `npm ls radix-ui` prints one version.

- [ ] **Step 2: Write the failing tests**

`src/components/ui/cx.test.ts`:
```ts
test('cx merges conflicting Tailwind classes, last wins', () => {
  expect(cx('p-2', false && 'hidden', 'p-3')).toBe('p-3');
});
```
`src/components/ui/tokens.test.ts`:
```ts
test('STAGES lists the seven stages in pipeline order', () => {
  expect(STAGES).toEqual(['spec', 'plan', 'build', 'review', 'test', 'commit', 'done']);
});
test('statusTone maps working, needs-you and idle to ok, warn and neutral', () => {
  expect(statusTone('working')).toBe('ok'); expect(statusTone('needs-you')).toBe('warn'); expect(statusTone('idle')).toBe('neutral');
});
test('statusLabel gives the chip copy', () => {
  expect(statusLabel('working')).toBe('Working'); expect(statusLabel('needs-you')).toBe('Needs you'); expect(statusLabel('idle')).toBe('Idle');
});
test('every tone, stage and swatch has soft, solid and text classes with no hex colour', () => {
  for (const map of [toneClasses, stageClasses, swatchClasses]) for (const fill of Object.values(map)) {
    for (const key of ['soft', 'solid', 'text'] as const) { expect(fill[key]).toBeTruthy(); expect(fill[key]).not.toMatch(/#[0-9a-fA-F]{3,8}/); }
  }
  expect(Object.keys(stageClasses)).toEqual([...STAGES]);
});
```
`src/lib/motion.test.ts`:
```ts
test('prefersReducedMotion is false when matchMedia is absent', () => {
  const saved = window.matchMedia; Object.defineProperty(window, 'matchMedia', { value: undefined, configurable: true });
  expect(prefersReducedMotion()).toBe(false); Object.defineProperty(window, 'matchMedia', { value: saved, configurable: true });
});
test('useReducedMotion reads the media query', () => {
  mockMatchMedia(true); expect(renderHook(() => useReducedMotion()).result.current).toBe(true);
});
test('useReducedMotion flips when the media query changes', () => {
  const media = mockMatchMedia(false); const { result } = renderHook(() => useReducedMotion());
  expect(result.current).toBe(false); act(() => media.set(true)); expect(result.current).toBe(true);
});
test('transitionFor returns the 150 ms ease under reduced motion and the spring otherwise', () => {
  expect(transitionFor(true)).toEqual({ duration: 0.15, ease: 'easeOut' }); expect(transitionFor(false)).toBe(SPRING);
});
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `npx vitest run src/components/ui src/lib/motion.test.ts`
Expected: FAIL — `Failed to resolve import "./cx"` / `"./tokens"` / `"./motion"` / `"../test/ui"`.

- [ ] **Step 4: Create the test harness**

`src/test/setup.ts`: `import '@testing-library/jest-dom/vitest'`; `afterEach(cleanup)`; fill jsdom gaps Radix, cmdk and Vaul touch — `window.ResizeObserver` (observe/unobserve/disconnect no-ops), `Element.prototype.scrollIntoView`, `hasPointerCapture` (returns false), `setPointerCapture`, `releasePointerCapture`, `window.DOMRect.fromRect` if missing; call `mockMatchMedia(false)` so every test starts with full motion and a light colour scheme.
`src/test/ui.ts`: `mockMatchMedia(reduced)` installs (with `Object.defineProperty`, `configurable: true`) a `window.matchMedia` stub whose `matches` is `reduced` for `REDUCED_MOTION_QUERY` and `false` for every other query, records `change` listeners per query, and returns `{ set }` which updates the state and calls the recorded listeners with `{ matches }`. `setupUser()` as above.
`vite.config.ts` `test` block must contain `environment: 'jsdom'`, `setupFiles: ['src/test/setup.ts']`, `include: ['src/**/*.test.{ts,tsx}']`, `css: false` (add the missing keys; keep Plan 01's others).

- [ ] **Step 5: Implement `cx.ts`, `tokens.ts`, `motion.ts`, `primitives.css`**

`cx.ts` as in Interfaces; `focusRing = 'outline-none focus-visible:ring-2 focus-visible:ring-accent/60 focus-visible:ring-offset-1 focus-visible:ring-offset-page'`.
`tokens.ts`: `toneClasses.neutral = { soft: 'bg-well text-mute', solid: 'bg-mute text-accent-ink', text: 'text-mute' }`, `accent = { soft: 'bg-accent/15 text-accent', solid: 'bg-accent text-accent-ink', text: 'text-accent' }`, `ok`/`warn`/`bad` the same pattern on `ok`/`warn`/`bad`; `stageClasses.spec = { soft: 'bg-stage-spec/15 text-stage-spec', solid: 'bg-stage-spec text-accent-ink', text: 'text-stage-spec' }` and so on for the seven; `swatchClasses` likewise on `swatch-<name>`. Write each entry literally (Tailwind scans source text).
`motion.ts`: `useReducedMotion` is `useSyncExternalStore(subscribe, prefersReducedMotion, () => false)` where `subscribe` adds a `change` listener on `matchMedia(REDUCED_MOTION_QUERY)`; `transitionFor(reduced, full = SPRING)` returns `EASE_REDUCED` when reduced, else `full`.
`primitives.css` (no colours; imported by `styles.css`):
```css
@utility animate-status-pulse { animation: tc-status-pulse 1.6s ease-out infinite; }
@keyframes tc-status-pulse { 0%, 100% { box-shadow: 0 0 0 0 color-mix(in oklab, currentColor 45%, transparent); } 70% { box-shadow: 0 0 0 5px transparent; } }
@media (prefers-reduced-motion: reduce) { .animate-status-pulse { animation: none; } }
[data-motion="reduced"][data-vaul-drawer], [data-motion="reduced"][data-vaul-overlay] { transition-duration: 150ms !important; transition-timing-function: ease-out !important; }
```
Add to `src/styles.css`, after the Tailwind import: `@import "tw-animate-css";` and `@import "./components/ui/primitives.css";`.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `npx vitest run src/components/ui src/lib/motion.test.ts`
Expected: `Test Files  3 passed (3)`, `Tests  9 passed (9)`.

- [ ] **Step 7: Write the ADR**

`docs/adr/2026-10-24-ui-primitive-dependencies.md` (Context / Decision / Consequences, under a page): the spec names shadcn/ui on Radix, Motion, Sonner, Vaul and dotLottie (§3); this plan adds the packages shadcn's generated code depends on (`radix-ui` unified, `clsx`, `tailwind-merge`, `class-variance-authority`, `lucide-react`, `tw-animate-css`, `cmdk` for Combobox) and Testing Library for render tests. Record that dotLottie's renderer is WebAssembly bundled through Vite `?url`, which needs `script-src 'self' 'wasm-unsafe-eval'` in the Tauri CSP (see Proposed contract additions).

- [ ] **Step 8: Commit**

```bash
git add package.json package-lock.json vite.config.ts src/styles.css src/components/ui src/lib/motion.ts src/lib/motion.test.ts src/test docs/adr
git commit -m "chore(ui): add primitive toolkit, tokens, motion hook and test harness"
```

---

### Task 2: Dev gallery shell, barrel skeleton and coverage test

**Files:**
- Create: `src/components/ui/index.ts`, `src/dev/primitives/index.tsx`, `src/dev/primitives/Gallery.tsx`, `src/dev/primitives/registry.ts`, `src/dev/primitives/registry.test.tsx`
- Modify: `src/App.tsx` (dev-only early return)

**Interfaces:**
- Consumes: `cx`, `tokens` (Task 1).
- Produces: barrel `src/components/ui/index.ts` re-exporting `cx`, `focusRing`, everything in `tokens.ts`, and `export const PRIMITIVES = { … } as const satisfies Record<string, readonly string[]>` (family name → exported member names; starts empty) with `export type PrimitiveName = keyof typeof PRIMITIVES`. `registry.ts`: `type ExamplePage = { name: PrimitiveName; blurb: string; Page: LazyExoticComponent<ComponentType> }` and `export const pages: ExamplePage[]`. `Gallery.tsx`: `Gallery(): ReactElement` — hash router on `#/dev/primitives/<PrimitiveName>` read with `useSyncExternalStore` on `hashchange`, a `<nav>` of `<a href>` links, main area renders the page inside `<Suspense>`; the index page heading is the literal `Primitives gallery`. `index.tsx`: `export default function PrimitivesGallery()`. `App.tsx` gains `isDevPrimitivesRoute(): boolean` (`import.meta.env.DEV && location.hash.startsWith('#/dev/primitives')`).

- [ ] **Step 1: Write the failing test**

`src/dev/primitives/registry.test.tsx`:
```tsx
import * as ui from '../../components/ui';
import { PRIMITIVES } from '../../components/ui';
import { pages } from './registry';
test('every name in PRIMITIVES has exactly one example page', () => {
  expect(pages.map((p) => p.name).sort()).toEqual(Object.keys(PRIMITIVES).sort());
});
test('every member PRIMITIVES lists is exported from the barrel', () => {
  for (const [name, members] of Object.entries(PRIMITIVES)) for (const m of members) expect(ui, `${name} → ${m}`).toHaveProperty(m);
});
test('each example page renders an h1 named after its primitive', async () => {
  for (const { name, Page } of pages) {
    render(<Suspense fallback={null}><Page /></Suspense>);
    expect(await screen.findByRole('heading', { level: 1, name })).toBeInTheDocument();
    cleanup();
  }
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npx vitest run src/dev/primitives/registry.test.tsx`
Expected: FAIL — `Failed to resolve import "../../components/ui"`.

- [ ] **Step 3: Create the barrel, registry, gallery and App hook**

Barrel and registry as in Interfaces. `Gallery` left column lists `pages` sorted by name with the blurb under each link; right column renders the matched page or the index (heading `Primitives gallery`, one line: "Dev builds only. One page per primitive; add yours to registry.ts."). In `App.tsx`, before any other render: `const PrimitivesGallery = import.meta.env.DEV ? lazy(() => import('./dev/primitives')) : null;` and `if (PrimitivesGallery && isDevPrimitivesRoute()) return <Suspense fallback={null}><PrimitivesGallery /></Suspense>;` — the dead branch lets Rollup drop the chunk from production.

- [ ] **Step 4: Run the test to verify it passes**

Run: `npx vitest run src/dev/primitives/registry.test.tsx`
Expected: `Tests  3 passed (3)`.

- [ ] **Step 5: Verify the gallery is absent from a production build**

Run: `npm run build && (grep -rl "Primitives gallery" dist/ && echo "LEAKED" || echo "not in bundle")`
Expected: last line `not in bundle`. Then `npm run dev` and open `http://localhost:1420/#/dev/primitives`: the heading `Primitives gallery` shows.

- [ ] **Step 6: Commit**

```bash
git add src/components/ui/index.ts src/dev/primitives src/App.tsx
git commit -m "feat(dev): add primitives gallery shell and coverage test"
```

---

### Task 3: `Button` and `IconButton`

**Files:**
- Create: `src/components/ui/Button.tsx`, `Button.test.tsx`, `IconButton.tsx`, `IconButton.test.tsx`, `src/dev/primitives/pages/ButtonPage.tsx`, `pages/IconButtonPage.tsx`
- Modify: `src/components/ui/index.ts`, `src/dev/primitives/registry.ts`

**Interfaces:**
- Consumes: `cx`, `focusRing`, `controlHeight`, `controlText`, `Size`; `Slot` from `radix-ui`; `cva`.
- Produces:
```ts
export type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger' | 'link';
export type ButtonProps = ComponentProps<'button'> & {
  variant?: ButtonVariant;   // default 'secondary'
  size?: Size;               // default 'md'
  loading?: boolean;         // aria-busy, disabled, spinner replaces `leading`
  leading?: ReactNode; trailing?: ReactNode;
  asChild?: boolean;         // Radix Slot: child element receives the classes
};
export function Button(props: ButtonProps): ReactElement;      // renders type="button" unless given
export const buttonVariants: (opts?: { variant?: ButtonVariant; size?: Size }) => string;
export type IconButtonProps = Omit<ButtonProps, 'leading' | 'trailing' | 'children' | 'variant'> & {
  icon: ReactNode; label: string;                               // label → aria-label (required)
  variant?: Exclude<ButtonVariant, 'link'>;                     // default 'ghost'
  active?: boolean;                                             // aria-pressed, for toggles (sidebar icon strip)
};
export function IconButton(props: IconButtonProps): ReactElement; // square: size-6 / size-7 / size-8
```
Variants: `primary` `bg-accent text-accent-ink hover:bg-accent/90`; `secondary` `bg-card text-ink border border-line hover:bg-well`; `ghost` `text-ink hover:bg-well`; `danger` `text-bad border border-line hover:bg-bad/10`; `link` `text-accent underline-offset-2 hover:underline h-auto px-0`. Radius `rounded-card`. Disabled `opacity-50 pointer-events-none`.

- [ ] **Step 1: Write the failing tests**

`Button.test.tsx`:
```tsx
test('renders a type="button" element with its children', () => { render(<Button>Save</Button>); expect(screen.getByRole('button', { name: 'Save' })).toHaveAttribute('type', 'button'); });
test('primary variant uses the accent fill', () => { render(<Button variant="primary">Go</Button>); expect(screen.getByRole('button')).toHaveClass('bg-accent'); });
test('loading sets aria-busy, disables and swallows clicks', async () => {
  const onClick = vi.fn(); render(<Button loading onClick={onClick}>Run</Button>);
  const b = screen.getByRole('button'); expect(b).toHaveAttribute('aria-busy', 'true'); expect(b).toBeDisabled();
  await setupUser().click(b); expect(onClick).not.toHaveBeenCalled();
});
test('asChild renders the child element with button classes', () => {
  render(<Button asChild><a href="#x">Open</a></Button>); expect(screen.getByRole('link', { name: 'Open' })).toHaveClass('inline-flex');
});
test('merges className last', () => { render(<Button className="mt-2">x</Button>); expect(screen.getByRole('button')).toHaveClass('mt-2'); });
test('ref reaches the button element', () => { const ref = createRef<HTMLButtonElement>(); render(<Button ref={ref}>x</Button>); expect(ref.current).toBeInstanceOf(HTMLButtonElement); });
```
`IconButton.test.tsx`:
```tsx
test('exposes label as the accessible name', () => { render(<IconButton icon={<svg />} label="Archive" />); expect(screen.getByRole('button', { name: 'Archive' })).toBeInTheDocument(); });
test('active sets aria-pressed', () => { render(<IconButton icon={<svg />} label="Tasks" active />); expect(screen.getByRole('button')).toHaveAttribute('aria-pressed', 'true'); });
test('is square at md', () => { render(<IconButton icon={<svg />} label="x" />); expect(screen.getByRole('button')).toHaveClass('size-7'); });
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `npx vitest run src/components/ui/Button.test.tsx src/components/ui/IconButton.test.tsx`
Expected: FAIL — `Failed to resolve import "./Button"`.

- [ ] **Step 3: Implement `Button.tsx` and `IconButton.tsx`**

`Button` builds classes with `cva` (`buttonVariants`), renders `Slot` when `asChild`, otherwise `<button type="button">`; `loading` renders a `Loader2` from `lucide-react` with `animate-spin motion-reduce:animate-none` in place of `leading`. `IconButton` composes `Button` with `aria-label={label}`, `aria-pressed={active}` (omitted when `active` is undefined), square sizes.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `npx vitest run src/components/ui/Button.test.tsx src/components/ui/IconButton.test.tsx`
Expected: `Tests  9 passed (9)`.

- [ ] **Step 5: Example pages and registry**

`ButtonPage.tsx` (`<h1>Button</h1>`, every variant × size, loading, disabled, leading/trailing icon, `asChild` link) and `IconButtonPage.tsx` (`<h1>IconButton</h1>`, variants × sizes, active). Register both in `registry.ts` with `lazy(() => import('./pages/ButtonPage'))`; add `Button: ['Button', 'buttonVariants'], IconButton: ['IconButton']` to `PRIMITIVES` and `export * from './Button'; export * from './IconButton';` to the barrel.

Run: `npx vitest run src/dev/primitives/registry.test.tsx` — Expected: `Tests  3 passed (3)`.

- [ ] **Step 6: Commit**

```bash
git add src/components/ui src/dev/primitives
git commit -m "feat(ui): add Button and IconButton primitives"
```

---

### Task 4: `KeyHint`, `Badge` and `StatusDot`

**Files:**
- Create: `src/components/ui/KeyHint.tsx`, `KeyHint.test.tsx`, `Badge.tsx`, `Badge.test.tsx`, `StatusDot.tsx`, `StatusDot.test.tsx`, `src/dev/primitives/pages/KeyHintPage.tsx`, `pages/BadgePage.tsx`, `pages/StatusDotPage.tsx`
- Modify: `src/components/ui/index.ts`, `src/dev/primitives/registry.ts`

**Interfaces:**
- Consumes: `tokens.ts` maps, `useReducedMotion`. Wraps: plain `<kbd>` / `<span>` elements, no Radix primitive.
- Produces:
```ts
// KeyHint.tsx — chord syntax: tokens joined by '+'. Modifiers: cmd|mod|meta → ⌘, ctrl|control → ⌃, alt|opt|option → ⌥, shift → ⇧.
// Keys: enter|return → ⏎, esc|escape → ⎋, tab → ⇥, space → ␣, backspace → ⌫, delete → ⌦, up|down|left|right → ↑↓←→,
// anything else → uppercased as written. Output order is ⌘ ⌃ ⌥ ⇧ then the key (the spec's copy puts ⌘ first; keep it).
export function formatChord(chord: string): string;          // 'cmd+shift+a' → '⌘⇧A'
export function chordLabel(chord: string): string;           // 'cmd+shift+a' → 'Command Shift A'
export type KeyHintProps = ComponentProps<'kbd'> & { keys: string; size?: 'sm' | 'md' };
export function KeyHint(props: KeyHintProps): ReactElement;  // <kbd aria-label={chordLabel}>{formatChord}</kbd>, font-mono text-mute

// Badge.tsx
export type BadgeProps = ComponentProps<'span'> & {
  tone?: Tone; stage?: StageToken; swatch?: Swatch;          // precedence: swatch > stage > tone; default tone 'neutral'
  variant?: 'soft' | 'solid' | 'outline';                    // default 'soft'
  size?: 'sm' | 'md'; mono?: boolean; icon?: ReactNode; dot?: boolean;
};
export function Badge(props: BadgeProps): ReactElement;      // truncates: 'truncate max-w-full', title = string children

// StatusDot.tsx
export type StatusDotProps = ComponentProps<'span'> & {
  status?: AgentStatus; stage?: StageToken; tone?: Tone; swatch?: Swatch;   // precedence: status > swatch > stage > tone
  size?: 'sm' | 'md'; label?: string; pulse?: boolean;      // pulse defaults to status === 'working'; never true under reduced motion
};
export function StatusDot(props: StatusDotProps): ReactElement;             // role="img" aria-label={label ?? statusLabel(status)}; data-pulse="true|false"
```
Colour is carried by a `text-*` class and the dot is `bg-current`, so the pulse ring can use `currentColor`.

- [ ] **Step 1: Write the failing tests**

`KeyHint.test.tsx`:
```tsx
test.each([
  ['cmd+shift+a', '⌘⇧A'], ['shift+cmd+a', '⌘⇧A'], ['cmd+enter', '⌘⏎'], ['ctrl+`', '⌃`'],
  ['cmd+alt+left', '⌘⌥←'], ['cmd+shift+[', '⌘⇧['], ['cmd+,', '⌘,'], ['cmd+1', '⌘1'], ['esc', '⎋'],
])('formatChord(%s) → %s', (chord, glyphs) => expect(formatChord(chord)).toBe(glyphs));
test('chordLabel spells the chord out', () => expect(chordLabel('cmd+shift+a')).toBe('Command Shift A'));
test('renders a kbd with the glyphs and a spoken label', () => {
  render(<KeyHint keys="cmd+k" />); const kbd = screen.getByText('⌘K'); expect(kbd.tagName).toBe('KBD'); expect(kbd).toHaveAttribute('aria-label', 'Command K');
});
```
`Badge.test.tsx`:
```tsx
test('defaults to the neutral soft fill', () => { render(<Badge>3</Badge>); expect(screen.getByText('3')).toHaveClass('bg-well', 'text-mute'); });
test('stage badge uses the stage token', () => { render(<Badge stage="plan">Plan</Badge>); expect(screen.getByText('Plan')).toHaveClass('bg-stage-plan/15', 'text-stage-plan'); });
test('solid swatch badge', () => { render(<Badge swatch="coral" variant="solid">api</Badge>); expect(screen.getByText('api')).toHaveClass('bg-swatch-coral', 'text-accent-ink'); });
test('mono uses the monospace font and keeps the text as title', () => { render(<Badge mono>TC-42</Badge>); const b = screen.getByText('TC-42'); expect(b).toHaveClass('font-mono', 'truncate'); expect(b).toHaveAttribute('title', 'TC-42'); });
```
`StatusDot.test.tsx`:
```tsx
test('working is green and pulses', () => { render(<StatusDot status="working" />); const d = screen.getByRole('img', { name: 'Working' }); expect(d).toHaveClass('text-ok', 'animate-status-pulse'); expect(d).toHaveAttribute('data-pulse', 'true'); });
test('needs-you is amber and still', () => { render(<StatusDot status="needs-you" />); const d = screen.getByRole('img', { name: 'Needs you' }); expect(d).toHaveClass('text-warn'); expect(d).not.toHaveClass('animate-status-pulse'); });
test('idle is grey', () => { render(<StatusDot status="idle" />); expect(screen.getByRole('img', { name: 'Idle' })).toHaveClass('text-mute'); });
test('stage colours the dot with the stage token', () => { render(<StatusDot stage="review" label="Review" />); expect(screen.getByRole('img', { name: 'Review' })).toHaveClass('text-stage-review'); });
test('pulse stops under reduced motion', () => {
  mockMatchMedia(true); render(<StatusDot status="working" />);
  const d = screen.getByRole('img'); expect(d).toHaveAttribute('data-pulse', 'false'); expect(d).not.toHaveClass('animate-status-pulse');
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `npx vitest run src/components/ui/KeyHint.test.tsx src/components/ui/Badge.test.tsx src/components/ui/StatusDot.test.tsx`
Expected: FAIL — `Failed to resolve import "./KeyHint"`.

- [ ] **Step 3: Implement the three files**

`formatChord` lowercases, splits on `+`, sorts modifiers into ⌘ ⌃ ⌥ ⇧, maps the key by the table, unknown single tokens uppercased. `Badge` picks `swatchClasses[swatch] ?? stageClasses[stage] ?? toneClasses[tone]` then `[variant]` (`outline` = `text` class + `border border-current/30`). `StatusDot` computes `pulse = (props.pulse ?? status === 'working') && !useReducedMotion()`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: same command — Expected: `Test Files  3 passed (3)`, `Tests  20 passed (20)`.

- [ ] **Step 5: Example pages and registry**

`KeyHintPage` (`<h1>KeyHint</h1>`, the whole §10.5 keyboard map rendered as a table of `KeyHint`s), `BadgePage` (`<h1>Badge</h1>`, tones × variants, all seven stages, all eight swatches, mono Jira key, a 120-character label to show truncation), `StatusDotPage` (`<h1>StatusDot</h1>`, three states × two sizes, seven stages, a note that the pulse stops under reduced motion). `PRIMITIVES` gains `KeyHint: ['KeyHint', 'formatChord', 'chordLabel'], Badge: ['Badge'], StatusDot: ['StatusDot']`.

Run: `npx vitest run src/dev/primitives/registry.test.tsx` — Expected: `Tests  3 passed (3)`.

- [ ] **Step 6: Commit**

```bash
git add src/components/ui src/dev/primitives
git commit -m "feat(ui): add KeyHint, Badge and StatusDot primitives"
```

---

### Task 5: `Field`, `Input` and `Textarea`

**Files:**
- Create: `src/components/ui/Field.tsx`, `Field.test.tsx`, `Input.tsx`, `Input.test.tsx`, `Textarea.tsx`, `Textarea.test.tsx`, `src/dev/primitives/pages/FieldPage.tsx`, `pages/InputPage.tsx`, `pages/TextareaPage.tsx`
- Modify: `src/components/ui/index.ts`, `src/dev/primitives/registry.ts`

**Interfaces:**
- Consumes: `cx`, `focusRing`, `controlHeight`, `controlText`. Wraps: the raw `<input>`, `<textarea>` and `<label>` elements (this folder is the only place they appear), no Radix primitive.
- Produces:
```ts
// Field.tsx — label, control, hint, error. Provides ids to the control through context.
export type FieldProps = ComponentProps<'div'> & {
  label?: ReactNode; hint?: ReactNode; error?: ReactNode; required?: boolean;
  layout?: 'stack' | 'row';                       // default 'stack'; 'row' = label left, control right (settings panes)
  id?: string;                                    // base id, default useId()
};
export function Field(props: FieldProps): ReactElement;   // label htmlFor={id}; hint id `${id}-hint`; error id `${id}-error` with role="alert" text-bad
export type FieldControlAttrs = { id: string; 'aria-describedby'?: string; 'aria-invalid'?: true; 'aria-required'?: true };
export function useFieldControl(): FieldControlAttrs | undefined;   // undefined outside a Field; controls spread it under their own props

// Input.tsx
export type InputProps = Omit<ComponentProps<'input'>, 'size'> & {
  size?: 'sm' | 'md'; invalid?: boolean; leading?: ReactNode; trailing?: ReactNode; mono?: boolean;
};
export function Input(props: InputProps): ReactElement;   // wrapper span when leading/trailing; aria-invalid from invalid || field error

// Textarea.tsx
export type TextareaProps = ComponentProps<'textarea'> & {
  size?: 'sm' | 'md'; invalid?: boolean; mono?: boolean;
  autoGrow?: boolean; maxRows?: number;           // autoGrow: rows = clamp(lineCount(value), rows ?? 2, maxRows ?? 8)
};
export function Textarea(props: TextareaProps): ReactElement;
```
Styling: `bg-card border border-line text-ink placeholder:text-mute rounded-card px-2`, `aria-invalid:border-bad`, `focusRing`.

- [ ] **Step 1: Write the failing tests**

`Field.test.tsx`:
```tsx
test('links the label to the control inside it', () => { render(<Field label="Title"><Input /></Field>); expect(screen.getByLabelText('Title')).toBeInstanceOf(HTMLInputElement); });
test('describes the control with hint and error', () => {
  render(<Field label="Title" hint="Keep it short" error="Required"><Input /></Field>);
  expect(screen.getByRole('textbox')).toHaveAccessibleDescription('Keep it short Required');
});
test('marks the control invalid and announces the error', () => {
  render(<Field label="Title" error="Required"><Input /></Field>);
  expect(screen.getByRole('textbox')).toHaveAttribute('aria-invalid', 'true'); expect(screen.getByRole('alert')).toHaveTextContent('Required');
});
test('required adds aria-required and a visual marker', () => {
  render(<Field label="Title" required><Input /></Field>); expect(screen.getByRole('textbox')).toHaveAttribute('aria-required', 'true'); expect(screen.getByText('*')).toHaveAttribute('aria-hidden', 'true');
});
test('row layout places label and control side by side', () => { const { container } = render(<Field label="Theme" layout="row"><Input /></Field>); expect(container.firstChild).toHaveClass('grid-cols-[minmax(8rem,1fr)_2fr]'); });
```
`Input.test.tsx`:
```tsx
test('forwards value and onChange', async () => { const onChange = vi.fn(); render(<Input value="" onChange={onChange} />); await setupUser().type(screen.getByRole('textbox'), 'a'); expect(onChange).toHaveBeenCalledTimes(1); });
test('renders leading and trailing adornments outside the input', () => { render(<Input leading={<span>L</span>} trailing={<span>T</span>} />); expect(screen.getByText('L').parentElement).toContainElement(screen.getByRole('textbox')); expect(screen.getByText('T')).toBeInTheDocument(); });
test('invalid sets aria-invalid', () => { render(<Input invalid />); expect(screen.getByRole('textbox')).toHaveAttribute('aria-invalid', 'true'); });
test('mono uses the monospace font', () => { render(<Input mono />); expect(screen.getByRole('textbox')).toHaveClass('font-mono'); });
test('ref reaches the input', () => { const ref = createRef<HTMLInputElement>(); render(<Input ref={ref} />); expect(ref.current).toBeInstanceOf(HTMLInputElement); });
```
`Textarea.test.tsx`:
```tsx
test('autoGrow sets rows from the line count', () => { render(<Textarea autoGrow value={'a\nb\nc'} readOnly />); expect(screen.getByRole('textbox')).toHaveAttribute('rows', '3'); });
test('autoGrow never exceeds maxRows nor drops below rows', () => {
  const { rerender } = render(<Textarea autoGrow maxRows={2} value={'a\nb\nc'} readOnly />); expect(screen.getByRole('textbox')).toHaveAttribute('rows', '2');
  rerender(<Textarea autoGrow rows={4} value="a" readOnly />); expect(screen.getByRole('textbox')).toHaveAttribute('rows', '4');
});
test('picks up Field ids', () => { render(<Field label="Intent"><Textarea /></Field>); expect(screen.getByLabelText('Intent').tagName).toBe('TEXTAREA'); });
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `npx vitest run src/components/ui/Field.test.tsx src/components/ui/Input.test.tsx src/components/ui/Textarea.test.tsx`
Expected: FAIL — `Failed to resolve import "./Field"`.

- [ ] **Step 3: Implement the three files**

`Field` holds a `createContext<FieldControlAttrs | undefined>`; `aria-describedby` joins the hint and error ids that exist. `Input`/`Textarea` do `const field = useFieldControl(); <input {...field} {...props} aria-invalid={invalid || field?.['aria-invalid'] ? true : undefined} />` so explicit props win.

- [ ] **Step 4: Run the tests to verify they pass**

Run: same command — Expected: `Tests  13 passed (13)`.

- [ ] **Step 5: Example pages and registry**

`FieldPage` (`<h1>Field</h1>`, stack and row layouts, hint, error, required, with Input and Textarea), `InputPage` (sizes, adornments, invalid, mono, disabled), `TextareaPage` (autoGrow demo bound to local state, mono). `PRIMITIVES` gains `Field: ['Field', 'useFieldControl'], Input: ['Input'], Textarea: ['Textarea']`.

Run: `npx vitest run src/dev/primitives/registry.test.tsx` — Expected: `Tests  3 passed (3)`.

- [ ] **Step 6: Commit**

```bash
git add src/components/ui src/dev/primitives
git commit -m "feat(ui): add Field, Input and Textarea primitives"
```

---

### Task 6: `Checkbox` and `Switch`

**Files:**
- Create: `src/components/ui/Checkbox.tsx`, `Checkbox.test.tsx`, `Switch.tsx`, `Switch.test.tsx`, `src/dev/primitives/pages/CheckboxPage.tsx`, `pages/SwitchPage.tsx`
- Modify: `src/components/ui/index.ts`, `src/dev/primitives/registry.ts`

**Interfaces:**
- Consumes: `Checkbox as CheckboxPrimitive`, `Switch as SwitchPrimitive` from `radix-ui`; `Check`, `Minus` from `lucide-react`; `useFieldControl`.
- Produces:
```ts
export type CheckboxProps = Omit<ComponentProps<typeof CheckboxPrimitive.Root>, 'asChild'> & {
  label?: ReactNode; description?: ReactNode; size?: 'sm' | 'md';
};   // checked?: boolean | 'indeterminate'; onCheckedChange?(checked: boolean | 'indeterminate'): void (Radix)
export function Checkbox(props: CheckboxProps): ReactElement;   // <label> wraps control + text when label is given; role="checkbox" from Radix

export type SwitchProps = Omit<ComponentProps<typeof SwitchPrimitive.Root>, 'asChild'> & {
  label?: ReactNode; description?: ReactNode; size?: 'sm' | 'md'; labelPosition?: 'start' | 'end';   // default 'end'
};
export function Switch(props: SwitchProps): ReactElement;       // role="switch" from Radix; checked thumb bg-accent
```
Keyboard (Radix): Space toggles a checkbox; Space and Enter toggle a switch; both are in the tab order unless disabled.

- [ ] **Step 1: Write the failing tests**

`Checkbox.test.tsx`:
```tsx
test('toggles with Space', async () => { const on = vi.fn(); render(<Checkbox label="Flag" onCheckedChange={on} />); screen.getByRole('checkbox').focus(); await setupUser().keyboard(' '); expect(on).toHaveBeenCalledWith(true); });
test('clicking the label toggles', async () => { const on = vi.fn(); render(<Checkbox label="Flag" onCheckedChange={on} />); await setupUser().click(screen.getByText('Flag')); expect(on).toHaveBeenCalledWith(true); });
test('indeterminate exposes aria-checked="mixed"', () => { render(<Checkbox checked="indeterminate" label="Some" />); expect(screen.getByRole('checkbox')).toHaveAttribute('aria-checked', 'mixed'); });
test('disabled does not toggle', async () => { const on = vi.fn(); render(<Checkbox label="Flag" disabled onCheckedChange={on} />); await setupUser().click(screen.getByText('Flag')); expect(on).not.toHaveBeenCalled(); });
```
`Switch.test.tsx`:
```tsx
test('has role switch and toggles with Space and Enter', async () => {
  const on = vi.fn(); render(<Switch label="May commit" onCheckedChange={on} />); const s = screen.getByRole('switch', { name: 'May commit' });
  s.focus(); const user = setupUser(); await user.keyboard(' '); await user.keyboard('{Enter}'); expect(on).toHaveBeenCalledTimes(2);
});
test('controlled checked reflects aria-checked', () => { render(<Switch checked label="On" />); expect(screen.getByRole('switch')).toHaveAttribute('aria-checked', 'true'); });
test('labelPosition start renders the text before the control', () => { render(<Switch label="Sidebar" labelPosition="start" />); const label = screen.getByText('Sidebar'); expect(label.compareDocumentPosition(screen.getByRole('switch')) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy(); });
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `npx vitest run src/components/ui/Checkbox.test.tsx src/components/ui/Switch.test.tsx`
Expected: FAIL — `Failed to resolve import "./Checkbox"`.

- [ ] **Step 3: Implement both files**

Each generates an id with `useId()` (or the Field's), renders `<label htmlFor={id}>` around the Radix root and the text when `label` is set; `description` is `text-mute` under the label and referenced by `aria-describedby`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: same command — Expected: `Tests  7 passed (7)`.

- [ ] **Step 5: Example pages and registry**

`CheckboxPage` (`<h1>Checkbox</h1>`, unchecked, checked, indeterminate, disabled, with description, two sizes), `SwitchPage` (`<h1>Switch</h1>`, both label positions, sizes, disabled, the "Team Code may commit" row as a `Field layout="row"`). `PRIMITIVES` gains `Checkbox: ['Checkbox'], Switch: ['Switch']`.

Run: `npx vitest run src/dev/primitives/registry.test.tsx` — Expected: `Tests  3 passed (3)`.

- [ ] **Step 6: Commit**

```bash
git add src/components/ui src/dev/primitives
git commit -m "feat(ui): add Checkbox and Switch primitives"
```

---

### Task 7: `Popover`, `Tooltip` and `UiProvider`

**Files:**
- Create: `src/components/ui/Popover.tsx`, `Popover.test.tsx`, `Tooltip.tsx`, `Tooltip.test.tsx`, `UiProvider.tsx`, `src/dev/primitives/pages/PopoverPage.tsx`, `pages/TooltipPage.tsx`
- Modify: `src/components/ui/index.ts`, `src/dev/primitives/registry.ts`, `src/dev/primitives/index.tsx` (wrap `Gallery` in `UiProvider`), `src/test/ui.ts` (add `renderUi`)

**Interfaces:**
- Consumes: `Popover as PopoverPrimitive`, `Tooltip as TooltipPrimitive` from `radix-ui`; `KeyHint`.
- Produces:
```ts
export const Popover: typeof PopoverPrimitive.Root; export const PopoverTrigger: typeof PopoverPrimitive.Trigger;
export const PopoverAnchor: typeof PopoverPrimitive.Anchor; export const PopoverClose: typeof PopoverPrimitive.Close;
export type PopoverContentProps = ComponentProps<typeof PopoverPrimitive.Content> & {
  width?: 'auto' | 'trigger';                    // 'trigger' → w-[var(--radix-popover-trigger-width)]
  padding?: 'none' | 'sm' | 'md';                // default 'sm'
};
export function PopoverContent(props: PopoverContentProps): ReactElement;   // Portal; sideOffset 6; align 'start'; bg-card border-line rounded-well shadow-card; data-motion

export type TooltipProps = {
  content: ReactNode; shortcut?: string;         // shortcut rendered as <KeyHint keys={shortcut} />
  side?: 'top' | 'right' | 'bottom' | 'left'; align?: 'start' | 'center' | 'end';
  delay?: number; disabled?: boolean;            // disabled → children rendered alone
  children: ReactElement;                        // must forward ref (every primitive does); a disabled child is wrapped in <span tabIndex={0}>
};
export function Tooltip(props: TooltipProps): ReactElement;
export function TooltipProvider(props: { children: ReactNode; delayDuration?: number }): ReactElement;  // delayDuration 400, skipDelayDuration 200

export type UiProviderProps = { children: ReactNode };
export function UiProvider(props: UiProviderProps): ReactElement;           // TooltipProvider now; Task 13 adds the Toaster
```
`src/test/ui.ts` gains `renderUi(ui: ReactElement): RenderResult` = `render(<UiProvider>{ui}</UiProvider>)`.
Keyboard (Radix): tooltip shows on focus without delay and hides on Escape or blur; popover opens on Enter/Space, traps nothing, closes on Escape and outside click and returns focus to its trigger.

- [ ] **Step 1: Write the failing tests**

`Popover.test.tsx`:
```tsx
test('opens on trigger click and closes on Escape returning focus', async () => {
  const user = setupUser(); render(<Popover><PopoverTrigger asChild><Button>Filters</Button></PopoverTrigger><PopoverContent>Body</PopoverContent></Popover>);
  await user.click(screen.getByRole('button', { name: 'Filters' })); expect(await screen.findByText('Body')).toBeVisible();
  await user.keyboard('{Escape}'); await waitFor(() => expect(screen.queryByText('Body')).not.toBeInTheDocument()); expect(screen.getByRole('button', { name: 'Filters' })).toHaveFocus();
});
test('width="trigger" sizes the content to the trigger', async () => {
  render(<Popover defaultOpen><PopoverTrigger>t</PopoverTrigger><PopoverContent width="trigger">Body</PopoverContent></Popover>);
  expect((await screen.findByText('Body')).closest('[data-state]')).toHaveClass('w-[var(--radix-popover-trigger-width)]');
});
```
`Tooltip.test.tsx`:
```tsx
test('shows on focus and hides on Escape', async () => {
  const user = setupUser(); renderUi(<Tooltip content="Archive task" delay={0}><Button>A</Button></Tooltip>);
  await user.tab(); expect(await screen.findByRole('tooltip')).toHaveTextContent('Archive task');
  await user.keyboard('{Escape}'); await waitFor(() => expect(screen.queryByRole('tooltip')).not.toBeInTheDocument());
});
test('renders the shortcut as a KeyHint', async () => {
  renderUi(<Tooltip content="Archive" shortcut="cmd+shift+a" delay={0}><Button>A</Button></Tooltip>); await setupUser().tab();
  expect(await screen.findByRole('tooltip')).toHaveTextContent('⌘⇧A');
});
test('disabled renders only the child', () => { renderUi(<Tooltip content="x" disabled><Button>A</Button></Tooltip>); expect(screen.getByRole('button')).toBeInTheDocument(); expect(document.querySelector('[data-state]')).toBeNull(); });
test('shows for a disabled child by wrapping it in a focusable span', async () => {
  renderUi(<Tooltip content="Session already started" delay={0}><Button disabled>Worktree</Button></Tooltip>);
  await setupUser().tab(); expect(await screen.findByRole('tooltip')).toHaveTextContent('Session already started');
  expect(screen.getByRole('button').parentElement).toHaveAttribute('tabindex', '0');
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `npx vitest run src/components/ui/Popover.test.tsx src/components/ui/Tooltip.test.tsx`
Expected: FAIL — `Failed to resolve import "./Popover"`.

- [ ] **Step 3: Implement the three files and `renderUi`**

`Tooltip` composes `TooltipPrimitive.Root/Trigger(asChild)/Portal/Content` with `delayDuration={delay}`; it reads `isValidElement(children) && children.props.disabled` to decide on the span wrapper. `PopoverContent` reads `useReducedMotion()` to set `data-motion` and choose `duration-150` (no zoom) versus `duration-200 zoom-in-95` (the same rule Task 12 pins for Dialog).

- [ ] **Step 4: Run the tests to verify they pass**

Run: same command — Expected: `Tests  6 passed (6)`.

- [ ] **Step 5: Example pages, provider wiring and registry**

`PopoverPage` (`<h1>Popover</h1>`, auto and trigger widths, paddings, a form inside with Field/Input and a PopoverClose Button), `TooltipPage` (`<h1>Tooltip</h1>`, four sides, with shortcut, on a disabled button, on an IconButton). Wrap the gallery root in `<UiProvider>`. `PRIMITIVES` gains `Popover: ['Popover', 'PopoverTrigger', 'PopoverContent', 'PopoverAnchor', 'PopoverClose'], Tooltip: ['Tooltip', 'TooltipProvider']`; the barrel also exports `UiProvider` (not a gallery entry).

Run: `npx vitest run src/dev/primitives/registry.test.tsx` — Expected: `Tests  3 passed (3)`.

- [ ] **Step 6: Commit**

```bash
git add src/components/ui src/dev/primitives src/test/ui.ts
git commit -m "feat(ui): add Popover, Tooltip and UiProvider"
```

---

### Task 8: `Select`

**Files:**
- Create: `src/components/ui/Select.tsx`, `Select.test.tsx`, `src/dev/primitives/pages/SelectPage.tsx`
- Modify: `src/components/ui/index.ts`, `src/dev/primitives/registry.ts`

**Interfaces:**
- Consumes: `Select as SelectPrimitive` from `radix-ui`; `ChevronDown`, `Check`, `ChevronUp` from `lucide-react`; `useFieldControl`.
- Produces:
```ts
export const Select: typeof SelectPrimitive.Root;        // value, defaultValue, onValueChange(value: string), open, disabled, name, required
export const SelectValue: typeof SelectPrimitive.Value;
export const SelectGroup: typeof SelectPrimitive.Group;
export type SelectTriggerProps = ComponentProps<typeof SelectPrimitive.Trigger> & { size?: 'sm' | 'md'; invalid?: boolean };
export function SelectTrigger(props: SelectTriggerProps): ReactElement;       // chevron appended; spreads useFieldControl()
export type SelectContentProps = ComponentProps<typeof SelectPrimitive.Content>;   // position defaults to 'popper'
export function SelectContent(props: SelectContentProps): ReactElement;       // Portal, scroll buttons, viewport, data-motion
export type SelectItemProps = ComponentProps<typeof SelectPrimitive.Item> & { hint?: ReactNode };
export function SelectItem(props: SelectItemProps): ReactElement;             // check indicator at the left, hint text-mute at the right
export function SelectLabel(props: ComponentProps<typeof SelectPrimitive.Label>): ReactElement;
export function SelectSeparator(props: ComponentProps<typeof SelectPrimitive.Separator>): ReactElement;
```
Keyboard (Radix): ArrowDown/ArrowUp/Enter/Space on the trigger open the list; arrows move the highlight; typeahead jumps; Enter or Space selects; Escape closes and refocuses the trigger.

- [ ] **Step 1: Write the failing test**

`Select.test.tsx` with a `Fixture({ onValueChange })` rendering `Select` → `SelectTrigger`(`aria-label="Priority"`) + `SelectValue placeholder="Priority"` → `SelectContent` → items `none`, `low`, `medium`, `high`:
```tsx
test('opens with ArrowDown, moves the highlight and selects with Enter', async () => {
  const on = vi.fn(); render(<Fixture onValueChange={on} />); const user = setupUser();
  screen.getByRole('combobox', { name: 'Priority' }).focus(); await user.keyboard('{ArrowDown}');
  expect(await screen.findByRole('listbox')).toBeInTheDocument();
  await user.keyboard('{ArrowDown}{Enter}'); expect(on).toHaveBeenCalledWith('low');
});
test('Escape closes the list and returns focus to the trigger', async () => {
  render(<Fixture />); const user = setupUser(); const t = screen.getByRole('combobox'); t.focus();
  await user.keyboard('{Enter}'); await screen.findByRole('listbox'); await user.keyboard('{Escape}');
  await waitFor(() => expect(screen.queryByRole('listbox')).not.toBeInTheDocument()); expect(t).toHaveFocus();
});
test('shows the selected label in the trigger', () => { render(<Fixture value="high" />); expect(screen.getByRole('combobox')).toHaveTextContent('High'); });
test('picks up Field ids and invalid state', () => {
  render(<Field label="Agent" error="Pick one"><Fixture /></Field>); const t = screen.getByRole('combobox', { name: 'Priority' }); expect(t).toHaveAttribute('aria-invalid', 'true'); expect(t).toHaveAttribute('id', expect.stringMatching(/./));
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npx vitest run src/components/ui/Select.test.tsx`
Expected: FAIL — `Failed to resolve import "./Select"`.

- [ ] **Step 3: Implement `Select.tsx`**

Trigger: `controlHeight[size] bg-card border border-line rounded-card px-2 text-left justify-between`, `data-[placeholder]:text-mute`. Content: `bg-card border-line rounded-well shadow-card`, `position="popper"` default, viewport `min-w-[var(--radix-select-trigger-width)]`. Item: `data-[highlighted]:bg-well`, `data-[disabled]:opacity-50`.

- [ ] **Step 4: Run the test to verify it passes**

Run: `npx vitest run src/components/ui/Select.test.tsx` — Expected: `Tests  4 passed (4)`.

- [ ] **Step 5: Example page and registry**

`SelectPage` (`<h1>Select</h1>`, sizes, with groups and labels, hints, disabled items, inside a Field with error). `PRIMITIVES` gains `Select: ['Select', 'SelectTrigger', 'SelectValue', 'SelectContent', 'SelectItem', 'SelectGroup', 'SelectLabel', 'SelectSeparator']`.

Run: `npx vitest run src/dev/primitives/registry.test.tsx` — Expected: `Tests  3 passed (3)`.

- [ ] **Step 6: Commit**

```bash
git add src/components/ui src/dev/primitives
git commit -m "feat(ui): add Select primitive"
```

---

### Task 9: `Combobox`

**Files:**
- Create: `src/components/ui/Combobox.tsx`, `Combobox.test.tsx`, `src/dev/primitives/pages/ComboboxPage.tsx`
- Modify: `src/components/ui/index.ts`, `src/dev/primitives/registry.ts`

**Interfaces:**
- Consumes: `Popover`, `PopoverTrigger`, `PopoverContent` (Task 7); `Button` (Task 3); `Command, CommandInput, CommandList, CommandEmpty, CommandItem` from `cmdk`; `useFieldControl`.
- Produces:
```ts
export type ComboboxOption = { value: string; label: string; hint?: string; keywords?: string[]; disabled?: boolean };
export type ComboboxProps = {
  options: readonly ComboboxOption[];
  value: string | null; onValueChange(value: string | null): void;
  placeholder?: string;        // 'Select…'
  searchPlaceholder?: string;  // 'Search…'
  emptyText?: string;          // 'No matches'
  size?: 'sm' | 'md'; invalid?: boolean; disabled?: boolean; clearable?: boolean; className?: string;
  renderOption?(option: ComboboxOption, selected: boolean): ReactNode;
  'aria-label'?: string;
};
export function Combobox(props: ComboboxProps): ReactElement;
// Trigger: <Button variant="secondary" role="combobox" aria-expanded aria-controls> showing the selected label or the placeholder (text-mute).
// Content: PopoverContent width="trigger" padding="none" containing cmdk's Command with CommandInput (role searchbox), CommandList (role listbox), CommandEmpty, one CommandItem per option (value = option.value, keywords = [label, ...keywords]).
```
Keyboard (cmdk inside Popover): typing filters; ArrowDown/ArrowUp move the highlight (first item highlighted on open); Enter selects the highlighted option, calls `onValueChange` and closes; Escape closes without selecting and refocuses the trigger.

- [ ] **Step 1: Write the failing test**

`Combobox.test.tsx` with `const options = [{ value: 'frontend', label: 'Frontend' }, { value: 'backend', label: 'Backend' }, { value: 'security', label: 'Security reviewer' }]`:
```tsx
test('typing filters the options', async () => {
  const user = setupUser(); render(<Combobox options={options} value={null} onValueChange={() => {}} aria-label="Role" />);
  await user.click(screen.getByRole('combobox', { name: 'Role' })); await user.type(screen.getByRole('combobox', { name: 'Search…' }) /* cmdk input */, 'back');
  expect(screen.getByText('Backend')).toBeInTheDocument(); expect(screen.queryByText('Frontend')).not.toBeInTheDocument();
});
test('ArrowDown then Enter selects the highlighted option and closes', async () => {
  const on = vi.fn(); const user = setupUser(); render(<Combobox options={options} value={null} onValueChange={on} aria-label="Role" />);
  await user.click(screen.getByRole('combobox', { name: 'Role' })); await screen.findByRole('listbox');
  await user.keyboard('{ArrowDown}{Enter}'); expect(on).toHaveBeenCalledWith('backend');
  await waitFor(() => expect(screen.queryByRole('listbox')).not.toBeInTheDocument());
});
test('Escape closes without selecting', async () => {
  const on = vi.fn(); const user = setupUser(); render(<Combobox options={options} value={null} onValueChange={on} aria-label="Role" />);
  await user.click(screen.getByRole('combobox', { name: 'Role' })); await screen.findByRole('listbox'); await user.keyboard('{Escape}');
  await waitFor(() => expect(screen.queryByRole('listbox')).not.toBeInTheDocument()); expect(on).not.toHaveBeenCalled(); expect(screen.getByRole('combobox', { name: 'Role' })).toHaveFocus();
});
test('shows emptyText when nothing matches', async () => {
  const user = setupUser(); render(<Combobox options={options} value={null} onValueChange={() => {}} emptyText="No roles" aria-label="Role" />);
  await user.click(screen.getByRole('combobox', { name: 'Role' })); await user.keyboard('zzz'); expect(await screen.findByText('No roles')).toBeInTheDocument();
});
test('shows the placeholder when value is not among the options', () => {
  render(<Combobox options={options} value="removed" onValueChange={() => {}} placeholder="Pick a role" aria-label="Role" />); expect(screen.getByRole('combobox', { name: 'Role' })).toHaveTextContent('Pick a role');
});
test('clearable renders a clear control that sets null', async () => {
  const on = vi.fn(); render(<Combobox options={options} value="backend" onValueChange={on} clearable aria-label="Role" />); await setupUser().click(screen.getByRole('button', { name: 'Clear' })); expect(on).toHaveBeenCalledWith(null);
});
```
Note on the first test: cmdk's input has `role="combobox"` too; query it by its placeholder name. If cmdk's version exposes it as `searchbox`, change that one query, not the component.

- [ ] **Step 2: Run the test to verify it fails**

Run: `npx vitest run src/components/ui/Combobox.test.tsx`
Expected: FAIL — `Failed to resolve import "./Combobox"`.

- [ ] **Step 3: Implement `Combobox.tsx`**

Internal `open` state; cmdk's `<Command label={searchPlaceholder}>` gives the search input its accessible name and `<CommandInput autoFocus>` puts the keyboard in it on open; `onSelect` of an item calls `onValueChange(option.value)` and closes; the selected item shows a `Check`; the clear control is an `IconButton label="Clear"` inside the trigger's trailing slot (stop propagation so it does not open the list). The cmdk list is styled `max-h-64 overflow-auto`, items `data-[selected=true]:bg-well`.

- [ ] **Step 4: Run the test to verify it passes**

Run: `npx vitest run src/components/ui/Combobox.test.tsx` — Expected: `Tests  6 passed (6)`.

- [ ] **Step 5: Example page and registry**

`ComboboxPage` (`<h1>Combobox</h1>`, the five roster roles with hints, a 60-item list, clearable, disabled, inside a Field with error, custom `renderOption` with a `Badge`). `PRIMITIVES` gains `Combobox: ['Combobox']`.

Run: `npx vitest run src/dev/primitives/registry.test.tsx` — Expected: `Tests  3 passed (3)`.

- [ ] **Step 6: Commit**

```bash
git add src/components/ui src/dev/primitives
git commit -m "feat(ui): add Combobox primitive"
```

---

### Task 10: `Segmented` and `Tabs`

**Files:**
- Create: `src/components/ui/Segmented.tsx`, `Segmented.test.tsx`, `Tabs.tsx`, `Tabs.test.tsx`, `src/dev/primitives/pages/SegmentedPage.tsx`, `pages/TabsPage.tsx`
- Modify: `src/components/ui/index.ts`, `src/dev/primitives/registry.ts`

**Interfaces:**
- Consumes: `RadioGroup as RadioGroupPrimitive`, `Tabs as TabsPrimitive` from `radix-ui`; `Badge`, `KeyHint`.
- Produces:
```ts
// Segmented.tsx — exactly one option selected; wraps Radix RadioGroup (role="radiogroup", items role="radio").
export type SegmentedOption<T extends string> = { value: T; label: ReactNode; icon?: ReactNode; disabled?: boolean; shortcut?: string };
export type SegmentedProps<T extends string> = Omit<ComponentProps<'div'>, 'onChange' | 'defaultValue'> & {
  options: readonly SegmentedOption<T>[]; value: T; onValueChange(value: T): void;
  size?: 'sm' | 'md'; fullWidth?: boolean; 'aria-label': string;
};
export function Segmented<T extends string>(props: SegmentedProps<T>): ReactElement;

// Tabs.tsx — Radix Tabs; activationMode 'automatic' (arrow keys both move and activate).
export const Tabs: typeof TabsPrimitive.Root;   // value, defaultValue, onValueChange(value: string), orientation
export type TabsListProps = ComponentProps<typeof TabsPrimitive.List> & { variant?: 'underline' | 'pill' | 'bar'; size?: 'sm' | 'md' };  // default 'underline'; 'bar' = window tab strip look on bg-bar
export function TabsList(props: TabsListProps): ReactElement;
export type TabsTriggerProps = ComponentProps<typeof TabsPrimitive.Trigger> & { icon?: ReactNode; count?: number; countTone?: Tone };  // count → <Badge tone={countTone ?? 'warn'} size="sm">; hidden when 0 or undefined
export function TabsTrigger(props: TabsTriggerProps): ReactElement;
export function TabsContent(props: ComponentProps<typeof TabsPrimitive.Content>): ReactElement;   // focusRing, outline-none
```
Keyboard (Radix): Segmented — ArrowRight/ArrowDown select the next option, ArrowLeft/ArrowUp the previous, wrapping; Home/End jump; Space selects a focused option; only the selected option is in the tab order. Tabs — ArrowRight/ArrowLeft move and activate (wrapping), Home/End jump, only the active tab has `tabIndex=0`.

- [ ] **Step 1: Write the failing tests**

`Segmented.test.tsx` with `const opts = [{ value: 'off', label: 'Off' }, { value: 'simple', label: 'Simple' }, { value: 'heavy', label: 'Heavy' }] as const`:
```tsx
test('renders a radiogroup with the selected option checked', () => { render(<Segmented aria-label="LiveSpec" options={opts} value="simple" onValueChange={() => {}} />); expect(screen.getByRole('radiogroup', { name: 'LiveSpec' })).toBeInTheDocument(); expect(screen.getByRole('radio', { name: 'Simple' })).toHaveAttribute('aria-checked', 'true'); });
test('ArrowRight selects the next option', async () => { const on = vi.fn(); render(<Segmented aria-label="L" options={opts} value="off" onValueChange={on} />); screen.getByRole('radio', { name: 'Off' }).focus(); await setupUser().keyboard('{ArrowRight}'); expect(on).toHaveBeenCalledWith('simple'); });
test('ArrowLeft wraps from the first to the last option', async () => { const on = vi.fn(); render(<Segmented aria-label="L" options={opts} value="off" onValueChange={on} />); screen.getByRole('radio', { name: 'Off' }).focus(); await setupUser().keyboard('{ArrowLeft}'); expect(on).toHaveBeenCalledWith('heavy'); });
test('clicking an option selects it', async () => { const on = vi.fn(); render(<Segmented aria-label="L" options={opts} value="off" onValueChange={on} />); await setupUser().click(screen.getByRole('radio', { name: 'Heavy' })); expect(on).toHaveBeenCalledWith('heavy'); });
test('only the selected option is in the tab order', () => { render(<Segmented aria-label="L" options={opts} value="simple" onValueChange={() => {}} />); expect(screen.getByRole('radio', { name: 'Simple' })).toHaveAttribute('tabindex', '0'); expect(screen.getByRole('radio', { name: 'Off' })).toHaveAttribute('tabindex', '-1'); });
```
`Tabs.test.tsx` with a `Fixture` of tabs `memory`, `team`, `changes` (the `team` trigger has `count={2}`):
```tsx
test('ArrowRight moves focus and activates the next tab', async () => {
  const on = vi.fn(); render(<Fixture onValueChange={on} />); screen.getByRole('tab', { name: /Memory/ }).focus(); await setupUser().keyboard('{ArrowRight}');
  expect(on).toHaveBeenCalledWith('team'); expect(screen.getByRole('tab', { name: /Team/ })).toHaveFocus();
});
test('Home and End jump to the first and last tab', async () => {
  const on = vi.fn(); render(<Fixture onValueChange={on} />); screen.getByRole('tab', { name: /Team/ }).focus(); const user = setupUser();
  await user.keyboard('{End}'); expect(on).toHaveBeenLastCalledWith('changes'); await user.keyboard('{Home}'); expect(on).toHaveBeenLastCalledWith('memory');
});
test('only the active tab is in the tab order and its panel is shown', () => {
  render(<Fixture value="team" />); expect(screen.getByRole('tab', { name: /Team/ })).toHaveAttribute('tabindex', '0'); expect(screen.getByRole('tab', { name: /Memory/ })).toHaveAttribute('tabindex', '-1'); expect(screen.getByRole('tabpanel')).toHaveTextContent('Team panel');
});
test('count renders an amber badge and is hidden at zero', () => { render(<Fixture value="team" />); expect(within(screen.getByRole('tab', { name: /Team/ })).getByText('2')).toHaveClass('text-warn'); expect(within(screen.getByRole('tab', { name: /Changes/ })).queryByText('0')).toBeNull(); });
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `npx vitest run src/components/ui/Segmented.test.tsx src/components/ui/Tabs.test.tsx`
Expected: FAIL — `Failed to resolve import "./Segmented"`.

- [ ] **Step 3: Implement both files**

`Segmented`: `RadioGroupPrimitive.Root` with `loop`, `orientation="horizontal"`, container `inline-flex bg-well rounded-well p-0.5 border border-line`; items `data-[state=checked]:bg-card data-[state=checked]:text-ink text-mute rounded-card`; `shortcut` renders a `KeyHint size="sm"`. `Tabs`: `TabsList` variants — `underline` (`border-b border-line`, active `border-b-2 border-accent text-ink`), `pill` (like Segmented), `bar` (`bg-bar`, active `bg-page`).

- [ ] **Step 4: Run the tests to verify they pass**

Run: same command — Expected: `Tests  9 passed (9)`.

- [ ] **Step 5: Example pages and registry**

`SegmentedPage` (`<h1>Segmented</h1>`, the LiveSpec switch Off/Simple/Heavy, the Detail level, icon-only options with tooltips, fullWidth, disabled option, with shortcuts), `TabsPage` (`<h1>Tabs</h1>`, the dock tab set with counts in all three variants, vertical orientation). `PRIMITIVES` gains `Segmented: ['Segmented'], Tabs: ['Tabs', 'TabsList', 'TabsTrigger', 'TabsContent']`.

Run: `npx vitest run src/dev/primitives/registry.test.tsx` — Expected: `Tests  3 passed (3)`.

- [ ] **Step 6: Commit**

```bash
git add src/components/ui src/dev/primitives
git commit -m "feat(ui): add Segmented and Tabs primitives"
```

---

### Task 11: `Menu` and `ContextMenu`

**Files:**
- Create: `src/components/ui/Menu.tsx`, `Menu.test.tsx`, `ContextMenu.tsx`, `ContextMenu.test.tsx`, `src/dev/primitives/pages/MenuPage.tsx`, `pages/ContextMenuPage.tsx`
- Modify: `src/components/ui/index.ts`, `src/dev/primitives/registry.ts`

**Interfaces:**
- Consumes: `DropdownMenu as DropdownMenuPrimitive`, `ContextMenu as ContextMenuPrimitive` from `radix-ui`; `KeyHint`; `Check`, `Circle`, `ChevronRight` from `lucide-react`.
- Produces (the two files export parallel sets; `ContextMenu*` has the same members and props with the prefix swapped):
```ts
export const Menu: typeof DropdownMenuPrimitive.Root; export const MenuTrigger: typeof DropdownMenuPrimitive.Trigger;
export const MenuGroup: typeof DropdownMenuPrimitive.Group; export const MenuSub: typeof DropdownMenuPrimitive.Sub;
export const MenuRadioGroup: typeof DropdownMenuPrimitive.RadioGroup;
export type MenuContentProps = ComponentProps<typeof DropdownMenuPrimitive.Content> & { minWidth?: number };   // default 180; sideOffset 4; align 'start'; Portal; data-motion
export function MenuContent(props: MenuContentProps): ReactElement;
export function MenuSubContent(props: ComponentProps<typeof DropdownMenuPrimitive.SubContent>): ReactElement;
export function MenuSubTrigger(props: ComponentProps<typeof DropdownMenuPrimitive.SubTrigger> & { icon?: ReactNode; inset?: boolean }): ReactElement;
export type MenuItemProps = ComponentProps<typeof DropdownMenuPrimitive.Item> & {
  icon?: ReactNode; shortcut?: string; hint?: ReactNode; danger?: boolean; inset?: boolean;   // hint: muted right-aligned text (e.g. "after the session starts"); shortcut: KeyHint
};
export function MenuItem(props: MenuItemProps): ReactElement;
export function MenuCheckboxItem(props: ComponentProps<typeof DropdownMenuPrimitive.CheckboxItem> & { shortcut?: string }): ReactElement;
export function MenuRadioItem(props: ComponentProps<typeof DropdownMenuPrimitive.RadioItem> & { swatch?: Swatch }): ReactElement;   // swatch renders a colour square before the label
export function MenuLabel(props: ComponentProps<typeof DropdownMenuPrimitive.Label> & { inset?: boolean }): ReactElement;
export function MenuSeparator(props: ComponentProps<typeof DropdownMenuPrimitive.Separator>): ReactElement;
// ContextMenu.tsx: ContextMenu, ContextMenuTrigger, ContextMenuContent, ContextMenuItem, ContextMenuCheckboxItem, ContextMenuRadioGroup, ContextMenuRadioItem,
// ContextMenuLabel, ContextMenuSeparator, ContextMenuGroup, ContextMenuSub, ContextMenuSubTrigger, ContextMenuSubContent — same props.
```
Keyboard (Radix): Enter, Space or ArrowDown on the trigger opens and focuses the first item; arrows move with wrap; typeahead; ArrowRight opens a submenu, ArrowLeft closes it; Enter or Space selects (`onSelect`) and closes; Escape closes and refocuses the trigger. ContextMenu opens on right-click or Shift+F10 at the pointer.

- [ ] **Step 1: Write the failing tests**

`Menu.test.tsx` with a `Fixture({ onRename, onToggle, onArchive })` — trigger `IconButton label="Card menu"`, items *Rename* (`shortcut="cmd+e"`), *Toggle worktree* (`disabled hint="Session started"`), separator, *Archive* (`danger shortcut="cmd+shift+a"`):
```tsx
test('Enter on the trigger opens the menu and focuses the first item', async () => {
  render(<Fixture />); screen.getByRole('button', { name: 'Card menu' }).focus(); await setupUser().keyboard('{Enter}');
  expect(await screen.findByRole('menu')).toBeInTheDocument(); expect(screen.getByRole('menuitem', { name: /Rename/ })).toHaveFocus();
});
test('ArrowDown moves to the next enabled item and Enter selects it', async () => {
  const onArchive = vi.fn(); render(<Fixture onArchive={onArchive} />); const user = setupUser();
  screen.getByRole('button', { name: 'Card menu' }).focus(); await user.keyboard('{Enter}'); await screen.findByRole('menu');
  await user.keyboard('{ArrowDown}{Enter}'); expect(onArchive).toHaveBeenCalledTimes(1); await waitFor(() => expect(screen.queryByRole('menu')).not.toBeInTheDocument());
});
test('Escape closes and returns focus to the trigger', async () => {
  render(<Fixture />); const user = setupUser(); const t = screen.getByRole('button', { name: 'Card menu' }); t.focus();
  await user.keyboard('{Enter}'); await screen.findByRole('menu'); await user.keyboard('{Escape}'); await waitFor(() => expect(screen.queryByRole('menu')).not.toBeInTheDocument()); expect(t).toHaveFocus();
});
test('renders shortcut glyphs and hints', async () => {
  render(<Fixture />); screen.getByRole('button', { name: 'Card menu' }).focus(); await setupUser().keyboard('{Enter}');
  expect(await screen.findByText('⌘⇧A')).toBeInTheDocument(); expect(screen.getByText('Session started')).toBeInTheDocument();
});
test('a disabled item is aria-disabled and does not fire onSelect', async () => {
  const onToggle = vi.fn(); render(<Fixture onToggle={onToggle} />); const user = setupUser(); screen.getByRole('button', { name: 'Card menu' }).focus(); await user.keyboard('{Enter}');
  const item = await screen.findByRole('menuitem', { name: /Toggle worktree/ }); expect(item).toHaveAttribute('aria-disabled', 'true'); await user.click(item); expect(onToggle).not.toHaveBeenCalled();
});
test('danger items use the bad tone', async () => { render(<Fixture />); screen.getByRole('button', { name: 'Card menu' }).focus(); await setupUser().keyboard('{Enter}'); expect(await screen.findByRole('menuitem', { name: /Archive/ })).toHaveClass('text-bad'); });
```
`ContextMenu.test.tsx` with a `Fixture` whose trigger is `<div>Card</div>` and items *Open*, *Archive*:
```tsx
test('opens on right-click', async () => { render(<Fixture />); fireEvent.contextMenu(screen.getByText('Card')); expect(await screen.findByRole('menu')).toBeInTheDocument(); });
test('ArrowDown and Enter select an item', async () => {
  const onOpen = vi.fn(); render(<Fixture onOpen={onOpen} />); fireEvent.contextMenu(screen.getByText('Card')); await screen.findByRole('menu');
  await setupUser().keyboard('{ArrowDown}{Enter}'); expect(onOpen).toHaveBeenCalledTimes(1);
});
test('Escape closes the menu', async () => { render(<Fixture />); fireEvent.contextMenu(screen.getByText('Card')); await screen.findByRole('menu'); await setupUser().keyboard('{Escape}'); await waitFor(() => expect(screen.queryByRole('menu')).not.toBeInTheDocument()); });
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `npx vitest run src/components/ui/Menu.test.tsx src/components/ui/ContextMenu.test.tsx`
Expected: FAIL — `Failed to resolve import "./Menu"`.

- [ ] **Step 3: Implement both files**

Shared item classes (keep them in `Menu.tsx` and import into `ContextMenu.tsx`): `flex h-7 items-center gap-2 rounded-card px-2 text-sm text-ink data-[highlighted]:bg-well data-[disabled]:opacity-50 outline-none`; `danger` → `text-bad data-[highlighted]:bg-bad/10`; right slot `ml-auto flex items-center gap-2 text-mute` holding `hint` then `KeyHint`. Content: `bg-card border border-line rounded-well shadow-card p-1`, `style={{ minWidth }}` (a size, not a colour).

- [ ] **Step 4: Run the tests to verify they pass**

Run: same command — Expected: `Tests  9 passed (9)`.

- [ ] **Step 5: Example pages and registry**

`MenuPage` (`<h1>Menu</h1>`, the whole §10.2 card menu: open in current view, open in new view ⌘-click, open in another window, rename ⌘E, priority radio group, colour radio group with swatches, flag and hide checkbox items, archive ⌘⇧A danger, link Jira issue, toggle worktree disabled with hint, stop agent), `ContextMenuPage` (`<h1>ContextMenu</h1>`, a card-sized box with the same menu). `PRIMITIVES` gains `Menu: [...all Menu members], ContextMenu: [...all ContextMenu members]`.

Run: `npx vitest run src/dev/primitives/registry.test.tsx` — Expected: `Tests  3 passed (3)`.

- [ ] **Step 6: Commit**

```bash
git add src/components/ui src/dev/primitives
git commit -m "feat(ui): add Menu and ContextMenu primitives"
```

---

### Task 12: `Dialog`

**Files:**
- Create: `src/components/ui/Dialog.tsx`, `Dialog.test.tsx`, `src/dev/primitives/pages/DialogPage.tsx`
- Modify: `src/components/ui/index.ts`, `src/dev/primitives/registry.ts`

**Interfaces:**
- Consumes: `Dialog as DialogPrimitive` from `radix-ui`; `IconButton`, `X` from `lucide-react`; `useReducedMotion`.
- Produces:
```ts
export const Dialog: typeof DialogPrimitive.Root; export const DialogTrigger: typeof DialogPrimitive.Trigger;
export const DialogClose: typeof DialogPrimitive.Close; export const DialogPortal: typeof DialogPrimitive.Portal;
export type DialogContentProps = ComponentProps<typeof DialogPrimitive.Content> & {
  size?: 'sm' | 'md' | 'lg';          // max-w-[360px] | max-w-[480px] | max-w-[640px]; default 'md'
  hideClose?: boolean;                // the top-right IconButton label="Close"
};
export function DialogContent(props: DialogContentProps): ReactElement;
// Renders Portal > Overlay (bg-ink/40) + Content (bg-card border-line rounded-well shadow-card p-4, fixed centred).
// data-motion="reduced" | "full" from useReducedMotion(): reduced → 'duration-150 ease-out data-[state=open]:fade-in-0 data-[state=closed]:fade-out-0';
// full → 'duration-200 data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95 data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95'.
export function DialogHeader(props: ComponentProps<'div'>): ReactElement;
export function DialogTitle(props: ComponentProps<typeof DialogPrimitive.Title>): ReactElement;          // text-ink font-medium
export function DialogDescription(props: ComponentProps<typeof DialogPrimitive.Description>): ReactElement;   // text-mute
export function DialogFooter(props: ComponentProps<'div'>): ReactElement;    // flex justify-end gap-2
```
Keyboard (Radix): focus moves into the content on open (first focusable, else the content), Tab is trapped, Escape closes, focus returns to the trigger on close; nested dialogs stack so Escape closes only the top one.

- [ ] **Step 1: Write the failing test**

`Dialog.test.tsx` with `Fixture()` = `Dialog` → `DialogTrigger asChild Button "Archive"` → `DialogContent` → `DialogHeader`(`DialogTitle "Archive task"`, `DialogDescription "The agent is still working."`) → `DialogFooter`(`DialogClose asChild Button "Cancel"`, `Button variant="danger" "Archive"`):
```tsx
test('opens from its trigger, moves focus inside, closes on Escape and returns focus', async () => {
  render(<Fixture />); const user = setupUser(); const t = screen.getByRole('button', { name: 'Archive' }); await user.click(t);
  const dialog = await screen.findByRole('dialog', { name: 'Archive task' }); expect(dialog).toContainElement(document.activeElement as HTMLElement);
  await user.keyboard('{Escape}'); await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument()); expect(t).toHaveFocus();
});
test('the close button closes it', async () => { render(<Fixture />); const user = setupUser(); await user.click(screen.getByRole('button', { name: 'Archive' })); await user.click(await screen.findByRole('button', { name: 'Close' })); await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument()); });
test('is described by its description', async () => { render(<Fixture />); await setupUser().click(screen.getByRole('button', { name: 'Archive' })); expect(await screen.findByRole('dialog')).toHaveAccessibleDescription('The agent is still working.'); });
test('full motion zooms in over 200 ms', async () => {
  render(<Fixture />); await setupUser().click(screen.getByRole('button', { name: 'Archive' })); const d = await screen.findByRole('dialog');
  expect(d).toHaveAttribute('data-motion', 'full'); expect(d).toHaveClass('duration-200', 'data-[state=open]:zoom-in-95');
});
test('reduced motion fades over 150 ms with no zoom', async () => {
  mockMatchMedia(true); render(<Fixture />); await setupUser().click(screen.getByRole('button', { name: 'Archive' })); const d = await screen.findByRole('dialog');
  expect(d).toHaveAttribute('data-motion', 'reduced'); expect(d).toHaveClass('duration-150'); expect(d).not.toHaveClass('data-[state=open]:zoom-in-95');
});
test('nested dialog: Escape closes only the inner dialog', async () => {
  render(<Dialog defaultOpen><DialogContent><DialogTitle>Outer</DialogTitle><Dialog><DialogTrigger asChild><Button>More</Button></DialogTrigger><DialogContent><DialogTitle>Inner</DialogTitle></DialogContent></Dialog></DialogContent></Dialog>);
  const user = setupUser(); await user.click(await screen.findByRole('button', { name: 'More' })); await screen.findByRole('dialog', { name: 'Inner' });
  await user.keyboard('{Escape}'); await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Inner' })).not.toBeInTheDocument()); expect(screen.getByRole('dialog', { name: 'Outer' })).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npx vitest run src/components/ui/Dialog.test.tsx`
Expected: FAIL — `Failed to resolve import "./Dialog"`.

- [ ] **Step 3: Implement `Dialog.tsx`** per Interfaces; the overlay also carries `data-motion` and fades with the same durations.

- [ ] **Step 4: Run the test to verify it passes**

Run: `npx vitest run src/components/ui/Dialog.test.tsx` — Expected: `Tests  6 passed (6)`.

- [ ] **Step 5: Example page and registry**

`DialogPage` (`<h1>Dialog</h1>`, three sizes, the archive confirmation, a form dialog with Fields, a nested dialog, hideClose). `PRIMITIVES` gains `Dialog: ['Dialog', 'DialogTrigger', 'DialogContent', 'DialogHeader', 'DialogTitle', 'DialogDescription', 'DialogFooter', 'DialogClose']`.

Run: `npx vitest run src/dev/primitives/registry.test.tsx` — Expected: `Tests  3 passed (3)`.

- [ ] **Step 6: Commit**

```bash
git add src/components/ui src/dev/primitives
git commit -m "feat(ui): add Dialog primitive with reduced-motion handling"
```

---

### Task 13: `Drawer` (Vaul) and `Toast` (Sonner)

**Files:**
- Create: `src/components/ui/Drawer.tsx`, `Drawer.test.tsx`, `Toast.tsx`, `Toast.test.tsx`, `src/dev/primitives/pages/DrawerPage.tsx`, `pages/ToastPage.tsx`
- Modify: `src/components/ui/index.ts`, `src/dev/primitives/registry.ts`, `src/components/ui/UiProvider.tsx` (mount `Toaster`)

**Interfaces:**
- Consumes: `Drawer as VaulDrawer` from `vaul`; `Toaster as SonnerToaster, toast` from `sonner`; `useReducedMotion`; `primitives.css` Vaul override (Task 1).
- Produces:
```ts
// Drawer.tsx
export type DrawerProps = Omit<ComponentProps<typeof VaulDrawer.Root>, 'direction' | 'shouldScaleBackground'> & { side?: 'bottom' | 'right' };  // default 'bottom'
export function Drawer(props: DrawerProps): ReactElement;    // VaulDrawer.Root direction={side} shouldScaleBackground={false}; provides side via context
export const DrawerTrigger: typeof VaulDrawer.Trigger; export const DrawerClose: typeof VaulDrawer.Close; export const DrawerPortal: typeof VaulDrawer.Portal;
export type DrawerContentProps = ComponentProps<typeof VaulDrawer.Content> & { size?: 'sm' | 'md' | 'lg' };   // bottom: max-h 40/60/85vh; right: w 320/420/560px
export function DrawerContent(props: DrawerContentProps): ReactElement;   // Portal > Overlay + Content; bottom side renders VaulDrawer.Handle; data-motion from useReducedMotion()
export function DrawerHeader(props: ComponentProps<'div'>): ReactElement; export function DrawerTitle(props: ComponentProps<typeof VaulDrawer.Title>): ReactElement;
export function DrawerDescription(props: ComponentProps<typeof VaulDrawer.Description>): ReactElement; export function DrawerFooter(props: ComponentProps<'div'>): ReactElement;

// Toast.tsx
export { toast } from 'sonner';                               // toast('…'), toast.success/error/info/warning/promise, { action: { label, onClick }, description, duration }
export type ToasterProps = { position?: 'bottom-right' | 'bottom-center' | 'top-right'; theme?: 'light' | 'dark' | 'system' };   // defaults 'bottom-right', 'system'
export function Toaster(props: ToasterProps): ReactElement;   // Sonner with closeButton, duration 4000, visibleToasts 4, token classNames (toast: bg-card border-line text-ink; description: text-mute; actionButton: bg-accent text-accent-ink; error icon text-bad; success icon text-ok)
```
`UiProvider` now renders `<TooltipProvider>{children}<Toaster /></TooltipProvider>`.
Keyboard: Drawer inherits Radix Dialog behaviour (focus in, trap, Escape closes, focus returns); Sonner toasts are reachable with the keyboard (Sonner's `hotkey` default Alt+T focuses the region) and the action button is a focusable button.

- [ ] **Step 1: Write the failing tests**

`Drawer.test.tsx` with `Fixture({ side })` = `Drawer side` → `DrawerTrigger asChild Button "Costs"` → `DrawerContent` → `DrawerHeader`(`DrawerTitle "Costs"`):
```tsx
test('opens from its trigger and closes on Escape', async () => {
  render(<Fixture />); const user = setupUser(); await user.click(screen.getByRole('button', { name: 'Costs' }));
  expect(await screen.findByRole('dialog', { name: 'Costs' })).toBeInTheDocument(); await user.keyboard('{Escape}'); await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
});
test('side="right" docks to the right edge', async () => { render(<Fixture side="right" />); await setupUser().click(screen.getByRole('button', { name: 'Costs' })); expect(await screen.findByRole('dialog')).toHaveClass('right-0', 'inset-y-0'); });
test('bottom drawers render a handle', async () => { render(<Fixture />); await setupUser().click(screen.getByRole('button', { name: 'Costs' })); expect((await screen.findByRole('dialog')).querySelector('[data-vaul-handle]')).not.toBeNull(); });
test('reduced motion marks the content so the 150 ms override applies', async () => {
  mockMatchMedia(true); render(<Fixture />); await setupUser().click(screen.getByRole('button', { name: 'Costs' }));
  expect(await screen.findByRole('dialog')).toHaveAttribute('data-motion', 'reduced');
});
test('full motion marks the content full', async () => { render(<Fixture />); await setupUser().click(screen.getByRole('button', { name: 'Costs' })); expect(await screen.findByRole('dialog')).toHaveAttribute('data-motion', 'full'); });
```
`Toast.test.tsx`:
```tsx
test('shows a toast message', async () => { render(<Toaster />); act(() => { toast('Moved to Done'); }); expect(await screen.findByText('Moved to Done')).toBeInTheDocument(); });
test('renders an action button that fires its handler', async () => {
  const onClick = vi.fn(); render(<Toaster />); act(() => { toast('Archived', { action: { label: 'Undo', onClick } }); });
  await setupUser().click(await screen.findByRole('button', { name: 'Undo' })); expect(onClick).toHaveBeenCalledTimes(1);
});
test('error toasts carry data-type="error"', async () => { render(<Toaster />); act(() => { toast.error('Hook failed'); }); expect((await screen.findByText('Hook failed')).closest('[data-sonner-toast]')).toHaveAttribute('data-type', 'error'); });
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `npx vitest run src/components/ui/Drawer.test.tsx src/components/ui/Toast.test.tsx`
Expected: FAIL — `Failed to resolve import "./Drawer"`.

- [ ] **Step 3: Implement both files and extend `UiProvider`**

`DrawerContent` classes: bottom `fixed inset-x-0 bottom-0 rounded-t-well`, right `fixed right-0 inset-y-0 rounded-l-well`; always `bg-card border-line shadow-card flex flex-col`. Vaul content already carries `data-vaul-drawer`, so `data-motion="reduced"` on the same element triggers the `primitives.css` override.

- [ ] **Step 4: Run the tests to verify they pass**

Run: same command — Expected: `Tests  8 passed (8)`.

- [ ] **Step 5: Example pages and registry**

`DrawerPage` (`<h1>Drawer</h1>`, bottom at three sizes, right side, with a Field form and footer), `ToastPage` (`<h1>Toast</h1>`, buttons firing default, success, error, with description, with Undo action, promise). `PRIMITIVES` gains `Drawer: ['Drawer', 'DrawerTrigger', 'DrawerContent', 'DrawerHeader', 'DrawerTitle', 'DrawerDescription', 'DrawerFooter', 'DrawerClose'], Toast: ['Toaster', 'toast']`.

Run: `npx vitest run src/dev/primitives/registry.test.tsx` — Expected: `Tests  3 passed (3)`.

- [ ] **Step 6: Commit**

```bash
git add src/components/ui src/dev/primitives
git commit -m "feat(ui): add Drawer and Toast primitives"
```

---

### Task 14: `Chip` and `Avatar`

**Files:**
- Create: `src/components/ui/Chip.tsx`, `Chip.test.tsx`, `Avatar.tsx`, `Avatar.test.tsx`, `src/dev/primitives/pages/ChipPage.tsx`, `pages/AvatarPage.tsx`
- Modify: `src/components/ui/index.ts`, `src/dev/primitives/registry.ts`

**Interfaces:**
- Consumes: `StatusDot`, `Badge`, `tokens.ts` maps, `statusTone`; `Avatar as AvatarPrimitive` from `radix-ui`; `X` from `lucide-react`. Wraps: Chip is a plain `<button>`/`<span>`; Avatar wraps Radix Avatar.
- Produces:
```ts
// Chip.tsx — the board's status chips, filter chips, stepper chips and attention-strip chips.
export type ChipProps = Omit<ComponentProps<'button'>, 'onClick'> & {
  tone?: Tone; stage?: StageToken; swatch?: Swatch; status?: AgentStatus;   // precedence: status (tone = statusTone(status), leading StatusDot) > swatch > stage > tone
  size?: 'sm' | 'md';
  selected?: boolean;                 // filter chips: aria-pressed; selected renders the 'solid' fill, otherwise 'soft'
  count?: number;                     // trailing count
  icon?: ReactNode;                   // leading icon when no status dot
  onClick?: MouseEventHandler<HTMLButtonElement>;
  onRemove?(): void; removeLabel?: string;   // trailing ✕ IconButton-like control, label default 'Remove'
};
export function Chip(props: ChipProps): ReactElement;
// Renders <button type="button" aria-pressed={selected}> when onClick or selected is given, otherwise <span>. Label text is 'truncate' with title = string children.

// Avatar.tsx
export type AvatarProps = ComponentProps<typeof AvatarPrimitive.Root> & {
  name: string; src?: string; size?: 'xs' | 'sm' | 'md' | 'lg';   // 16 / 20 / 24 / 32 px
  swatch?: Swatch; shape?: 'circle' | 'square'; status?: AgentStatus;   // status renders a StatusDot overlay at the bottom-right
};
export function Avatar(props: AvatarProps): ReactElement;        // Fallback shows initials(name); role="img" aria-label={name}
export function initials(name: string): string;                  // first letters of the first two words (split on spaces and hyphens), uppercased; one word → its first two letters
export type AvatarGroupProps = ComponentProps<'div'> & { max?: number; size?: AvatarProps['size'] };   // overlapping row, "+N" Badge past max (default 3)
export function AvatarGroup(props: AvatarGroupProps): ReactElement;
```

- [ ] **Step 1: Write the failing tests**

`Chip.test.tsx`:
```tsx
test('working chip renders a pulsing dot and the ok tone', () => { render(<Chip status="working">Working</Chip>); expect(screen.getByRole('img', { name: 'Working' })).toHaveAttribute('data-pulse', 'true'); expect(screen.getByText('Working').closest('[data-chip]')).toHaveClass('text-ok'); });
test('needs-you chip is amber and idle chip is grey', () => {
  render(<><Chip status="needs-you">Plan written, go?</Chip><Chip status="idle">Idle</Chip></>);
  expect(screen.getByText('Plan written, go?').closest('[data-chip]')).toHaveClass('text-warn'); expect(screen.getByText('Idle').closest('[data-chip]')).toHaveClass('text-mute');
});
test('is a pressable button when selectable', async () => {
  const onClick = vi.fn(); render(<Chip selected={false} onClick={onClick}>Needs you</Chip>); const b = screen.getByRole('button', { name: /Needs you/ });
  expect(b).toHaveAttribute('aria-pressed', 'false'); await setupUser().click(b); expect(onClick).toHaveBeenCalledTimes(1);
});
test('selected uses the solid fill', () => { render(<Chip stage="build" selected onClick={() => {}}>Build</Chip>); expect(screen.getByRole('button')).toHaveClass('bg-stage-build', 'text-accent-ink'); });
test('is static when not interactive', () => { render(<Chip>Spec</Chip>); expect(screen.queryByRole('button')).toBeNull(); });
test('renders the count', () => { render(<Chip count={3} onClick={() => {}}>api</Chip>); expect(screen.getByText('3')).toBeInTheDocument(); });
test('remove control fires onRemove and not onClick', async () => {
  const onClick = vi.fn(); const onRemove = vi.fn(); render(<Chip onClick={onClick} onRemove={onRemove}>TC-42</Chip>);
  await setupUser().click(screen.getByRole('button', { name: 'Remove' })); expect(onRemove).toHaveBeenCalledTimes(1); expect(onClick).not.toHaveBeenCalled();
});
test('truncates long labels and exposes the full text as title', () => {
  const long = 'I could not find the test command; which script should I run for the integration suite in this repository?';
  render(<Chip status="needs-you">{long}</Chip>); const label = screen.getByText(long); expect(label).toHaveClass('truncate'); expect(label).toHaveAttribute('title', long);
});
```
`Avatar.test.tsx`:
```tsx
test.each([['frontend', 'FR'], ['best-practice reviewer', 'BR'], ['Johnny Nguyen', 'JN'], ['codebase-consistency reviewer', 'CR']])('initials(%s) → %s', (n, i) => expect(initials(n)).toBe(i));
test('renders initials when there is no image', () => { render(<Avatar name="security reviewer" />); expect(screen.getByRole('img', { name: 'security reviewer' })).toHaveTextContent('SR'); });
test('status renders a dot overlay', () => { render(<Avatar name="frontend" status="working" />); expect(screen.getByRole('img', { name: 'Working' })).toBeInTheDocument(); });
test('AvatarGroup shows +N past max', () => { render(<AvatarGroup max={2}><Avatar name="a" /><Avatar name="b" /><Avatar name="c" /><Avatar name="d" /></AvatarGroup>); expect(screen.getByText('+2')).toBeInTheDocument(); expect(screen.getAllByRole('img')).toHaveLength(2); });
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `npx vitest run src/components/ui/Chip.test.tsx src/components/ui/Avatar.test.tsx`
Expected: FAIL — `Failed to resolve import "./Chip"`.

- [ ] **Step 3: Implement both files**

Chip root carries `data-chip` and the classes `inline-flex max-w-full items-center gap-1 rounded-full border border-transparent px-2 h-5 text-xs` (`md`: `h-6 text-sm`), fill from the maps; the label is a `<span className="truncate" title=…>`; the remove control is a nested `<button type="button" aria-label={removeLabel}>` that stops propagation — a button inside a button is invalid, so when `onRemove` is set the root is a `<span role="group">` and the main click target becomes an inner `<button>`. `Avatar` uses `AvatarPrimitive.Root/Image/Fallback` with `delayMs={0}`, sizes `size-4 size-5 size-6 size-8`, fallback `swatchClasses[swatch].soft` or `bg-well text-mute`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: same command — Expected: `Tests  15 passed (15)`.

- [ ] **Step 5: Example pages and registry**

`ChipPage` (`<h1>Chip</h1>`, the three status chips with the spec's hand-back texts *Spec ready*, *Plan written, go?*, *Build done*, *Message drafted*, *Replied: …* (long), *Permission needed*, *Error*; the seven stage chips as a stepper with done ones `tone="ok"` and the current one solid; filter chips with counts; removable Jira chips; swatch chips for the attention strip), `AvatarPage` (`<h1>Avatar</h1>`, the five roster roles at four sizes, with status, square shape, AvatarGroup overflow). `PRIMITIVES` gains `Chip: ['Chip'], Avatar: ['Avatar', 'AvatarGroup', 'initials']`.

Run: `npx vitest run src/dev/primitives/registry.test.tsx` — Expected: `Tests  3 passed (3)`.

- [ ] **Step 6: Commit**

```bash
git add src/components/ui src/dev/primitives
git commit -m "feat(ui): add Chip and Avatar primitives"
```

---

### Task 15: `EmptyState` (dotLottie, lazy)

**Files:**
- Create: `src/components/ui/EmptyState.tsx`, `EmptyStateLottie.tsx`, `EmptyState.test.tsx`, `src/dev/primitives/pages/EmptyStatePage.tsx`, `src/dev/primitives/assets/empty-column.lottie` (any small CC0 dotLottie placeholder; the real assets are Plan 07's and Plan 06's)
- Modify: `src/components/ui/index.ts`, `src/dev/primitives/registry.ts`, `src/vite-env.d.ts` (declare `*.lottie?url` as `string` if Vite's `?url` typing does not already cover it)

**Interfaces:**
- Consumes: `useReducedMotion`; `DotLottieReact`, `setWasmUrl` from `@lottiefiles/dotlottie-react` (no Radix primitive); the renderer WASM `@lottiefiles/dotlottie-web/dist/dotlottie-player.wasm?url`.
- Produces:
```ts
export type EmptyStateProps = ComponentProps<'div'> & {
  title: ReactNode; description?: ReactNode;
  animation?: string;                 // URL of a .lottie file: import art from './x.lottie?url'
  icon?: ReactNode;                   // shown when there is no animation
  action?: ReactNode;                 // a Button, supplied by the caller
  size?: 'sm' | 'md' | 'lg';          // art 64 / 120 / 180 px — column, board, first run
};
export function EmptyState(props: EmptyStateProps): ReactElement;   // title is an <h2>; art area is a fixed-size box so the lazy chunk never shifts layout
// EmptyStateLottie.tsx (separate chunk, loaded with React.lazy only when `animation` is set):
export default function EmptyStateLottie(props: { src: string; size: number; reduced: boolean }): ReactElement;   // calls setWasmUrl(bundledWasmUrl) once at module scope; <DotLottieReact src loop={!reduced} autoplay={!reduced} />
```

- [ ] **Step 1: Write the failing test**

`EmptyState.test.tsx`, mocking the chunk: `vi.mock('./EmptyStateLottie', () => ({ default: (p: { src: string; reduced: boolean }) => <div data-testid="lottie-stub" data-src={p.src} data-reduced={String(p.reduced)} /> }))`:
```tsx
test('renders title, description and action', () => {
  render(<EmptyState title="No tasks in Build" description="Drag a card here or press Start build." action={<Button>New task</Button>} />);
  expect(screen.getByRole('heading', { level: 2, name: 'No tasks in Build' })).toBeInTheDocument(); expect(screen.getByText(/Drag a card/)).toBeInTheDocument(); expect(screen.getByRole('button', { name: 'New task' })).toBeInTheDocument();
});
test('renders the icon and no Lottie when there is no animation', () => { render(<EmptyState title="Empty" icon={<svg data-testid="icon" />} />); expect(screen.getByTestId('icon')).toBeInTheDocument(); expect(screen.queryByTestId('lottie-stub')).toBeNull(); });
test('loads the Lottie chunk lazily when animation is set', async () => { render(<EmptyState title="Empty" animation="/art/empty.lottie" />); expect(await screen.findByTestId('lottie-stub')).toHaveAttribute('data-src', '/art/empty.lottie'); });
test('reduced motion renders the animation paused', async () => { mockMatchMedia(true); render(<EmptyState title="Empty" animation="/art/empty.lottie" />); expect(await screen.findByTestId('lottie-stub')).toHaveAttribute('data-reduced', 'true'); });
test('art box has a fixed size per size prop', () => { const { container } = render(<EmptyState title="x" size="sm" animation="/a.lottie" />); expect(container.querySelector('[data-art]')).toHaveClass('size-16'); });
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `npx vitest run src/components/ui/EmptyState.test.tsx`
Expected: FAIL — `Failed to resolve import "./EmptyState"`.

- [ ] **Step 3: Implement both files**

`EmptyState`: `flex flex-col items-center text-center gap-2 p-6 text-mute`; `<div data-art className={cx('size-16' | 'size-30' | 'size-45')}>` holds `<Suspense fallback={null}><EmptyStateLottie … /></Suspense>` or the icon. `EmptyStateLottie` imports the WASM with `?url` and calls `setWasmUrl` at module scope so the renderer is served from the bundle, never fetched.

- [ ] **Step 4: Run the test to verify it passes**

Run: `npx vitest run src/components/ui/EmptyState.test.tsx` — Expected: `Tests  5 passed (5)`.

- [ ] **Step 5: Example page, registry and a manual check**

`EmptyStatePage` (`<h1>EmptyState</h1>`, three sizes with the placeholder `.lottie`, icon-only variant, with action). `PRIMITIVES` gains `EmptyState: ['EmptyState']`. Then `npm run dev`, open `#/dev/primitives/EmptyState`: the animation plays, and the Network panel shows the `.wasm` served from `localhost`, not a CDN. Under macOS *Reduce motion* the first frame shows still.

Run: `npx vitest run src/dev/primitives/registry.test.tsx` — Expected: `Tests  3 passed (3)`.

- [ ] **Step 6: Commit**

```bash
git add src/components/ui src/dev/primitives src/vite-env.d.ts
git commit -m "feat(ui): add EmptyState primitive with lazy dotLottie"
```

---

### Task 16: `Panel` and `Resizable`

**Files:**
- Create: `src/components/ui/Panel.tsx`, `Panel.test.tsx`, `Resizable.tsx`, `Resizable.test.tsx`, `src/dev/primitives/pages/PanelPage.tsx`, `pages/ResizablePage.tsx`
- Modify: `src/components/ui/index.ts`, `src/dev/primitives/registry.ts`

**Interfaces:**
- Consumes: `cx`, `focusRing`. Wraps: plain elements for Panel; Resizable is hand-rolled (see below).
- Produces:
```ts
// Panel.tsx — the surface primitive for dock, inspector, sidebar sections and cards.
export type PanelProps = ComponentProps<'section'> & {
  variant?: 'card' | 'well' | 'bar' | 'plain';   // card: bg-card border border-line rounded-card shadow-card; well: bg-well rounded-well; bar: bg-bar border-b border-line; plain: nothing
  padding?: 'none' | 'sm' | 'md';                // p-0 / p-2 / p-4; default 'sm'
  as?: 'section' | 'div' | 'aside' | 'nav';      // default 'section'
};
export function Panel(props: PanelProps): ReactElement;
export type PanelHeaderProps = ComponentProps<'header'> & { title?: ReactNode; actions?: ReactNode; sticky?: boolean };   // h-8 flex items-center justify-between; title text-ink text-sm font-medium
export function PanelHeader(props: PanelHeaderProps): ReactElement;
export function PanelBody(props: ComponentProps<'div'> & { scroll?: boolean }): ReactElement;   // scroll → min-h-0 overflow-auto
export function PanelFooter(props: ComponentProps<'footer'>): ReactElement;

// Resizable.tsx — one split with a draggable, keyboard-operable handle. No Radix equivalent; hand-rolled on the ARIA separator pattern.
// Maps 1:1 onto LayoutNode::Split { direction, ratio }: direction 'horizontal' = first | second side by side, 'vertical' = first above second.
export type ResizableProps = Omit<ComponentProps<'div'>, 'children'> & {
  direction: 'horizontal' | 'vertical';
  ratio: number; onRatioChange(ratio: number): void;   // share of `first`, clamped to [min, 1 − min]
  min?: number;                                        // default 0.1
  step?: number;                                       // keyboard step, default 0.05; Shift doubles it
  first: ReactNode; second: ReactNode;
  handleLabel?: string;                                // aria-label on the separator, default 'Resize'
};
export function Resizable(props: ResizableProps): ReactElement;
// Handle: <div role="separator" tabIndex={0} aria-orientation={direction === 'horizontal' ? 'vertical' : 'horizontal'} aria-valuenow={Math.round(clamped * 100)} aria-valuemin aria-valuemax>
// Keys: horizontal — ArrowRight grows first, ArrowLeft shrinks; vertical — ArrowDown grows, ArrowUp shrinks; Home → min; End → 1 − min; Enter → 0.5.
// Pointer: pointerdown captures, pointermove sets ratio from (pointer − containerStart) / containerSize, pointerup releases. Layout via CSS grid template from the clamped ratio (sizes only, no colours).
```

- [ ] **Step 1: Write the failing tests**

`Panel.test.tsx`:
```tsx
test('card variant renders card tokens and radius', () => { render(<Panel variant="card" aria-label="p" />); expect(screen.getByLabelText('p')).toHaveClass('bg-card', 'border-line', 'rounded-card'); });
test('well variant', () => { render(<Panel variant="well" aria-label="p" />); expect(screen.getByLabelText('p')).toHaveClass('bg-well', 'rounded-well'); });
test('header renders title and actions', () => { render(<PanelHeader title="Changes" actions={<Button>Tidy</Button>} />); expect(screen.getByText('Changes')).toBeInTheDocument(); expect(screen.getByRole('button', { name: 'Tidy' })).toBeInTheDocument(); });
test('as renders the given element', () => { render(<Panel as="aside" />); expect(screen.getByRole('complementary')).toBeInTheDocument(); });
```
`Resizable.test.tsx` with `const split = (p: Partial<ResizableProps>) => render(<Resizable direction="horizontal" ratio={0.5} onRatioChange={vi.fn()} first={<div>A</div>} second={<div>B</div>} {...p} />)`:
```tsx
test('renders both panes and a separator carrying the ratio', () => { split({ ratio: 0.3 }); expect(screen.getByText('A')).toBeInTheDocument(); expect(screen.getByText('B')).toBeInTheDocument(); expect(screen.getByRole('separator', { name: 'Resize' })).toHaveAttribute('aria-valuenow', '30'); });
test('ArrowRight grows the first pane by one step', async () => { const on = vi.fn(); split({ onRatioChange: on }); screen.getByRole('separator').focus(); await setupUser().keyboard('{ArrowRight}'); expect(on).toHaveBeenCalledWith(0.55); });
test('Shift+ArrowRight grows by two steps', async () => { const on = vi.fn(); split({ onRatioChange: on }); screen.getByRole('separator').focus(); await setupUser().keyboard('{Shift>}{ArrowRight}{/Shift}'); expect(on).toHaveBeenCalledWith(0.6); });
test('ArrowUp shrinks the first pane of a vertical split', async () => { const on = vi.fn(); split({ direction: 'vertical', onRatioChange: on }); screen.getByRole('separator').focus(); await setupUser().keyboard('{ArrowUp}'); expect(on).toHaveBeenCalledWith(0.45); });
test('Home and End clamp to min and 1 − min', async () => { const on = vi.fn(); split({ min: 0.2, onRatioChange: on }); screen.getByRole('separator').focus(); const u = setupUser(); await u.keyboard('{Home}'); expect(on).toHaveBeenLastCalledWith(0.2); await u.keyboard('{End}'); expect(on).toHaveBeenLastCalledWith(0.8); });
test('Enter resets to one half', async () => { const on = vi.fn(); split({ ratio: 0.3, onRatioChange: on }); screen.getByRole('separator').focus(); await setupUser().keyboard('{Enter}'); expect(on).toHaveBeenCalledWith(0.5); });
test('dragging the handle sets the ratio from the pointer position', () => {
  const on = vi.fn(); const { container } = split({ onRatioChange: on });
  vi.spyOn(container.firstElementChild as HTMLElement, 'getBoundingClientRect').mockReturnValue({ left: 0, top: 0, width: 400, height: 0, right: 400, bottom: 0, x: 0, y: 0, toJSON() {} } as DOMRect);
  const h = screen.getByRole('separator'); fireEvent.pointerDown(h, { pointerId: 1, clientX: 200 }); fireEvent.pointerMove(h, { pointerId: 1, clientX: 100 }); expect(on).toHaveBeenLastCalledWith(0.25);
  fireEvent.pointerMove(h, { pointerId: 1, clientX: 0 }); expect(on).toHaveBeenLastCalledWith(0.1); fireEvent.pointerUp(h, { pointerId: 1 });
});
test('clamps an out-of-range ratio for ARIA and steps from the clamped value', async () => {
  const on = vi.fn(); split({ ratio: 1.4, onRatioChange: on }); const h = screen.getByRole('separator'); expect(h).toHaveAttribute('aria-valuenow', '90'); h.focus(); await setupUser().keyboard('{ArrowLeft}'); expect(on).toHaveBeenCalledWith(0.85);
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `npx vitest run src/components/ui/Panel.test.tsx src/components/ui/Resizable.test.tsx`
Expected: FAIL — `Failed to resolve import "./Panel"`.

- [ ] **Step 3: Implement both files**

`Resizable` root: `grid h-full w-full min-h-0 min-w-0` with `style={{ gridTemplateColumns: \`${r}fr 1px ${1 - r}fr\` }}` (or rows); the handle is a 1 px `bg-line` track with a wider invisible hit area (`before:absolute before:-inset-x-1`) and `hover:bg-accent focus-visible:bg-accent`, `cursor-col-resize` / `cursor-row-resize`. Round emitted ratios to 3 decimals so `0.55` compares exactly.

- [ ] **Step 4: Run the tests to verify they pass**

Run: same command — Expected: `Tests  12 passed (12)`.

- [ ] **Step 5: Example pages and registry**

`PanelPage` (`<h1>Panel</h1>`, four variants, header with actions, scrolling body, nested panels on a `bg-page` background in both themes), `ResizablePage` (`<h1>Resizable</h1>`, a horizontal split, a vertical split with a 2/3 "dock" ratio, a nested tree mirroring a `LayoutNode`, ratio shown live). `PRIMITIVES` gains `Panel: ['Panel', 'PanelHeader', 'PanelBody', 'PanelFooter'], Resizable: ['Resizable']`.

Run: `npx vitest run src/dev/primitives/registry.test.tsx` — Expected: `Tests  3 passed (3)`.

- [ ] **Step 6: Commit**

```bash
git add src/components/ui src/dev/primitives
git commit -m "feat(ui): add Panel and Resizable primitives"
```

---

### Task 17: Finish — barrel pin, lint boundary conformance, gallery toolbar, docs

**Files:**
- Create: `src/components/ui/lintBoundary.test.ts`, `src/components/ui/index.test.ts`
- Modify: `src/components/ui/index.ts` (final member order), `src/dev/primitives/Gallery.tsx` (theme toolbar), `ARCHITECTURE.md` (primitives section), `.claude/rules/ui.md` (one line: how to add a primitive)

**Interfaces:**
- Consumes: Plan 01's `eslint.config.js` with the raw-control and hex-colour rules; the complete barrel.
- Produces: the pinned `PRIMITIVES` key order (the spec's §15.3 order) and the guarantee, tested from this side, that the lint boundary holds.

- [ ] **Step 1: Write the failing tests**

`index.test.ts`:
```ts
test('PRIMITIVES names the twenty-six primitives of spec §15.3 in order', () => {
  expect(Object.keys(PRIMITIVES)).toEqual(['Button', 'IconButton', 'Input', 'Textarea', 'Select', 'Combobox', 'Checkbox', 'Switch', 'Segmented', 'Tabs', 'Menu', 'ContextMenu',
    'Popover', 'Tooltip', 'Dialog', 'Drawer', 'Toast', 'Field', 'Chip', 'Badge', 'Avatar', 'KeyHint', 'StatusDot', 'EmptyState', 'Panel', 'Resizable']);
});
test('the barrel also exports the shared toolkit', () => { for (const m of ['cx', 'focusRing', 'UiProvider', 'STAGES', 'statusTone', 'statusLabel', 'toneClasses', 'stageClasses', 'swatchClasses']) expect(ui).toHaveProperty(m); });
```
`lintBoundary.test.ts` (runs ESLint in-process; `const eslint = new ESLint({ cwd: process.cwd(), overrideConfig: { languageOptions: { parserOptions: { projectService: false, project: null } } } })`, `const src = 'export const Probe = () => <button type="button">x</button>;'`):
```ts
test('a raw <button> outside src/components/ui is a lint error', async () => {
  const [r] = await eslint.lintText(src, { filePath: 'src/features/probe/Probe.tsx' }); expect(r.messages.some((m) => m.severity === 2 && /button/i.test(m.message))).toBe(true);
});
test('the same element inside src/components/ui passes', async () => {
  const [r] = await eslint.lintText(src, { filePath: 'src/components/ui/Probe.tsx' }); expect(r.messages.filter((m) => /button/i.test(m.message))).toHaveLength(0);
});
test('a hex colour outside the theme file is a lint error', async () => {
  const [r] = await eslint.lintText('export const c = <div className="bg-[#fff]" />;', { filePath: 'src/features/probe/Probe.tsx' }); expect(r.messages.some((m) => m.severity === 2 && /#fff|hex|colou?r/i.test(m.message))).toBe(true);
});
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `npx vitest run src/components/ui/index.test.ts src/components/ui/lintBoundary.test.ts`
Expected: `index.test.ts` FAILS on key order (the record was built in task order); `lintBoundary.test.ts` passes if Plan 01's rule is in place — if it fails, the rule is missing or misnamed and that is a Plan 01 defect to raise, not to patch here.

- [ ] **Step 3: Reorder `PRIMITIVES` into the spec order** (the `export *` lines may stay in any order).

- [ ] **Step 4: Run the tests to verify they pass**

Run: `npx vitest run src/components/ui/index.test.ts src/components/ui/lintBoundary.test.ts` — Expected: `Tests  5 passed (5)`.

- [ ] **Step 5: Gallery toolbar and docs**

Add a toolbar to `Gallery.tsx`: a `Segmented aria-label="Theme"` with `light | dark` that sets `document.documentElement.dataset.theme`, and a `Switch label="Dense"` toggling `text-xs` on the page area. Append to `ARCHITECTURE.md` a short "UI primitives" section: the folder, the barrel, `PRIMITIVES`, `UiProvider` at the root, `src/lib/motion.ts`, the gallery route, and the rule that colour-by-prop goes through `tokens.ts`. Add to `.claude/rules/ui.md` one line: "To add a primitive: `src/components/ui/<Name>.tsx` + `<Name>.test.tsx`, export it from `index.ts`, add it to `PRIMITIVES`, add `src/dev/primitives/pages/<Name>Page.tsx` and register it; `registry.test.tsx` fails until all four exist."

- [ ] **Step 6: Run the quality gates**

Run: `npm run typecheck && npm run lint && npx vitest run && npm run build`
Expected: each exits 0; vitest prints `Test Files  32 passed (32)` (the count may be higher if Plan 01 added tests) with no skipped tests; the build output contains no `Primitives gallery` string (`grep -rl "Primitives gallery" dist/ || echo "not in bundle"` → `not in bundle`). Open `#/dev/primitives` in `npm run dev`, switch theme to dark, and walk every page: no unstyled element, no hex colour in DevTools computed styles that is not a token value.

- [ ] **Step 7: Commit**

```bash
git add src/components/ui src/dev/primitives ARCHITECTURE.md .claude/rules/ui.md
git commit -m "feat(ui): pin the primitive set, lint boundary test and gallery toolbar"
```

---

## Proposed contract additions

Names this plan introduces that other plans will use; the overview should adopt or amend them before execution.

1. **Theme tokens beyond §11.1** (defined by Plan 01 in `src/styles.css`, read here): `--color-accent-ink` (text on `accent`, stage and swatch solid fills; `#ffffff` in both themes); `--color-swatch-{violet,indigo,cyan,mint,lime,gold,coral,magenta}` — the shared task and project colour palette, distinct from the stage colours, so `TaskColor` and `ProjectColor` strings are these eight names; `--radius-card: 5px` and `--radius-well: 6px` (`rounded-card`, `rounded-well`); `--shadow-card` (1 px shadow in light, `none` in dark). Statement to pin: every colour token is a Tailwind v4 `@theme` colour, usable with any colour utility prefix (`bg-`, `text-`, `border-`, `ring-`, `fill-`) and opacity modifier.
2. **Shared unions exported from `src/components/ui`**: `Size`, `Tone`, `StageToken` (lowercase stage names matching the `stage-*` tokens), `Swatch`, `AgentStatus = 'working' | 'needs-you' | 'idle'`, with `STAGES`, `statusTone`, `statusLabel`. Consumers map the bindings' `Stage` and `AgentState` to these at the feature boundary (one helper in `src/lib/format.ts`, owned by whichever plan first needs it — suggested Plan 07).
3. **`src/lib/motion.ts` is owned by Plan 02**: `REDUCED_MOTION_QUERY`, `prefersReducedMotion()`, `useReducedMotion()`, `SPRING`, `EASE_REDUCED`, `transitionFor(reduced, full?)`, `GLOW_MS = 400`, `REFLOW_MS = 300`. Plans 07 and 09 pass `transitionFor(useReducedMotion())` to Motion.
4. **`UiProvider`** (`TooltipProvider` + Sonner `Toaster`) is mounted once at each window's root by Plan 06's `App.tsx`; `toast` from `src/components/ui` is the only toast API (the pipeline's `Effect::Toast(String)` ends in a `toast(...)` call).
5. **Shared test harness**: `src/test/setup.ts` (jsdom gaps, jest-dom, cleanup) and `src/test/ui.ts` (`mockMatchMedia`, `setupUser`, `renderUi`) are the Vitest setup for every webview test; `vite.config.ts` `test.setupFiles` points at the former.
6. **CSP**: the dotLottie renderer is WebAssembly compiled in the webview, so `tauri.conf.json` needs `script-src 'self' 'wasm-unsafe-eval'` alongside `default-src 'self'` (Plan 01's file; recorded in the ADR). The `.wasm` is bundled through Vite `?url`, never fetched.
7. **`src/styles.css`** imports `tw-animate-css` and `./components/ui/primitives.css` (keyframes and the Vaul reduced-motion override; no colours).
8. **`Resizable` ↔ `LayoutNode::Split`**: `direction` is the lowercase `SplitDirection`, `ratio` is `Split.ratio`; Plan 06 renders a split tree by nesting `Resizable`, Plan 08 uses a vertical one for the dock (`ratio` 2/3 when the Team tab grows).
9. **Chord syntax for `KeyHint.formatChord`**: `'cmd+shift+a'` style tokens; Plan 06's `src/lib/keyboard.ts` should emit this form so menus, tooltips and the palette render shortcuts through `KeyHint`. Output order is ⌘ ⌃ ⌥ ⇧ (the spec's copy, not Apple's menu order).
10. **Ownership note**: the overview's plan table lists "the lint rule that forbids raw controls" under Plan 02, while the brief for this plan has Plan 01 shipping the ESLint rule. This plan consumes the rule and pins its behaviour from the primitives' side (Task 17 `lintBoundary.test.ts`); the overview row should drop the rule from 02.
11. **New dependencies** (ADR `docs/adr/2026-10-24-ui-primitive-dependencies.md`): `radix-ui`, `cmdk`, `sonner`, `vaul`, `motion`, `@lottiefiles/dotlottie-react`, `clsx`, `tailwind-merge`, `class-variance-authority`, `lucide-react`, `tw-animate-css`; dev: `@testing-library/react`, `@testing-library/user-event`, `@testing-library/jest-dom`, `jsdom`.

# ADR: Dependencies of the UI primitives

- **Status:** accepted 2026-10-25

## Context

The spec (§3) names shadcn/ui on Radix, Motion, Sonner, Vaul and dotLottie for the webview. shadcn/ui is a pattern, not
a package: its generated components import a small set of helpers, and the primitives in `src/components/ui/` (§15.3)
follow that pattern with thin, token-styled wrappers. Render and keyboard tests need a DOM and Testing Library.

## Decision

Add these runtime packages: `radix-ui` (the unified package current shadcn/ui uses), `cmdk` (the Combobox list),
`sonner` (Toast), `vaul` (Drawer), `motion`, `@lottiefiles/dotlottie-react` (EmptyState art), `clsx`, `tailwind-merge`
and `class-variance-authority` (the `cx` helper and variants), `lucide-react` (icons) and `tw-animate-css` (the
open/close animation utilities Radix content uses).

Add these dev packages: `@testing-library/user-event` and `@testing-library/jest-dom` (with the `@testing-library/react`,
`@testing-library/dom` and `jsdom` that Plan 01 already installed).

The dotLottie renderer is WebAssembly. The `.wasm` file is bundled through a Vite `?url` import and handed to
`setWasmUrl`, so it is served from the app bundle and never fetched from a CDN. The webview must therefore compile
WebAssembly, which needs `script-src 'self' 'wasm-unsafe-eval'` in the Tauri CSP. Plan 01 already ships that CSP
(overview amendment A9).

## Consequences

- One Radix version (`npm ls radix-ui` prints one) instead of a package per primitive.
- Colour never enters through these packages: every class goes through the tokens in `src/styles.css`.
- The dotLottie chunk loads only when an `EmptyState` has an animation, so the first paint does not pay for WASM.
- `'wasm-unsafe-eval'` is the one CSP relaxation; it allows WebAssembly compilation, not string `eval`.

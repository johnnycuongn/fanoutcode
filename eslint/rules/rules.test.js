import tsParser from "@typescript-eslint/parser";
import { RuleTester } from "eslint";
import { describe, it } from "vitest";
import featureBoundaries from "./featureBoundaries.js";
import noColourLiterals from "./noColourLiterals.js";
import noManualIpc from "./noManualIpc.js";
import noRawControls from "./noRawControls.js";

RuleTester.describe = describe;
RuleTester.it = it;
const ruleTester = new RuleTester({ languageOptions: { parser: tsParser } });
const tsx = { languageOptions: { parser: tsParser, parserOptions: { ecmaFeatures: { jsx: true } } } };

ruleTester.run("no-raw-controls", noRawControls, {
  valid: [
    { code: "<Button/>", filename: "/r/src/features/board/components/Card.tsx", ...tsx },
    { code: "<button/>", filename: "/r/src/components/ui/Button.tsx", ...tsx },
    { code: "<input/>", filename: "/r/src/components/ui/Input.tsx", ...tsx },
  ],
  invalid: ["button", "input", "select", "textarea", "dialog"].map((tag) => ({
    code: `<${tag}/>`,
    filename: "/r/src/features/board/components/Card.tsx",
    ...tsx,
    errors: [{ messageId: "raw" }],
  })),
});

ruleTester.run("no-colour-literals", noColourLiterals, {
  valid: [
    { code: 'const c = "bg-page text-ink";', filename: "/r/src/a.ts" },
    { code: 'const id = "#root";', filename: "/r/src/a.ts" }, // not a colour: 4 letters, not hex
    { code: "<div style={{ width: 4 }}/>", filename: "/r/src/a.tsx", ...tsx },
  ],
  invalid: [
    { code: 'const c = "#fff";', filename: "/r/src/a.ts", errors: [{ messageId: "colour" }] },
    { code: "const c = `border: 1px solid #e2e2e6`;", filename: "/r/src/a.ts", errors: [{ messageId: "colour" }] },
    {
      code: "<div style={{ color: tokens.ink }}/>",
      filename: "/r/src/a.tsx",
      ...tsx,
      errors: [{ messageId: "colour" }],
    },
    { code: '<div style={{ "--x": v }}/>', filename: "/r/src/a.tsx", ...tsx, errors: [{ messageId: "colour" }] },
    { code: 'const c = "#1f9d55";', filename: "/r/src/components/ui/Button.tsx", errors: [{ messageId: "colour" }] },
  ],
});

ruleTester.run("feature-boundaries", featureBoundaries, {
  valid: [
    { code: 'import { Board } from "@/features/board";', filename: "/r/src/features/shell/components/Tabs.tsx" },
    { code: 'import { cards } from "../store";', filename: "/r/src/features/board/components/Card.tsx" },
    { code: 'import { createStore } from "@/lib/stores/createStore";', filename: "/r/src/features/board/store.ts" },
    { code: 'import { Board } from "@/features/board/index";', filename: "/r/src/App.tsx" },
  ],
  invalid: [
    {
      code: 'import { boardStore } from "@/features/board/store";',
      filename: "/r/src/features/shell/store.ts",
      errors: [{ messageId: "boundary" }],
    },
    {
      // The plan's `../board/…` from shell/components/ resolves inside shell; `../../board/…` reaches the board feature.
      code: 'import { Card } from "../../board/components/Card";',
      filename: "/r/src/features/shell/components/Tabs.tsx",
      errors: [{ messageId: "boundary" }],
    },
    {
      code: 'export { boardStore } from "@/features/board/store";',
      filename: "/r/src/App.tsx",
      errors: [{ messageId: "boundary" }],
    },
  ],
});

ruleTester.run("no-manual-ipc", noManualIpc, {
  valid: [
    { code: 'import { invoke } from "@tauri-apps/api/core";', filename: "/r/src/lib/bindings.ts" },
    { code: 'import { commands } from "@/lib/bindings";', filename: "/r/src/features/x/store.ts" },
    { code: 'import { getCurrentWindow } from "@tauri-apps/api/window";', filename: "/r/src/lib/windowId.ts" },
    // A Channel is how a caller receives a generated command's stream; it is not hand-written IPC.
    { code: 'import { Channel } from "@tauri-apps/api/core";', filename: "/r/src/features/views/store.ts" },
  ],
  invalid: [
    {
      code: 'import { invoke } from "@tauri-apps/api/core";',
      filename: "/r/src/features/x/store.ts",
      errors: [{ messageId: "manual" }],
    },
    {
      code: 'import { listen } from "@tauri-apps/api/event";',
      filename: "/r/src/lib/events.ts",
      errors: [{ messageId: "manual" }],
    },
    {
      code: 'import { Channel, invoke } from "@tauri-apps/api/core";',
      filename: "/r/src/features/x/store.ts",
      errors: [{ messageId: "manual" }],
    },
    {
      code: 'import * as core from "@tauri-apps/api/core";',
      filename: "/r/src/features/x/store.ts",
      errors: [{ messageId: "manual" }],
    },
  ],
});

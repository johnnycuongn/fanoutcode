// The `teamcode` ESLint plugin: the repository rules that lint enforces (CLAUDE.md, spec §15).
import featureBoundaries from "./featureBoundaries.js";
import noColourLiterals from "./noColourLiterals.js";
import noManualIpc from "./noManualIpc.js";
import noRawControls from "./noRawControls.js";

export default {
  meta: { name: "teamcode" },
  rules: {
    "no-raw-controls": noRawControls,
    "no-colour-literals": noColourLiterals,
    "no-manual-ipc": noManualIpc,
    "feature-boundaries": featureBoundaries,
  },
};

// The spec-editor feature's public surface: the Spec stage page and its store.
export { SpecStagePage } from "./components/SpecStagePage";
export type { SpecStagePageProps } from "./components/SpecStagePage";
export { specStore, createSpecStore, SAVE_FAILED } from "./store";
export type { SpecStore } from "./store";
export { SpecSession, useSpecSession } from "./specSession";
export type { SpecIpc, SpecState, SpecTaskState } from "./types";
export { TerminalStrip } from "./components/TerminalStrip";

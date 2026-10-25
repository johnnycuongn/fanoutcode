// The task page feature's public surface (A5): Plan 06 renders TaskPage for PaneContent::Task and routes the task
// chords to TaskPageHandle; Plan 09 supplies renderTeam and onOpenTeamPane. The action helpers are shared with
// Plan 06's palette and Plan 07's card menu so every surface uses the same labels.
export { TaskPage } from "./components/TaskPage";
export type { TaskPageHandle, TaskPageProps } from "./components/TaskPage";
export { taskPageStore, createTaskPageStore, toTaskSettings } from "./store";
export type { TaskPageStore } from "./store";
export { subscribeTaskPageEvents } from "./events";
export { primaryActionFor, forwardActionFor, freshSessionAllowed, PREVIOUS_STAGE, QUEUED_HINT } from "./primaryAction";
export type { PrimaryButton, PrimaryCtx, PrimaryIntent } from "./primaryAction";
export { bodyFor } from "./body";
export type { BodyKind } from "./body";
export type { TaskSettings, TaskPageIpc, TaskPageState } from "./types";
export type { TaskSource } from "./taskSource";

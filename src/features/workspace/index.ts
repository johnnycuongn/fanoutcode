// The workspace feature's public surface: its stores, the linking copy, the import flow and its components.
export { workspaceStore } from "./store";
export { contextDocsStore, containerKey, containerFromKey } from "./contextDocsStore";
export { importStore, openWorkspaceFile } from "./importStore";
export { pickWorkspaceFile } from "./pickers";
export * from "./linking";
export type * from "./types";
export { WorkspaceSettings } from "./components/WorkspaceSettings";
export type { WorkspaceSettingsProps } from "./components/WorkspaceSettings";
export { ContextDocsEditor } from "./components/ContextDocsEditor";
export { WorkspaceImportDialog } from "./components/WorkspaceImportDialog";
export { useDocRows, addDroppedPath } from "./sidebarDocs";
export type { DocRow } from "./sidebarDocs";
export { jiraStore, issueKey } from "./jiraStore";
export type { JiraScope, JiraState } from "./jiraStore";
export { JiraLinkForm } from "./components/JiraLinkForm";
export { IssuePicker } from "./components/IssuePicker";

// The spec editor's state and command set.
import type { Live } from "@/lib/ipcResult";
import type { LiveSpecLevel } from "@/lib/bindings";

/** The commands the spec store calls (resolving to the value, rejecting with the error string). */
export type SpecIpc = Live<
  | "memoryRead"
  | "memoryWrite"
  | "settingsGet"
  | "settingsSet"
  | "tasksAction"
  | "livespecAnalyze"
  | "livespecCancel"
  | "sessionsOpen"
> &
  Partial<Live<"memoryMissing">>;

export interface SpecTaskState {
  text: string;
  loaded: boolean;
  error: string | null;
  saving: "idle" | "saved" | "failed";
  terminalExpanded: boolean;
  terminalRatio: number;
}

export interface SpecState {
  byTask: Record<string, SpecTaskState>;
  levelByProject: Record<string, LiveSpecLevel>;
}

// The only file in this plan that branches on AgentState (A1: `{ type: "NeedsYou", reason: { type, … } }`). It holds
// no display text: chips read `handbackLabel(task)` from src/lib/format.ts (A2).
import type { AgentState, HandbackReason } from "@/lib/bindings";

export type HandbackKind = HandbackReason["type"];

export function handbackKind(state: AgentState): HandbackKind | null {
  return state.type === "NeedsYou" ? state.reason.type : null;
}

export function isWorking(state: AgentState): boolean {
  return state.type === "Working";
}

export function testExitCode(state: AgentState): number | null {
  return state.type === "NeedsYou" && state.reason.type === "TestFailed" ? state.reason.exit_code : null;
}

/** Same state, compared by value (a task-changed brings a fresh object). */
export function sameState(a: AgentState, b: AgentState): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

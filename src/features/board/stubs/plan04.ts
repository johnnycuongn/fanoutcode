// STUB(plan 04): `sessions_stop` is not in `src/lib/bindings.ts` yet (Plan 04 adds it). When Plan 04 merges, call
// `commands.sessionsStop(sessionId)` from `ipc.ts` and delete this file.
// eslint-disable-next-line teamcode/no-manual-ipc -- STUB(plan 04): not in the generated bindings yet
import { invoke } from "@tauri-apps/api/core";

export async function sessionsStop(sessionId: string): Promise<void> {
  try {
    await invoke("sessions_stop", { sessionId });
  } catch (err) {
    throw err instanceof Error ? err : new Error(String(err));
  }
}

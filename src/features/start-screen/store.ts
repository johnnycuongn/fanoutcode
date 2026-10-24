// The start screen's recent projects and workspaces.
import { commands, type RecentEntry } from "@/lib/bindings";
import { createStore } from "@/lib/stores/createStore";

export const startStore = createStore<{ recents: RecentEntry[]; loading: boolean }>({ recents: [], loading: false });

export async function loadRecents(): Promise<void> {
  startStore.set({ loading: true });
  try {
    const r = await commands.windowsRecent();
    if (r?.status === "ok") startStore.set({ recents: r.data });
  } finally {
    startStore.set({ loading: false });
  }
}

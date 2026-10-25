// The native pickers the workspace screens use (Tauri's dialog plugin); each returns null when cancelled.
import { open, save } from "@tauri-apps/plugin-dialog";

export async function pickFolder(title: string): Promise<string | null> {
  const p = await open({ directory: true, title });
  return typeof p === "string" ? p : null;
}

export async function pickFile(title: string): Promise<string | null> {
  const p = await open({ title });
  return typeof p === "string" ? p : null;
}

export async function pickWorkspaceFile(): Promise<string | null> {
  const p = await open({ title: "Open a workspace", filters: [{ name: "Team Code workspace", extensions: ["json"] }] });
  return typeof p === "string" ? p : null;
}

export async function pickSavePath(defaultPath: string): Promise<string | null> {
  const p = await save({ defaultPath });
  return typeof p === "string" ? p : null;
}

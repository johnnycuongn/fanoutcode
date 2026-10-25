// Turns a generated command's `Result` into its data or a thrown `Error` carrying the shell's message.
import type { Result } from "@/lib/bindings";

export function unwrap<T>(r: Result<T, string>): T {
  if (r.status === "error") throw new Error(r.error);
  return r.data;
}

export const messageOf = (e: unknown): string => (e instanceof Error ? e.message : String(e));

// The generated commands answer `Result<T, string>`; the task page's stores want a call that resolves to the value
// and rejects with the error string (Team Code's `Error` serialises as a plain string), so their command sets can be
// faked with plain `vi.fn`s. `live(...names)` builds such a set over the generated `commands`, looked up at call time.
import { commands, type Result } from "@/lib/bindings";

type Commands = typeof commands;
type Value<R> = [R] extends [Result<infer T, string>] ? T : R;
type Unwrapped<F> = F extends (...args: infer A) => Promise<infer R> ? (...args: A) => Promise<Value<R>> : never;
export type Live<K extends keyof Commands> = { [P in K]: Unwrapped<Commands[P]> };

function isResult(r: unknown): r is Result<unknown, string> {
  if (typeof r !== "object" || r === null || !("status" in r)) return false;
  const { status } = r as { status: unknown };
  return (status === "ok" && "data" in r) || (status === "error" && "error" in r);
}

/** The value of a specta `Result`, or the error string thrown; anything else passes through. */
export function unwrap<T>(r: Result<T, string> | T): T {
  if (!isResult(r)) return r as T;
  if (r.status === "error") throw r.error;
  return r.data as T;
}

export function live<K extends keyof Commands>(...names: K[]): Live<K> {
  const out = {} as Record<K, unknown>;
  for (const name of names) {
    out[name] = async (...args: unknown[]) =>
      unwrap(await (commands[name] as (...a: unknown[]) => Promise<unknown>)(...args));
  }
  return out as Live<K>;
}

/** What a rejected command (or anything thrown) says, for showing verbatim. */
export function errorMessage(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return String(e);
}

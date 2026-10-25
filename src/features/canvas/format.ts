// Canvas display formats: tool-call durations and the local wall-clock time of an event.

export function formatDuration(ms: number | null): string {
  if (ms === null) return "";
  return ms < 1000 ? `${ms} ms` : `${(ms / 1000).toFixed(1)} s`;
}

const pad = (n: number) => String(n).padStart(2, "0");

/** `HH:MM:SS` in local time. */
export function clockTime(at: string): string {
  const d = new Date(at);
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

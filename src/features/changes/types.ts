// Review notes and file references for the Changes tab.
export type Severity = "critical" | "high" | "medium" | "low" | "info";

export interface ReviewNote {
  id: string;
  reviewer: string | null;
  security: boolean;
  location: string;
  line: number;
  severity: Severity;
  text: string;
}

/** `repo` is the repo name; `home` marks the task's home repo, which unprefixed locations belong to. */
export interface FileRef {
  repo: string;
  home: boolean;
  path: string;
}

// The sidebar's types.

/** A context doc row in the Docs section; Plan 10 supplies the list. */
export type ContextDocItem = {
  id: string;
  kind: "file" | "folder" | "url" | "confluence";
  label: string;
  href: string;
};

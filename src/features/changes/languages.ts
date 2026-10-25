// Syntax highlighting for the diff editor, loaded lazily by file name from @codemirror/language-data.
import { LanguageDescription } from "@codemirror/language";
import { languages } from "@codemirror/language-data";
import type { Extension } from "@codemirror/state";

export async function languageFor(path: string): Promise<Extension | null> {
  const name = path.split("/").pop() ?? path;
  const desc = LanguageDescription.matchFilename(languages, name);
  if (!desc) return null;
  try {
    return await desc.load();
  } catch {
    return null;
  }
}

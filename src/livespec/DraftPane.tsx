// The Spec stage's writing surface. Copied from LiveSpec (src/components/DraftPane.tsx) and moved onto the Textarea
// primitive and Team Code's tokens; Literata comes from the theme's `font-serif`.
import { useEffect, useState } from "react";
import { Button, Textarea, cx } from "@/components/ui";
import { copyText } from "./clipboard";

interface Props {
  value: string;
  onChange: (v: string) => void;
  placeholder: string;
  /** Editing is off, e.g. when the memory folder is missing. */
  readOnly?: boolean;
}

// Centres a ~40rem (about 68 characters) column inside the full-width pane, so the whole pane is the click and scroll
// target but lines stay readable. Percent padding resolves against the pane, so textarea and footer line up.
const GUTTER = "px-[max(2rem,calc((100%_-_40rem)/2))]";

const COPY_FAILED = "Couldn't copy. Select the text and copy it instead.";

export const wordCount = (value: string): number => (value.trim() ? value.trim().split(/\s+/).length : 0);

export function DraftPane({ value, onChange, placeholder, readOnly = false }: Props) {
  const words = wordCount(value);
  const [copy, setCopy] = useState<"idle" | "copied" | "failed">("idle");

  // "Copied" and the failure note clear themselves.
  useEffect(() => {
    if (copy === "idle") return;
    const t = setTimeout(() => setCopy("idle"), 2000);
    return () => clearTimeout(t);
  }, [copy]);

  const copySpec = () => {
    void copyText(value).then((ok) => setCopy(ok ? "copied" : "failed"));
  };

  return (
    <section className="flex min-h-0 flex-col bg-page" aria-label="Your spec">
      <Textarea
        aria-label="Spec"
        className={cx(
          "min-h-0 w-full flex-1 resize-none overflow-y-auto rounded-none border-0 bg-transparent pt-10 pb-6",
          "font-serif text-[17px] leading-[1.75] focus-visible:ring-0",
          GUTTER,
        )}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        readOnly={readOnly}
        spellCheck
        placeholder={placeholder}
      />
      <footer className={cx("flex shrink-0 items-center gap-3 pt-1 pb-3 text-xs tabular-nums text-mute", GUTTER)}>
        <span>{words} words</span>
        <Button variant="link" size="sm" onClick={copySpec} disabled={!value.trim()} title="Copy the spec">
          Copy spec
        </Button>
        {copy === "copied" && <span aria-live="polite">Copied</span>}
        {copy === "failed" && (
          <span className="text-bad" role="alert">
            {COPY_FAILED}
          </span>
        )}
      </footer>
    </section>
  );
}

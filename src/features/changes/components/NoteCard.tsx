// One review note under its line: severity, reviewer, the note through MiniMarkdown, and a reply sent to the agent
// as a Reply prompt (A7).
import { useState, type ReactElement } from "react";
import { Badge, Button, Chip, Textarea, type Tone } from "@/components/ui";
import { MiniMarkdown } from "@/livespec/MiniMarkdown";
import type { ReviewNote, Severity } from "../types";

const TONE: Record<Severity, Tone> = { critical: "bad", high: "bad", medium: "warn", low: "neutral", info: "neutral" };

export function NoteCard({
  note,
  clamped,
  onReply,
}: {
  note: ReviewNote;
  clamped: boolean;
  onReply(text: string): Promise<void>;
}): ReactElement {
  const [replying, setReplying] = useState(false);
  const [text, setText] = useState("");
  const [sending, setSending] = useState(false);
  const send = async () => {
    setSending(true);
    try {
      await onReply(text.trim());
      setText("");
      setReplying(false);
    } finally {
      setSending(false);
    }
  };
  return (
    <article className="rounded-card border border-line bg-card p-2 font-sans text-[13px] text-ink shadow-card">
      <header className="mb-1 flex items-center gap-1.5 text-xs text-mute">
        <Chip size="sm" tone={TONE[note.severity]}>
          {note.severity}
        </Chip>
        {note.reviewer && <span>{note.reviewer}</span>}
        {note.security && (
          <Badge size="sm" tone="bad">
            security
          </Badge>
        )}
        {clamped && <span>moved to last line (was line {note.line})</span>}
        <span className="flex-1" />
        <Button variant="ghost" size="sm" onClick={() => setReplying((r) => !r)}>
          Reply
        </Button>
      </header>
      <MiniMarkdown text={note.text} className="text-[13px]" />
      {replying && (
        <div className="mt-2 grid gap-1.5">
          <Textarea
            size="sm"
            autoGrow
            aria-label={`Reply to ${note.location}:${note.line}`}
            value={text}
            onChange={(e) => setText(e.target.value)}
          />
          <div>
            <Button size="sm" variant="primary" disabled={!text.trim()} loading={sending} onClick={() => void send()}>
              Send to agent
            </Button>
          </div>
        </div>
      )}
    </article>
  );
}

// First run: which agent CLIs this Mac can run, so a missing one shows up here rather than as a dead terminal on the
// first task.
import { useEffect, useState, type ReactElement } from "react";
import { Button, Chip } from "@/components/ui";
import { commands, type AgentCli, type AgentKind } from "@/lib/bindings";

const NAMES: Record<AgentKind, string> = { ClaudeCode: "Claude Code", Codex: "Codex", Shell: "Shell" };
const INSTALL: Partial<Record<AgentKind, string>> = {
  ClaudeCode: "npm install -g @anthropic-ai/claude-code",
  Codex: "npm install -g @openai/codex",
};

async function check(): Promise<AgentCli[]> {
  try {
    const r = await commands.agentsCheck();
    return r?.status === "ok" ? r.data : [];
  } catch {
    return [];
  }
}

export function AgentCheck(): ReactElement {
  const [agents, setAgents] = useState<AgentCli[] | null>(null);
  const [round, setRound] = useState(0);
  useEffect(() => {
    let live = true;
    void check().then((a) => live && setAgents(a));
    return () => {
      live = false;
    };
  }, [round]);
  const again = () => {
    setAgents(null);
    setRound((n) => n + 1);
  };
  return (
    <section aria-label="Agents on this Mac" className="flex flex-col gap-2">
      <h3 className="text-sm font-medium">Agents on this Mac</h3>
      {agents === null ? (
        <p className="text-sm text-mute">Checking…</p>
      ) : (
        <ul className="flex flex-col gap-1.5">
          {agents.map((a) => (
            <li key={a.kind} className="flex min-w-0 items-center gap-2 text-sm">
              <span className="w-24 shrink-0 font-medium">{NAMES[a.kind]}</span>
              {a.path ? (
                <Chip size="sm" tone="ok">
                  {a.version ?? "Installed"}
                </Chip>
              ) : (
                <>
                  <Chip size="sm" tone="warn">
                    Not found
                  </Chip>
                  <code className="min-w-0 truncate font-mono text-xs text-mute">{INSTALL[a.kind]}</code>
                </>
              )}
            </li>
          ))}
        </ul>
      )}
      {agents && !agents.some((a) => a.path) && (
        <p className="text-xs text-bad">
          Install Claude Code or Codex to run tasks. You can continue now and install one later.
        </p>
      )}
      <div>
        <Button size="sm" variant="ghost" onClick={again} disabled={agents === null}>
          Check again
        </Button>
      </div>
    </section>
  );
}

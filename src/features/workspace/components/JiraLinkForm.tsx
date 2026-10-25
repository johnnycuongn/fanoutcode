// Links a Jira site for a project or as a workspace's default: site, email and an API token. The token goes to the
// keychain through the shell and is cleared from the form at once; it is never shown or kept.
import { useEffect, useState, type FormEvent, type ReactElement } from "react";
import { Button, Field, Input } from "@/components/ui";
import type { JiraSite } from "@/lib/bindings";
import { jiraStore, type JiraScope } from "../jiraStore";
import { messageOf } from "../result";

export type JiraLinkFormProps = {
  scope: JiraScope;
  /** The site already linked here (for a workspace, from the caller's settings). */
  linked?: JiraSite | null;
  /** The workspace site a project inherits, when it has none of its own. */
  inheritedFrom?: JiraSite | null;
};

export function JiraLinkForm({ scope, linked: given, inheritedFrom }: JiraLinkFormProps): ReactElement {
  const projectId = "project" in scope ? scope.project : null;
  const status = jiraStore.useSelector((s) => (projectId ? s.status[projectId] : undefined));
  const [site, setSite] = useState("");
  const [email, setEmail] = useState("");
  const [token, setToken] = useState("");
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    if (projectId) void jiraStore.loadStatus(projectId).catch(() => undefined);
  }, [projectId]);
  const own = projectId ? (status && !status.inherited ? status.site : null) : (given ?? null);
  const inherited = projectId && status?.inherited ? status.site : (inheritedFrom ?? null);
  const submit = async (e: FormEvent) => {
    e.preventDefault();
    const typed = token;
    setToken("");
    setPending(true);
    setError(null);
    try {
      await jiraStore.link(scope, site, email, typed);
    } catch (err) {
      setError(messageOf(err));
    } finally {
      setPending(false);
    }
  };
  return (
    <form className="flex flex-col gap-2" onSubmit={(e) => void submit(e)} aria-label="Link Jira">
      {own && <p className="text-sm text-ok">{`Linked as ${own.email} at ${own.site_url}`}</p>}
      {!own && inherited && <p className="text-sm text-mute">{`Inherits ${inherited.site_url} from the workspace`}</p>}
      <Field label="Site URL">
        <Input
          size="sm"
          placeholder="https://acme.atlassian.net"
          value={site}
          onChange={(e) => setSite(e.target.value)}
        />
      </Field>
      <Field label="Email">
        <Input size="sm" type="email" value={email} onChange={(e) => setEmail(e.target.value)} />
      </Field>
      <Field label="API token">
        <Input size="sm" type="password" autoComplete="off" value={token} onChange={(e) => setToken(e.target.value)} />
      </Field>
      {error && <p className="text-xs text-bad">{error}</p>}
      <div className="flex gap-1">
        <Button size="sm" type="submit" loading={pending} disabled={!site || !email || !token}>
          Link
        </Button>
        {own && (
          <Button
            size="sm"
            variant="ghost"
            onClick={() => void jiraStore.unlink(scope).catch((e) => setError(messageOf(e)))}
          >
            Unlink
          </Button>
        )}
      </div>
    </form>
  );
}

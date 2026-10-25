# ADR 0004: Background session driver (spike result)

- Status: accepted for this release, provisional (the pass bar was not run)
- Date: 2026-10-25
- Plan: 04 Sessions and terminals, Task 16

## Context

Sessions sit behind the `SessionDriver` trait (`teamcode_core::sessions::driver`). **Owned** spawns the agent in a
PTY the app owns, so quitting the app ends its sessions. **Background** would start Claude Code detached with
`claude --bg`, run `claude attach <id>` in the pane, and reconcile at launch with `claude agents --json --cwd <root>`.

The spec (§7.1) sets the pass bar for the spike:

> attach shows the session's full current screen within 500 ms and echoes keystrokes within 50 ms under heavy
> output; after the app quits and relaunches, a session that was mid-turn is still running and re-attaches within 2 s
> with no lost turn, without any service the developer has to install or keep alive themselves (if macOS stops
> background sessions when Claude's background service stops, the driver must own keeping that service alive, or it
> fails this bar); hooks keep landing in the event log while the app is closed; memory overhead per attached task
> stays under 150 MB above Owned. Any failure rejects Background for this release.

The spike lives on the branch `spike/background-driver` (not merged). It adds
`crates/teamcode-core/src/sessions/background.rs`: `BackgroundDriver` implements `SessionDriver`. `start` runs
`claude --bg <claude_launch args>` in the launch's cwd, parses the printed id, and spawns the pane PTY as
`claude attach <id>`. `graceful_exit` types no keys, because Stop agent becomes `claude stop <id>`. `remove` runs
`claude rm <id>`. `ends_on_quit` is false. `reconcile(root)` parses `claude agents --json --cwd <root>`. The branch
compiles, passes clippy, and has two parse tests. Nothing in the shell uses it.

### What `claude agents --json` prints (Claude Code 2.1.310, read on this machine)

A JSON array. Each row has these fields:

| Field | Type | Values seen |
|---|---|---|
| `id` | string | the id `attach`, `logs`, `stop` and `rm` take |
| `name` | string | the session's name |
| `cwd` | string | the session's working directory |
| `kind` | string | `background`, `interactive` |
| `status` | string, optional | `busy`, `idle`, `waiting` |
| `state` | string, optional | `working`, `blocked` |
| `waitingFor` | string, optional | `dialog open` |
| `sessionId` | string | the Claude session id (the one `--resume` takes) |
| `pid` | number | the agent process |
| `startedAt` | number | Unix milliseconds |

The plan expected `id, name, cwd, status, started`. The real names are `startedAt` (milliseconds) and an extra
`kind`, `state`, `waitingFor`, `sessionId` and `pid`. `claude agents --help` says `--json` prints active sessions,
interactive and background, and `--all` adds completed background sessions. A reconcile must filter on
`kind == "background"`.

## Pass bar

The spike was run by an agent without the GUI. It could not launch the app, attach a pane, quit and relaunch it,
or measure a real long Build. It also did not start a real `claude --bg` session, because that runs a paid,
unsupervised agent. None of the rows below was measured.

| Check | Target | Result | How to measure |
|---|---|---|---|
| Attach shows the full current screen | ≤ 500 ms | Observed: not measured (needs GUI) | `performance.now()` at the `sessionsAttach` call → the `term.write` callback of the first live batch after replay |
| Keystroke echo under heavy output | ≤ 50 ms | Observed: not measured (needs GUI) | `sessionsWrite('qzx')` timestamp → first `onData` batch containing `qzx`, while `for i in $(seq 1 200); do yes \| head -c 200000; sleep 1; done` runs |
| Quit and relaunch mid-turn | still running; re-attached ≤ 2 s; no lost turn | Observed: not measured (needs GUI) | ⌘Q during the loop, relaunch, open the task; compare the transcript's last assistant message before and after |
| No developer-maintained service | none installed or kept alive by the developer | Observed: not measured (needs GUI) | `launchctl list \| grep -i claude` before and after; if Claude's own background service stops sessions when it exits, the driver must restart it, or this row fails |
| Hooks land while the app is closed | lines appended to `<app_data>/events/<task-id>.jsonl` while quit | Observed: not measured (needs GUI) | `wc -l` before quit and after relaunch, with the agent still working in between |
| Memory per attached task | ≤ 150 MB above Owned | Observed: not measured (needs GUI) | `ps -o rss= -p <app pid> <attach pid>` for one Owned task versus one Background task, both attached, after 5 minutes |
| `worktree.bgIsolation: "none"` | a background session with the worktree flag off can edit the shared checkout | Observed: not measured (needs GUI) | run with `bg_isolation_none: true`, ask for an edit outside any worktree, confirm it lands |

## Decision

**Decision:** No-go (the default): `OwnedDriver` is the only driver for this release.

The pass bar rejects Background on any failed row. Here no row was measured, so Background has not passed. The
spec's default stands, and Owned stays the only driver. This is not evidence that Background fails. It is the
absence of evidence that it passes. Re-run the table above in the GUI to decide.

## Consequences

- `OwnedDriver` is the only driver. `DriverKind::Background` stays in the contract and is unused. Quitting ends
  sessions after the quit warning and the graceful stop.
- The spike branch `spike/background-driver` is kept, not deleted. The plan says to delete it on No-go, but this
  No-go was not measured, and keeping the branch lets someone run the measurements without rewriting the driver.
  Delete it once a measured No-go is recorded here.
- If a measured run passes every row (Go), a follow-up plan covers: `claude stop` for Stop agent, `claude rm` on
  archive (which may remove the session's worktree), quitting without ending sessions (`ends_on_quit` false skips
  the quit warning), reconciling at launch against `claude agents --json` (filter `kind == "background"`, match
  `sessionId` to `Task.claude_session_id`), and the `bg_isolation_none` rule for tasks whose worktree flag is off.
- The hook transport does not depend on the driver. Each hook appends to `<app_data>/events/<task-id>.jsonl`, so
  switching drivers costs nothing on the event side.

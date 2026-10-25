//! The live sessions: each one's PTY, scrollback, flow control, idle tracker and prompt queue, and the single pane
//! it is attached to (§7.2). Output always lands in the scrollback; it reaches a pane only through that pane's sink.

use std::collections::HashMap;
use std::time::Instant;

use crate::domain::{AgentKind, PaneId, Session, SessionId, TaskId, WindowId};
use crate::sessions::driver::Pty;
use crate::sessions::flow::FlowControl;
use crate::sessions::inject::{frame_prompt, IdleTracker, InjectQueue, QueueOutcome};
use crate::sessions::scrollback::{Scrollback, ScrollbackBuffer};
use crate::Error;

/// Where an attached pane's bytes go (the shell wraps a Tauri channel).
pub trait OutputSink: Send {
    /// False when the receiver is gone; the registry then detaches.
    fn send(&self, bytes: Vec<u8>) -> bool;
}

/// A 0×0 PTY confuses full-screen programs; a collapsed pane resizes to this instead.
pub const MIN_COLS: u16 = 2;
pub const MIN_ROWS: u16 = 1;

pub struct LiveSession {
    pub session: Session,
    pub idle: IdleTracker,
    pub queue: InjectQueue,
    /// `None` once exited, or while a stop sequence has it.
    pty: Option<Box<dyn Pty>>,
    scrollback: ScrollbackBuffer,
    sink: Option<Box<dyn OutputSink>>,
    flow: FlowControl,
    size: (u16, u16),
}

impl LiveSession {
    fn unpause(&mut self) {
        self.flow.reset();
        if let Some(pty) = &self.pty {
            pty.pause_output(false);
        }
    }

    fn drop_sink(&mut self) {
        self.sink = None;
        self.session.attached_to = None;
        self.unpause();
    }

    /// Installs the sink and replays the scrollback through it in one step, so no byte is both replayed and streamed.
    fn install(&mut self, window: WindowId, pane: PaneId, sink: Box<dyn OutputSink>) -> Scrollback {
        self.unpause();
        let snapshot = self.scrollback.snapshot();
        let described = self.scrollback.describe(self.size.0, self.size.1);
        self.session.attached_to = Some((window, pane));
        self.sink = Some(sink);
        if !snapshot.is_empty() {
            self.send(snapshot);
        }
        described
    }

    fn send(&mut self, bytes: Vec<u8>) {
        let Some(sink) = &self.sink else { return };
        let n = bytes.len();
        if !sink.send(bytes) {
            self.drop_sink();
            return;
        }
        if self.flow.on_sent(n) {
            if let Some(pty) = &self.pty {
                pty.pause_output(true);
            }
        }
    }

    fn pty_mut(&mut self) -> Result<&mut Box<dyn Pty>, Error> {
        if self.session.exited.is_some() {
            return Err(Error::Precondition("session exited".into()));
        }
        self.pty
            .as_mut()
            .ok_or_else(|| Error::Precondition("session is stopping".into()))
    }

    pub fn kind(&self) -> AgentKind {
        self.session.kind
    }

    pub fn size(&self) -> (u16, u16) {
        self.size
    }
}

pub struct SessionRegistry {
    scrollback_lines: usize,
    live: HashMap<SessionId, LiveSession>,
}

impl SessionRegistry {
    pub fn new(scrollback_lines: usize) -> Self {
        Self {
            scrollback_lines,
            live: HashMap::new(),
        }
    }

    /// The ring size for sessions inserted from now on (the `scrollback_lines` setting).
    pub fn set_scrollback_lines(&mut self, lines: usize) {
        self.scrollback_lines = lines;
    }

    pub fn insert(&mut self, session: Session, pty: Box<dyn Pty>, size: (u16, u16)) {
        let live = LiveSession {
            idle: IdleTracker::for_kind(session.kind),
            queue: InjectQueue::default(),
            pty: Some(pty),
            scrollback: ScrollbackBuffer::new(self.scrollback_lines),
            sink: None,
            flow: FlowControl::default(),
            size,
            session,
        };
        self.live.insert(live.session.id.clone(), live);
    }

    fn live(&mut self, id: &SessionId) -> Result<&mut LiveSession, Error> {
        self.live
            .get_mut(id)
            .ok_or_else(|| Error::Invalid(format!("Unknown session {id}")))
    }

    /// Attaches a pane. Another pane holding it is `Precondition("attached-elsewhere:<window>:<pane>")`; the same
    /// pane may attach again (a remounted terminal), which replays the scrollback afresh.
    pub fn attach(
        &mut self,
        id: &SessionId,
        window: WindowId,
        pane: PaneId,
        sink: Box<dyn OutputSink>,
    ) -> Result<Scrollback, Error> {
        let live = self.live(id)?;
        if let Some((w, p)) = &live.session.attached_to {
            if (w, p) != (&window, &pane) {
                return Err(Error::Precondition(format!("attached-elsewhere:{w}:{p}")));
            }
        }
        Ok(live.install(window, pane, sink))
    }

    /// Moves the attachment to this pane, whoever holds it.
    pub fn bring(
        &mut self,
        id: &SessionId,
        window: WindowId,
        pane: PaneId,
        sink: Box<dyn OutputSink>,
    ) -> Result<Scrollback, Error> {
        let live = self.live(id)?;
        live.drop_sink();
        Ok(live.install(window, pane, sink))
    }

    pub fn detach(&mut self, id: &SessionId) -> Result<(), Error> {
        self.live(id)?.drop_sink();
        Ok(())
    }

    /// Detaches every session a closed window held; the processes keep running.
    pub fn detach_window(&mut self, window: &WindowId) -> Vec<SessionId> {
        let mut ids = Vec::new();
        for (id, live) in &mut self.live {
            if live.session.attached_to.as_ref().is_some_and(|(w, _)| w == window) {
                live.drop_sink();
                ids.push(id.clone());
            }
        }
        ids.sort();
        ids
    }

    pub fn write(&mut self, id: &SessionId, bytes: &[u8]) -> Result<(), Error> {
        self.live(id)?.pty_mut()?.write(bytes)
    }

    /// Clamped to `MIN_COLS`×`MIN_ROWS`; an unchanged size does not reach the PTY.
    pub fn resize(&mut self, id: &SessionId, cols: u16, rows: u16) -> Result<(), Error> {
        let live = self.live(id)?;
        let size = (cols.max(MIN_COLS), rows.max(MIN_ROWS));
        if size == live.size {
            return Ok(());
        }
        live.pty_mut()?.resize(size.0, size.1)?;
        live.size = size;
        Ok(())
    }

    /// The pane wrote `n` more bytes to the screen.
    pub fn ack(&mut self, id: &SessionId, n: usize) -> Result<(), Error> {
        let live = self.live(id)?;
        if live.flow.on_ack(n) {
            if let Some(pty) = &live.pty {
                pty.pause_output(false);
            }
        }
        Ok(())
    }

    pub fn on_output(&mut self, id: &SessionId, bytes: &[u8], now: Instant) {
        let Some(live) = self.live.get_mut(id) else { return };
        live.scrollback.push(bytes);
        live.idle.on_output(now);
        live.send(bytes.to_vec());
    }

    /// Marks the session exited: the queue is cleared, the pane keeps its scrollback but gets no more bytes, and
    /// writes are refused. `None` for an unknown session.
    pub fn on_exit(&mut self, id: &SessionId, code: i32) -> Option<Session> {
        let live = self.live.get_mut(id)?;
        live.queue.clear();
        live.unpause();
        live.sink = None;
        live.pty = None;
        live.session.exited = Some(code);
        Some(live.session.clone())
    }

    pub fn get(&self, id: &SessionId) -> Option<&Session> {
        self.live.get(id).map(|l| &l.session)
    }

    pub fn get_mut(&mut self, id: &SessionId) -> Option<&mut LiveSession> {
        self.live.get_mut(id)
    }

    pub fn list(&self) -> Vec<Session> {
        let mut all: Vec<Session> = self.live.values().map(|l| l.session.clone()).collect();
        all.sort_by(|a, b| a.started_at.cmp(&b.started_at).then_with(|| a.id.cmp(&b.id)));
        all
    }

    /// Sessions not exited whose agent is not idle.
    pub fn working(&self, now: Instant) -> Vec<Session> {
        self.list()
            .into_iter()
            .filter(|s| s.exited.is_none() && self.live.get(&s.id).is_some_and(|l| !l.idle.is_idle(now)))
            .collect()
    }

    /// Types `prompt` now when the agent is idle, else queues it until it is.
    pub fn inject(&mut self, id: &SessionId, prompt: String, now: Instant) -> Result<QueueOutcome, Error> {
        let live = self.live(id)?;
        if live.session.exited.is_some() {
            return Err(Error::Precondition("session exited".into()));
        }
        if live.queue.is_empty() && live.idle.is_idle(now) {
            live.pty_mut()?.write(&frame_prompt(&prompt))?;
            live.idle.on_injected();
            return Ok(QueueOutcome::Sent);
        }
        Ok(QueueOutcome::Queued {
            pending: live.queue.push(prompt),
        })
    }

    /// Sends the oldest queued prompt when the agent is idle; `Some(pending)` when one was sent.
    pub fn pump(&mut self, id: &SessionId, now: Instant) -> Result<Option<usize>, Error> {
        let live = self.live(id)?;
        if live.queue.is_empty() || !live.idle.is_idle(now) {
            return Ok(None);
        }
        let Some(prompt) = live.queue.pop_front() else {
            return Ok(None);
        };
        if let Err(e) = live.pty_mut().and_then(|pty| pty.write(&frame_prompt(&prompt))) {
            live.queue.push_front(prompt); // nothing was typed
            return Err(e);
        }
        live.idle.on_injected();
        Ok(Some(live.queue.len()))
    }

    /// The task's live session, if any: not exited and not being stopped (a Start fresh session's old session
    /// still runs its stop sequence while the new one takes prompts).
    pub fn live_for_task(&self, task: &TaskId) -> Option<SessionId> {
        self.live
            .values()
            .filter(|l| l.session.exited.is_none() && l.pty.is_some() && l.session.task_id.as_ref() == Some(task))
            .max_by(|a, b| a.session.started_at.cmp(&b.session.started_at))
            .map(|l| l.session.id.clone())
    }

    /// Takes the PTY out for a stop sequence; the reader is unpaused first so the child can see the exit keys.
    pub fn take_pty(&mut self, id: &SessionId) -> Option<Box<dyn Pty>> {
        let live = self.live.get_mut(id)?;
        live.unpause();
        live.pty.take()
    }

    pub fn remove(&mut self, id: &SessionId) -> Option<Session> {
        self.live.remove(id).map(|l| l.session)
    }
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;

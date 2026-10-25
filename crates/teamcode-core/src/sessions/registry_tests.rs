//! `SessionRegistry` tests over `FakePty` and a `VecSink`.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use chrono::Utc;

use super::*;
use crate::domain::{AgentKind, DriverKind, PaneId, Session, SessionId, WindowId};
use crate::hooks::HookEvent;
use crate::sessions::fake::{FakePty, FakePtyHandle};
use crate::sessions::flow::FlowControl;
use crate::sessions::inject::{frame_prompt, QueueOutcome};
use crate::Error;

#[derive(Clone, Default)]
struct VecSink(Arc<Mutex<Vec<u8>>>);

impl VecSink {
    fn bytes(&self) -> Vec<u8> {
        self.0.lock().unwrap().clone()
    }
}

impl OutputSink for VecSink {
    fn send(&self, bytes: Vec<u8>) -> bool {
        self.0.lock().unwrap().extend(bytes);
        true
    }
}

fn now() -> Instant {
    Instant::now()
}

fn w(s: &str) -> WindowId {
    WindowId::parse(s).unwrap()
}

fn p(s: &str) -> PaneId {
    PaneId::parse(s).unwrap()
}

fn session(kind: AgentKind) -> Session {
    Session {
        id: SessionId::new(),
        task_id: None,
        kind,
        cwd: "/r".into(),
        driver: DriverKind::Owned,
        attached_to: None,
        started_at: Utc::now(),
        exited: None,
    }
}

pub(super) fn insert_second(r: &mut SessionRegistry, kind: AgentKind) -> SessionId {
    let s = session(kind);
    let id = s.id.clone();
    let (pty, _) = FakePty::pair();
    r.insert(s, Box::new(pty), (80, 24));
    id
}

pub(super) fn registry_with_fake(kind: AgentKind) -> (SessionRegistry, SessionId, FakePtyHandle) {
    let mut r = SessionRegistry::new(100);
    let s = session(kind);
    let id = s.id.clone();
    let (pty, h) = FakePty::pair();
    r.insert(s, Box::new(pty), (80, 24));
    (r, id, h)
}

#[test]
fn attach_replays_scrollback_then_streams_live_output() {
    let (mut r, id, _pty) = registry_with_fake(AgentKind::Shell);
    r.on_output(&id, b"old\n", now());
    let sink = VecSink::default();
    let snap = r.attach(&id, w("w-1"), p("pane-a"), Box::new(sink.clone())).unwrap();
    r.on_output(&id, b"new\n", now());
    assert_eq!(sink.bytes(), b"old\nnew\n");
    assert_eq!(snap.lines, 1);
    assert_eq!(r.get(&id).unwrap().attached_to, Some((w("w-1"), p("pane-a"))));
}

#[test]
fn second_pane_cannot_attach_while_first_holds_it() {
    let (mut r, id, _) = registry_with_fake(AgentKind::Shell);
    r.attach(&id, w("w-1"), p("pane-a"), Box::new(VecSink::default()))
        .unwrap();
    let err = r
        .attach(&id, w("w-2"), p("pane-b"), Box::new(VecSink::default()))
        .unwrap_err();
    assert_eq!(err, Error::Precondition("attached-elsewhere:w-1:pane-a".into()));
    r.attach(&id, w("w-1"), p("pane-a"), Box::new(VecSink::default()))
        .unwrap(); // same pane may re-attach
}

#[test]
fn bring_moves_the_attachment() {
    let (mut r, id, _) = registry_with_fake(AgentKind::Shell);
    let a = VecSink::default();
    let b = VecSink::default();
    r.attach(&id, w("w-1"), p("pane-a"), Box::new(a.clone())).unwrap();
    r.bring(&id, w("w-2"), p("pane-b"), Box::new(b.clone())).unwrap();
    r.on_output(&id, b"x", now());
    assert_eq!(a.bytes(), b"");
    assert_eq!(b.bytes(), b"x");
    assert_eq!(r.get(&id).unwrap().attached_to, Some((w("w-2"), p("pane-b"))));
}

#[test]
fn detach_window_keeps_process_and_clears_attachment() {
    let (mut r, id, pty) = registry_with_fake(AgentKind::Shell);
    r.attach(&id, w("w-1"), p("pane-a"), Box::new(VecSink::default()))
        .unwrap();
    assert_eq!(r.detach_window(&w("w-1")), vec![id.clone()]);
    assert_eq!(r.get(&id).unwrap().attached_to, None);
    assert!(!pty.killed());
    r.attach(&id, w("w-3"), p("pane-c"), Box::new(VecSink::default()))
        .unwrap();
}

#[test]
fn output_while_detached_goes_only_to_scrollback() {
    let (mut r, id, _) = registry_with_fake(AgentKind::Shell);
    r.on_output(&id, b"quiet\n", now());
    let s = VecSink::default();
    r.attach(&id, w("w-1"), p("pane-a"), Box::new(s.clone())).unwrap();
    assert_eq!(s.bytes(), b"quiet\n");
}

#[test]
fn flow_control_pauses_pty_and_ack_resumes_it() {
    let (mut r, id, pty) = registry_with_fake(AgentKind::Shell);
    r.attach(&id, w("w-1"), p("pane-a"), Box::new(VecSink::default()))
        .unwrap();
    r.on_output(&id, &vec![b'a'; FlowControl::HIGH_WATER], now());
    assert!(pty.paused());
    r.ack(&id, FlowControl::HIGH_WATER - FlowControl::LOW_WATER + 1)
        .unwrap();
    assert!(!pty.paused());
}

#[test]
fn detach_resets_flow_control() {
    let (mut r, id, pty) = registry_with_fake(AgentKind::Shell);
    r.attach(&id, w("w-1"), p("pane-a"), Box::new(VecSink::default()))
        .unwrap();
    r.on_output(&id, &vec![b'a'; FlowControl::HIGH_WATER], now());
    assert!(pty.paused());
    r.detach(&id).unwrap();
    assert!(!pty.paused());
}

#[test]
fn resize_clamps_to_minimum() {
    let (mut r, id, pty) = registry_with_fake(AgentKind::Shell);
    r.resize(&id, 0, 0).unwrap();
    assert_eq!(pty.last_resize(), Some((MIN_COLS, MIN_ROWS)));
}

#[test]
fn exit_clears_queue_and_sink() {
    let (mut r, id, _) = registry_with_fake(AgentKind::ClaudeCode);
    r.get_mut(&id).unwrap().queue.push("later".into());
    let s = r.on_exit(&id, 3).unwrap();
    assert_eq!(s.exited, Some(3));
    assert_eq!(r.get_mut(&id).unwrap().queue.len(), 0);
    assert_eq!(
        r.write(&id, b"x").unwrap_err(),
        Error::Precondition("session exited".into())
    );
}

#[test]
fn inject_sends_when_idle_and_queues_when_working() {
    let (mut r, id, pty) = registry_with_fake(AgentKind::ClaudeCode);
    let now = Instant::now();
    assert!(matches!(
        r.inject(&id, "p1".into(), now).unwrap(),
        QueueOutcome::Queued { pending: 1 }
    ));
    assert!(pty.writes().is_empty());
    r.get_mut(&id).unwrap().idle.on_hook(&HookEvent::Stop {
        last_assistant_message: "".into(),
        background_tasks: 0,
    });
    assert_eq!(r.pump(&id, now).unwrap(), Some(0));
    assert_eq!(pty.writes(), vec![frame_prompt("p1")]);
    // injecting made it working again
    assert!(matches!(
        r.inject(&id, "p2".into(), now).unwrap(),
        QueueOutcome::Queued { pending: 1 }
    ));
}

#[test]
fn pump_sends_one_prompt_per_idle_in_fifo_order() {
    let (mut r, id, pty) = registry_with_fake(AgentKind::ClaudeCode);
    let now = Instant::now();
    r.inject(&id, "a".into(), now).unwrap();
    r.inject(&id, "b".into(), now).unwrap();
    r.get_mut(&id).unwrap().idle.on_hook(&HookEvent::Stop {
        last_assistant_message: "".into(),
        background_tasks: 0,
    });
    assert_eq!(r.pump(&id, now).unwrap(), Some(1));
    assert_eq!(r.pump(&id, now).unwrap(), None);
    assert_eq!(pty.writes(), vec![frame_prompt("a")]);
}

#[test]
fn working_lists_only_unexited_non_idle_sessions() {
    let (mut r, a, _) = registry_with_fake(AgentKind::ClaudeCode); // Hooks{idle:false} → working
    let _b = insert_second(&mut r, AgentKind::Shell); // Always idle
    let c = insert_second(&mut r, AgentKind::ClaudeCode);
    r.on_exit(&c, 0);
    assert_eq!(
        r.working(Instant::now())
            .iter()
            .map(|s| s.id.clone())
            .collect::<Vec<_>>(),
        vec![a]
    );
}

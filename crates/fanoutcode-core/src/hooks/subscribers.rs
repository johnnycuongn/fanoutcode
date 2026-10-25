//! The registry behind `hooks_tailer::subscribe` (A18): Plan 04 registers its `note_hook_event` here, and the tailer
//! publishes every parsed event to it once, in log order.

use std::sync::RwLock;

use super::HookEvent;
use crate::domain::TaskId;

/// A callback for every parsed hook event (A18).
pub type HookSubscriber = Box<dyn Fn(&TaskId, &HookEvent) + Send + Sync>;

/// Subscribers in registration order. `publish` never clones or buffers, so "each event once" rests on the tailer
/// parsing each line once.
#[derive(Default)]
pub struct Subscribers {
    list: RwLock<Vec<HookSubscriber>>,
}

impl Subscribers {
    pub fn subscribe(&self, f: HookSubscriber) {
        self.list.write().expect("subscribers lock").push(f);
    }

    /// Calls every subscriber, in registration order.
    pub fn publish(&self, task_id: &TaskId, ev: &HookEvent) {
        for f in self.list.read().expect("subscribers lock").iter() {
            f(task_id, ev);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::hooks::parse::{parse_line, split_complete_lines};

    fn tid() -> TaskId {
        TaskId::parse("t-1").unwrap()
    }

    #[test]
    fn a_subscriber_receives_each_parsed_event_once_in_order() {
        let subs = Subscribers::default();
        let seen: Arc<Mutex<Vec<(TaskId, HookEvent)>>> = Arc::default();
        let sink = seen.clone();
        subs.subscribe(Box::new(move |id, ev| {
            sink.lock().unwrap().push((id.clone(), ev.clone()))
        }));
        let id = tid();
        let mut seq = 0;
        let mut pump = |buf: &mut Vec<u8>| {
            for l in split_complete_lines(buf) {
                seq += 1;
                subs.publish(&id, &parse_line(&id, seq, &l).unwrap().event);
            }
        };
        let mut buf = b"UserPromptSubmit\t2026-10-24T05:12:40Z\t{}\nNotification\t2026-10-24T05:12:41Z\t{\"message\":\"a\"}\nStop\t2026-10-24T05:12".to_vec();
        pump(&mut buf); // two complete lines, partial Stop waits
        buf.extend_from_slice(b":42Z\t{\"last_assistant_message\":\"x\",\"background_tasks\":[]}\n");
        pump(&mut buf);
        pump(&mut buf); // nothing new: nothing redelivered
        let seen = seen.lock().unwrap();
        assert_eq!(
            seen.iter().map(|(_, e)| e.clone()).collect::<Vec<_>>(),
            vec![
                HookEvent::UserPromptSubmit,
                HookEvent::Notification { message: "a".into() },
                HookEvent::Stop {
                    last_assistant_message: "x".into(),
                    background_tasks: 0
                }
            ]
        );
        assert!(seen.iter().all(|(t, _)| *t == tid()));
    }
}

//! The Spec stage's LiveSpec run (§6.1, A14): LiveSpec's analyze sequence moved into core, unchanged in
//! livespec-core: the Jev pre-phase, one notes request per lane, the lanes merged, and the Conflicts fact-checked
//! when the plan asks. Events mirror LiveSpec's channel events.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

use livespec_core::agents::run_limited;
use livespec_core::analysis::{merge_lanes, pre_phase, PreInput, PreOutcome, PRE_BUDGET};
use livespec_core::jev::check::{self, CheckEvent, CheckResult};
use livespec_core::jev::Jev;
use livespec_core::llm::{Chunk, Provider};
use livespec_core::project::ProjectIndex;
use livespec_core::prompt::{analysis_request, section_body, section_groups};
use livespec_core::sections;
use livespec_core::usage::{metered, UsageLedger, UsageTag};
use serde::{Deserialize, Serialize};

use super::livespec_level::LanePlan;

/// What `livespec_analyze` streams to the Spec stage editor (A14).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(tag = "type")]
pub enum LiveSpecEvent {
    Text { text: String, lane: u32 },
    Started,
    Skipped,
    Notice { message: String },
    Checking { count: u32, total: u32 },
    Check { result: LiveSpecCheck },
    Done { text: String, partial: bool },
    Error { message: String },
}

/// One fact-check verdict. `verdict` is livespec-core's serialised value (`"contradicted"`); livespec-core's own
/// type has no `specta::Type`, hence this mirror.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
pub struct LiveSpecCheck {
    pub claim: String,
    pub verdict: String,
    pub confidence: Option<f64>,
    pub path: Option<String>,
    pub line: Option<u32>,
}

impl From<CheckResult> for LiveSpecCheck {
    fn from(r: CheckResult) -> Self {
        let verdict = serde_json::to_value(r.verdict)
            .ok()
            .and_then(|v| v.as_str().map(str::to_owned))
            .unwrap_or_default();
        LiveSpecCheck {
            claim: r.claim,
            verdict,
            confidence: r.confidence,
            path: r.path,
            line: r.line.map(|l| l as u32),
        }
    }
}

/// What one run reads.
pub struct AnalyzeInput<'a> {
    pub draft: &'a str,
    /// The last draft that was analysed (the Jev gate skips an unchanged idea).
    pub previous_draft: Option<&'a str>,
    pub index: Option<&'a ProjectIndex>,
    pub brief: Option<&'a str>,
    pub plan: &'a LanePlan,
    pub gate: bool,
    pub rerank: bool,
}

/// LiveSpec's analyze: pre-phase (`Skipped` ends it), `Started`, the lanes streaming `Text { lane }`, the merge
/// (`Notice` for missing sections, `Done`), then the Conflicts fact-check when planned. The merged notes, else
/// `None`.
pub async fn analyze(
    input: AnalyzeInput<'_>,
    provider: &Provider,
    jev: Option<&Jev>,
    ledger: Arc<UsageLedger>,
    tag: UsageTag,
    emit: &(dyn Fn(LiveSpecEvent) + Send + Sync),
) -> Option<String> {
    let budget = provider.context_budget_chars();
    let pre = PreInput {
        draft: input.draft,
        previous_draft: input.previous_draft,
        index: input.index,
        max_files: if budget < 40_000 { 3 } else { 5 },
        budget_chars: budget / 4,
        jev,
        gate: input.gate,
        rerank: input.rerank,
    };
    let snippets = match pre_phase(pre, PRE_BUDGET).await {
        PreOutcome::Skip => {
            emit(LiveSpecEvent::Skipped);
            return None;
        }
        PreOutcome::Analyze { snippets, notice } => {
            emit(LiveSpecEvent::Started);
            if let Some(message) = notice {
                emit(LiveSpecEvent::Notice { message });
            }
            snippets
        }
    };
    let ids: Vec<String> = input.plan.section_ids.iter().map(|s| (*s).to_owned()).collect();
    let chosen = sections::resolve(&ids);
    let groups = section_groups(input.plan.lanes, &chosen);
    let jobs: Vec<_> = groups
        .iter()
        .enumerate()
        .map(|(lane, group)| {
            let req = analysis_request(input.draft, input.index, input.brief, &snippets, provider, group, "");
            let (ledger, tag) = (ledger.clone(), tag.clone());
            async move {
                let mut forward = |c: Chunk| {
                    if let Chunk::Text(text) = c {
                        emit(LiveSpecEvent::Text {
                            text,
                            lane: lane as u32,
                        });
                    }
                };
                metered(provider, &req, ledger, tag, &mut forward).await
            }
        })
        .collect();
    let results = run_limited(groups.len(), jobs).await;
    // the lane that wrote the checkable section, for the fact-check
    let conflicts = groups
        .iter()
        .zip(&results)
        .find(|(g, _)| g.iter().any(|s| s.checkable))
        .and_then(|(_, r)| r.as_ref().ok())
        .and_then(|text| section_body(text, "conflicts"));
    let (notes, notice) = match merge_lanes(&groups, results) {
        Ok(merged) => merged,
        Err(e) => {
            emit(LiveSpecEvent::Error { message: e.to_string() });
            return None;
        }
    };
    if let Some(message) = notice.clone() {
        emit(LiveSpecEvent::Notice { message });
    }
    emit(LiveSpecEvent::Done {
        text: notes.clone(),
        partial: notice.is_some(),
    });
    if let (true, Some(index), Some(jev), Some(faults)) = (input.plan.fact_check, input.index, jev, conflicts) {
        check::run(jev, index, &faults, |e| {
            emit(match e {
                CheckEvent::Checking { count, total } => LiveSpecEvent::Checking {
                    count: count as u32,
                    total: total as u32,
                },
                CheckEvent::Result(r) => LiveSpecEvent::Check { result: r.into() },
                CheckEvent::Notice(message) => LiveSpecEvent::Notice { message },
            })
        })
        .await;
    }
    Some(notes)
}

/// One run per key; a new run aborts the one it displaces (dropping its CLI children through `kill_on_drop`).
pub struct RunSlots<K> {
    runs: Mutex<HashMap<K, tokio::task::JoinHandle<()>>>,
}

impl<K> Default for RunSlots<K> {
    fn default() -> Self {
        Self {
            runs: Mutex::new(HashMap::new()),
        }
    }
}

impl<K: Eq + Hash> RunSlots<K> {
    pub fn replace(&self, key: K, run: tokio::task::JoinHandle<()>) {
        if let Some(old) = self.runs.lock().expect("run slots lock").insert(key, run) {
            old.abort();
        }
    }

    /// Aborts the key's run; whether there was one still in flight.
    pub fn cancel(&self, key: &K) -> bool {
        match self.runs.lock().expect("run slots lock").remove(key) {
            Some(run) => {
                let live = !run.is_finished();
                run.abort();
                live
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    use livespec_core::jev::check::Verdict;
    use livespec_core::llm::OpenAiCompat;
    use livespec_core::sections::DEFAULT_SECTIONS;
    use livespec_core::settings::{Settings, Tier};
    use serde_json::json;

    use super::*;
    use crate::domain::LiveSpecLevel;
    use crate::helpers::livespec_level::lane_plan;
    use crate::helpers::tag_for;
    use crate::helpers::test_server::{sse_body, sse_server_n};

    const SIMPLE_REPLY: &str =
        "## Conflicts\n- Nothing so far.\n## Gaps\n- No acceptance criteria.\n## Questions\n- Which board?\n";
    const ALL_FIVE_REPLY: &str =
        "## Intent\n- x\n## Conflicts\n- x\n## Gaps\n- x\n## Ambiguities\n- x\n## Questions\n- x\n";

    type Events = Arc<Mutex<Vec<LiveSpecEvent>>>;

    fn collect() -> (Events, impl Fn(LiveSpecEvent) + Send + Sync) {
        let events: Events = Arc::default();
        let sink = events.clone();
        (events, move |e| sink.lock().unwrap().push(e))
    }

    fn lanes(ev: &[LiveSpecEvent]) -> BTreeSet<u32> {
        ev.iter()
            .filter_map(|e| match e {
                LiveSpecEvent::Text { lane, .. } => Some(*lane),
                _ => None,
            })
            .collect()
    }

    fn input<'a>(draft: &'a str, plan: &'a LanePlan) -> AnalyzeInput<'a> {
        AnalyzeInput {
            draft,
            previous_draft: None,
            index: None,
            brief: None,
            plan,
            gate: false,
            rerank: false,
        }
    }

    fn fake(url: String) -> Provider {
        Provider::OpenAiCompat(OpenAiCompat {
            base_url: url,
            model: "qwen3-4b-instruct".into(),
            context_tokens: 4096,
            http: reqwest::Client::new(),
        })
    }

    fn ledger() -> Arc<UsageLedger> {
        Arc::new(UsageLedger::new(
            std::env::temp_dir().join("fanoutcode-livespec-test-usage.json"),
        ))
    }

    #[tokio::test]
    async fn simple_streams_one_lane_then_done() {
        let (url, _s) = sse_server_n(&sse_body(SIMPLE_REPLY, 10, 10), 1).await;
        let plan = lane_plan(LiveSpecLevel::Simple, &Settings::default(), false).unwrap();
        let (events, emit) = collect();
        let notes = analyze(
            input("# Spec\nRows per project.", &plan),
            &fake(url),
            None,
            ledger(),
            tag_for(Path::new("/repo")),
            &emit,
        )
        .await;
        let ev = events.lock().unwrap().clone();
        assert_eq!(ev.first(), Some(&LiveSpecEvent::Started));
        assert!(lanes(&ev).iter().all(|&l| l == 0));
        assert!(matches!(ev.last(), Some(LiveSpecEvent::Done { text, partial: false }) if text.contains("## Gaps")));
        assert!(notes.unwrap().contains("## Questions"));
        assert!(!ev
            .iter()
            .any(|e| matches!(e, LiveSpecEvent::Checking { .. } | LiveSpecEvent::Check { .. })));
    }

    #[tokio::test]
    async fn heavy_lanes_tag_their_text_by_lane() {
        let (url, _s) = sse_server_n(&sse_body(ALL_FIVE_REPLY, 10, 10), 2).await;
        let plan = LanePlan {
            tier: Tier::Deep,
            section_ids: DEFAULT_SECTIONS.to_vec(),
            lanes: 2,
            fact_check: false,
        };
        let (events, emit) = collect();
        analyze(
            input("# Spec", &plan),
            &fake(url),
            None,
            ledger(),
            tag_for(Path::new("/repo")),
            &emit,
        )
        .await
        .unwrap();
        let ev = events.lock().unwrap().clone();
        assert_eq!(lanes(&ev), BTreeSet::from([0, 1]));
        assert!(matches!(ev.last(), Some(LiveSpecEvent::Done { partial: false, .. })));
    }

    #[tokio::test]
    async fn a_lane_missing_its_sections_ends_partial_with_a_notice() {
        let (url, _s) = sse_server_n(&sse_body("## Conflicts\n- x\n", 10, 10), 1).await;
        let plan = lane_plan(LiveSpecLevel::Simple, &Settings::default(), false).unwrap();
        let (events, emit) = collect();
        analyze(
            input("# Spec", &plan),
            &fake(url),
            None,
            ledger(),
            tag_for(Path::new("/repo")),
            &emit,
        )
        .await;
        let ev = events.lock().unwrap().clone();
        assert!(ev
            .iter()
            .any(|e| matches!(e, LiveSpecEvent::Notice { message } if message.starts_with("Couldn't write"))));
        assert!(matches!(ev.last(), Some(LiveSpecEvent::Done { partial: true, .. })));
    }

    #[tokio::test]
    async fn a_provider_failure_ends_with_an_error_event() {
        let plan = lane_plan(LiveSpecLevel::Simple, &Settings::default(), false).unwrap();
        let (events, emit) = collect();
        let notes = analyze(
            input("# Spec", &plan),
            &fake("http://127.0.0.1:9".into()),
            None,
            ledger(),
            tag_for(Path::new("/repo")),
            &emit,
        )
        .await;
        assert!(notes.is_none());
        assert!(matches!(
            events.lock().unwrap().last(),
            Some(LiveSpecEvent::Error { .. })
        ));
    }

    #[test]
    fn live_spec_events_serialise_tagged_by_type() {
        assert_eq!(
            serde_json::to_value(LiveSpecEvent::Text {
                text: "a".into(),
                lane: 1
            })
            .unwrap(),
            json!({"type": "Text", "text": "a", "lane": 1})
        );
        assert_eq!(
            serde_json::to_value(LiveSpecEvent::Started).unwrap(),
            json!({"type": "Started"})
        );
    }

    #[test]
    fn check_results_map_to_live_spec_check() {
        let c: LiveSpecCheck = CheckResult {
            claim: "x".into(),
            verdict: Verdict::Contradicted,
            confidence: Some(0.9),
            path: Some("src/a.rs".into()),
            line: Some(3),
        }
        .into();
        assert_eq!(
            (c.verdict.as_str(), c.path.as_deref(), c.line),
            ("contradicted", Some("src/a.rs"), Some(3))
        );
    }

    #[tokio::test]
    async fn a_new_run_replaces_the_one_in_flight() {
        let slots = RunSlots::<String>::default();
        let first = tokio::spawn(std::future::pending::<()>());
        let probe = first.abort_handle();
        slots.replace("t-1".into(), first);
        slots.replace("t-1".into(), tokio::spawn(std::future::pending::<()>()));
        tokio::task::yield_now().await;
        assert!(probe.is_finished());
        assert!(slots.cancel(&"t-1".into()));
        assert!(!slots.cancel(&"t-1".into()));
    }

    #[test]
    fn every_live_spec_event_round_trips() {
        let check = LiveSpecCheck {
            claim: "c".into(),
            verdict: "confirmed".into(),
            confidence: None,
            path: None,
            line: None,
        };
        for e in [
            LiveSpecEvent::Text {
                text: "a".into(),
                lane: 0,
            },
            LiveSpecEvent::Started,
            LiveSpecEvent::Skipped,
            LiveSpecEvent::Notice { message: "n".into() },
            LiveSpecEvent::Checking { count: 1, total: 2 },
            LiveSpecEvent::Check { result: check },
            LiveSpecEvent::Done {
                text: "d".into(),
                partial: true,
            },
            LiveSpecEvent::Error { message: "e".into() },
        ] {
            let back: LiveSpecEvent = serde_json::from_str(&serde_json::to_string(&e).unwrap()).unwrap();
            assert_eq!(back, e);
        }
    }
}

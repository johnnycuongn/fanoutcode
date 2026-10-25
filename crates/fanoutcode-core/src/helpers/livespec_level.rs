//! The Spec stage's LiveSpec switch (§6.1): Off, Simple, Heavy mapped onto livespec-core's tiers, sections and
//! lanes.

use livespec_core::sections::DEFAULT_SECTIONS;
use livespec_core::settings::{Settings, Tier};

use crate::domain::LiveSpecLevel;

/// How one LiveSpec run goes: the tier, the sections, how many lanes write them, and whether the Conflicts are
/// fact-checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanePlan {
    pub tier: Tier,
    pub section_ids: Vec<&'static str>,
    pub lanes: usize,
    pub fact_check: bool,
}

/// The most lanes Heavy runs, one per section at most.
const MAX_HEAVY_LANES: usize = 5;

/// Off: nothing. Simple: one fast lane with Conflicts, Gaps and Questions, never checked. Heavy: every section on
/// the deep tier in up to `max_agents` lanes (one with the Local provider), fact-checked when Jev is allowed and a
/// TypeSafe key exists.
pub fn lane_plan(level: LiveSpecLevel, settings: &Settings, typesafe_key_present: bool) -> Option<LanePlan> {
    match level {
        LiveSpecLevel::Off => None,
        LiveSpecLevel::Simple => Some(LanePlan {
            tier: Tier::Fast,
            section_ids: vec!["conflicts", "gaps", "questions"],
            lanes: 1,
            fact_check: false,
        }),
        LiveSpecLevel::Heavy => Some(LanePlan {
            tier: Tier::Deep,
            section_ids: DEFAULT_SECTIONS.to_vec(),
            lanes: if settings.jev_allowed() {
                settings.max_agents().min(MAX_HEAVY_LANES)
            } else {
                1
            },
            fact_check: settings.jev_allowed() && typesafe_key_present,
        }),
    }
}

#[cfg(test)]
mod tests {
    use livespec_core::settings::ProviderSettings;

    use super::*;

    #[test]
    fn off_runs_nothing() {
        assert!(lane_plan(LiveSpecLevel::Off, &Settings::default(), true).is_none());
    }

    #[test]
    fn simple_is_one_fast_lane_with_three_sections_and_no_check() {
        let p = lane_plan(LiveSpecLevel::Simple, &Settings::default(), true).unwrap();
        assert_eq!(
            (p.tier, p.section_ids.clone(), p.lanes, p.fact_check),
            (Tier::Fast, vec!["conflicts", "gaps", "questions"], 1, false)
        );
    }

    #[test]
    fn heavy_is_all_sections_on_deep_with_parallel_lanes() {
        let p = lane_plan(LiveSpecLevel::Heavy, &Settings::default(), true).unwrap(); // max_agents defaults to 4
        assert_eq!(
            (p.tier, p.section_ids.clone(), p.lanes, p.fact_check),
            (Tier::Deep, DEFAULT_SECTIONS.to_vec(), 4, true)
        );
    }

    #[test]
    fn heavy_fact_checks_only_with_a_key() {
        assert!(
            !lane_plan(LiveSpecLevel::Heavy, &Settings::default(), false)
                .unwrap()
                .fact_check
        );
    }

    #[test]
    fn a_local_provider_runs_one_lane_and_never_checks() {
        let s = Settings {
            provider: ProviderSettings::Local {
                model_id: "qwen3-4b-instruct".into(),
            },
            ..Settings::default()
        };
        let p = lane_plan(LiveSpecLevel::Heavy, &s, true).unwrap();
        assert_eq!((p.lanes, p.fact_check), (1, false));
    }
}

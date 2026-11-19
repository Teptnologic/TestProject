//! What a match reports when it ends.

use arena_kernel::{EntityId, Event, World};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Serialize)]
pub struct UnitReport {
    pub team: String,
    pub unit: String,
    pub alive: bool,
    /// Health-like measure at the end, `0..=1`.
    pub vitality: f64,
    pub kills: u32,
    pub deaths: u32,
    /// Sum of event values this unit caused, by event name (e.g. `damage_dealt`).
    pub dealt: BTreeMap<String, f64>,
    /// Sum of event values that happened to this unit, by event name.
    pub taken: BTreeMap<String, f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MatchResult {
    pub scenario: String,
    pub seed: u64,
    /// Team names, best first. Teams in the same group tied.
    pub placements: Vec<Vec<String>>,
    /// The sole first-placed team, if there is one.
    pub winner: Option<String>,
    pub reason: String,
    pub duration_s: f64,
    pub ticks: u64,
    pub scores: BTreeMap<String, f64>,
    pub units: Vec<UnitReport>,
    /// Fingerprint of the final world; equal for identical matches.
    pub state_hash: u64,
}

/// Accumulates per-unit totals from the event stream.
#[derive(Clone, Debug, Default)]
pub(crate) struct Stats {
    pub units: BTreeMap<EntityId, UnitReport>,
}

impl Stats {
    pub fn track(&mut self, id: EntityId, team: &str, unit: &str) {
        let report = UnitReport {
            team: team.to_string(),
            unit: unit.to_string(),
            ..UnitReport::default()
        };
        self.units.insert(id, report);
    }

    pub fn record(&mut self, w: &World, ev: &Event) {
        let died = ev.kind == w.defs.reg.ev.died;
        let name = w.defs.reg.events.name(ev.kind.0);
        if let Some(r) = ev.source.and_then(|s| self.units.get_mut(&s)) {
            r.kills += died as u32;
            if ev.value != 0.0 {
                *r.dealt.entry(name.to_string()).or_insert(0.0) += ev.value;
            }
        }
        if let Some(r) = ev.target.and_then(|t| self.units.get_mut(&t)) {
            r.deaths += died as u32;
            if ev.value != 0.0 {
                *r.taken.entry(name.to_string()).or_insert(0.0) += ev.value;
            }
        }
    }
}

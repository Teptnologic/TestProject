//! Many matches of one scenario, in parallel, for win rates.

use crate::{Content, Match, Scenario};
use rayon::prelude::*;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, Serialize)]
pub struct TeamRecord {
    pub wins: u32,
    pub win_rate: f64,
    /// 95% confidence interval for the win rate (Wilson score).
    pub win_rate_low: f64,
    pub win_rate_high: f64,
    /// Average `damage_dealt`-style totals per match, by event name.
    pub avg_dealt: BTreeMap<String, f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct BatchResult {
    pub scenario: String,
    pub runs: u32,
    pub first_seed: u64,
    /// Matches with no single winner.
    pub draws: u32,
    pub avg_duration_s: f64,
    pub teams: BTreeMap<String, TeamRecord>,
}

fn wilson(wins: u32, n: u32) -> (f64, f64) {
    if n == 0 {
        return (0.0, 0.0);
    }
    let (z, n, p) = (1.96, n as f64, wins as f64 / n as f64);
    let centre = p + z * z / (2.0 * n);
    let margin = z * (p * (1.0 - p) / n + z * z / (4.0 * n * n)).sqrt();
    let denom = 1.0 + z * z / n;
    (
        ((centre - margin) / denom).max(0.0),
        ((centre + margin) / denom).min(1.0),
    )
}

/// Runs `runs` matches with seeds `first_seed, first_seed + 1, …`.
/// The result does not depend on how many threads are used.
pub fn run_batch(
    content: &Content,
    scenario: &Scenario,
    first_seed: u64,
    runs: u32,
) -> Result<BatchResult, String> {
    let results: Result<Vec<_>, String> = (0..runs)
        .into_par_iter()
        .map(|i| Match::new(content, scenario, first_seed + i as u64).map(|mut m| m.run()))
        .collect();
    let results = results?;

    let mut teams: BTreeMap<String, TeamRecord> = scenario
        .teams
        .iter()
        .map(|t| (t.name.clone(), TeamRecord::default()))
        .collect();
    let mut draws = 0;
    let mut duration = 0.0;
    for r in &results {
        duration += r.duration_s;
        match &r.winner {
            Some(w) => teams.get_mut(w).expect("winner is a scenario team").wins += 1,
            None => draws += 1,
        }
        for unit in &r.units {
            let team = teams
                .get_mut(&unit.team)
                .expect("unit belongs to a scenario team");
            for (name, value) in &unit.dealt {
                *team.avg_dealt.entry(name.clone()).or_insert(0.0) += value / runs as f64;
            }
        }
    }
    for record in teams.values_mut() {
        record.win_rate = if runs > 0 {
            record.wins as f64 / runs as f64
        } else {
            0.0
        };
        (record.win_rate_low, record.win_rate_high) = wilson(record.wins, runs);
    }
    Ok(BatchResult {
        scenario: scenario.name.clone(),
        runs,
        first_seed,
        draws,
        avg_duration_s: if runs > 0 {
            duration / runs as f64
        } else {
            0.0
        },
        teams,
    })
}

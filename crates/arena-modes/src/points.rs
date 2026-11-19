use crate::{rank, GameMode, MatchInfo, Outcome, Status};
use arena_kernel::{EntityId, Event, TeamId, Vec2, World};
use serde::Deserialize;
use std::collections::BTreeSet;

/// A circle that pays points to the only team standing in it.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneConfig {
    pub name: String,
    pub at: [f64; 2],
    pub radius: f64,
    pub points_per_s: f64,
}

/// Team that owns neutral mode entities such as zones.
const NEUTRAL: TeamId = TeamId(u16::MAX);

/// First team to reach the target score. Points come from kills and from holding zones.
#[derive(Clone, Debug)]
pub struct Points {
    target: f64,
    per_kill: f64,
    zones: Vec<ZoneConfig>,
    zone_entities: Vec<(EntityId, f64)>,
    info: MatchInfo,
}

impl Points {
    pub fn new(target: f64, per_kill: f64, zones: Vec<ZoneConfig>) -> Points {
        Points {
            target,
            per_kill,
            zones,
            zone_entities: Vec::new(),
            info: MatchInfo::default(),
        }
    }
}

impl GameMode for Points {
    fn name(&self) -> &'static str {
        "points"
    }

    fn setup(&mut self, w: &mut World, info: &MatchInfo) {
        self.info = info.clone();
        for z in &self.zones {
            let id = w.spawn_region(&z.name, NEUTRAL, Vec2::new(z.at[0], z.at[1]), z.radius);
            self.zone_entities.push((id, z.points_per_s));
        }
    }

    fn on_event(&mut self, w: &mut World, ev: &Event) {
        if ev.kind != w.defs.reg.ev.died {
            return;
        }
        let (Some(killer), Some(victim)) = (ev.source, ev.target) else {
            return;
        };
        if !w.hostile(killer, victim) {
            return;
        }
        if let Some(team) = w.ent(killer).map(|e| e.team) {
            w.add_score(team, self.per_kill);
        }
    }

    fn on_tick(&mut self, w: &mut World) {
        let secs = w.tick_cfg.tick_us as f64 / 1e6;
        for (zone, rate) in &self.zone_entities {
            let inside = w
                .ent(*zone)
                .and_then(|e| e.region.as_ref())
                .map(|r| r.inside.clone());
            let teams: BTreeSet<TeamId> = inside
                .unwrap_or_default()
                .iter()
                .filter_map(|u| w.ent(*u))
                .map(|e| e.team)
                .collect();
            // A contested zone pays nobody.
            if let (1, Some(team)) = (teams.len(), teams.first()) {
                w.add_score(*team, rate * secs);
            }
        }
    }

    fn status(&self, w: &World) -> Status {
        let standings = self.standings(w);
        if !standings.iter().any(|(_, s)| *s >= self.target) {
            return Status::Ongoing;
        }
        Status::Finished(Outcome {
            placements: rank(standings),
            reason: "points".into(),
        })
    }

    fn standings(&self, w: &World) -> Vec<(TeamId, f64)> {
        let score = |t: TeamId| w.scores.get(&t).copied().unwrap_or(0.0);
        self.info
            .teams
            .iter()
            .map(|t| (t.id, score(t.id)))
            .collect()
    }

    fn clone_box(&self) -> Box<dyn GameMode> {
        Box::new(self.clone())
    }
}

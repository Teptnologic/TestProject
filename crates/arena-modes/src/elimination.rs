use crate::{place_survivors_then_out, team_vitality, GameMode, MatchInfo, Outcome, Status};
use arena_kernel::{EntityId, Event, SimTime, TeamId, World};
use std::collections::BTreeMap;

/// Last team standing. A team is out when every member has used all its lives.
#[derive(Clone, Debug)]
pub struct Elimination {
    lives: u32,
    info: MatchInfo,
    deaths: BTreeMap<EntityId, u32>,
    out: Vec<(SimTime, TeamId)>,
}

impl Elimination {
    pub fn new(lives: u32) -> Elimination {
        Elimination {
            lives: lives.max(1),
            info: MatchInfo::default(),
            deaths: BTreeMap::new(),
            out: Vec::new(),
        }
    }

    fn spent(&self, unit: EntityId) -> bool {
        self.deaths.get(&unit).copied().unwrap_or(0) >= self.lives
    }

    fn is_out(&self, team: TeamId) -> bool {
        self.out.iter().any(|(_, t)| *t == team)
    }
}

impl GameMode for Elimination {
    fn name(&self) -> &'static str {
        "elimination"
    }

    fn setup(&mut self, _: &mut World, info: &MatchInfo) {
        self.info = info.clone();
    }

    fn on_event(&mut self, w: &mut World, ev: &Event) {
        if ev.kind != w.defs.reg.ev.died {
            return;
        }
        let Some(unit) = ev.target else { return };
        let Some(team) = self
            .info
            .teams
            .iter()
            .find(|t| t.members.iter().any(|(m, _)| *m == unit))
        else {
            return;
        };
        *self.deaths.entry(unit).or_insert(0) += 1;
        if !self.is_out(team.id) && team.members.iter().all(|(m, _)| self.spent(*m)) {
            self.out.push((ev.time, team.id));
        }
    }

    fn on_tick(&mut self, _: &mut World) {}

    fn status(&self, w: &World) -> Status {
        if self.info.teams.len() - self.out.len() > 1 {
            return Status::Ongoing;
        }
        let survivors = self
            .standings(w)
            .into_iter()
            .filter(|(t, _)| !self.is_out(*t))
            .collect();
        Status::Finished(Outcome {
            placements: place_survivors_then_out(survivors, &self.out),
            reason: "elimination".into(),
        })
    }

    fn standings(&self, w: &World) -> Vec<(TeamId, f64)> {
        let standing = |t: &crate::TeamInfo| {
            let lives_left: u32 = t
                .members
                .iter()
                .map(|(m, _)| {
                    self.lives
                        .saturating_sub(self.deaths.get(m).copied().unwrap_or(0))
                })
                .sum();
            // Lives count first; remaining health breaks ties.
            let health = team_vitality(w, t.members.iter().map(|(m, _)| *m));
            lives_left as f64 + health / (t.members.len().max(1) as f64 + 1.0)
        };
        self.info
            .teams
            .iter()
            .map(|t| (t.id, if self.is_out(t.id) { -1.0 } else { standing(t) }))
            .collect()
    }

    fn may_respawn(&self, unit: EntityId) -> bool {
        !self.spent(unit)
    }

    fn clone_box(&self) -> Box<dyn GameMode> {
        Box::new(self.clone())
    }
}

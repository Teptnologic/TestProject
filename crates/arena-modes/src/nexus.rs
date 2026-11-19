use crate::{place_survivors_then_out, team_vitality, GameMode, MatchInfo, Outcome, Status};
use arena_kernel::{Event, SimTime, TeamId, World};

/// Each team protects its objectives. A team is out when all of them are destroyed.
#[derive(Clone, Debug, Default)]
pub struct Nexus {
    info: MatchInfo,
    out: Vec<(SimTime, TeamId)>,
}

impl Nexus {
    fn is_out(&self, team: TeamId) -> bool {
        self.out.iter().any(|(_, t)| *t == team)
    }
}

impl GameMode for Nexus {
    fn name(&self) -> &'static str {
        "nexus"
    }

    fn setup(&mut self, _: &mut World, info: &MatchInfo) {
        self.info = info.clone();
    }

    fn on_event(&mut self, w: &mut World, ev: &Event) {
        if ev.kind != w.defs.reg.ev.died {
            return;
        }
        let Some(dead) = ev.target else { return };
        let Some(team) = self
            .info
            .teams
            .iter()
            .find(|t| t.objectives.contains(&dead))
        else {
            return;
        };
        if !self.is_out(team.id) && team.objectives.iter().all(|o| !w.is_alive(*o)) {
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
            reason: "nexus destroyed".into(),
        })
    }

    fn standings(&self, w: &World) -> Vec<(TeamId, f64)> {
        let health = |t: &crate::TeamInfo| team_vitality(w, t.objectives.iter().copied());
        self.info
            .teams
            .iter()
            .map(|t| (t.id, if self.is_out(t.id) { -1.0 } else { health(t) }))
            .collect()
    }

    fn clone_box(&self) -> Box<dyn GameMode> {
        Box::new(self.clone())
    }
}

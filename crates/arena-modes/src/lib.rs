//! Game modes: what a match is about and how it ends.
//!
//! A [`GameMode`] watches events, may add its own entities (zones) and scores,
//! and reports whether the match is over and who placed where. Modes are
//! independent of any ruleset: they only use the kernel's generic machinery.
//! Respawning is a separate rule ([`Respawner`]) that modes can veto.

mod elimination;
mod nexus;
mod points;
mod respawn;

pub use elimination::Elimination;
pub use nexus::Nexus;
pub use points::{Points, ZoneConfig};
pub use respawn::Respawner;

use arena_kernel::{EntityId, Event, TeamId, Vec2, World};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct TeamInfo {
    pub id: TeamId,
    pub name: String,
    /// Each member and where it spawns.
    pub members: Vec<(EntityId, Vec2)>,
    /// Structures the team must protect.
    pub objectives: Vec<EntityId>,
}

/// What a mode is told about the match at setup.
#[derive(Clone, Debug, Default)]
pub struct MatchInfo {
    pub teams: Vec<TeamInfo>,
}

impl MatchInfo {
    pub fn team_of(&self, unit: EntityId) -> Option<TeamId> {
        let has = |t: &&TeamInfo| {
            t.members.iter().any(|(m, _)| *m == unit) || t.objectives.contains(&unit)
        };
        self.teams.iter().find(has).map(|t| t.id)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Outcome {
    /// Best first. Teams in the same group tied.
    pub placements: Vec<Vec<TeamId>>,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Ongoing,
    Finished(Outcome),
}

pub trait GameMode: Send {
    fn name(&self) -> &'static str;
    /// Called once, after every unit has been spawned.
    fn setup(&mut self, w: &mut World, info: &MatchInfo);
    fn on_event(&mut self, w: &mut World, ev: &Event);
    fn on_tick(&mut self, w: &mut World);
    fn status(&self, w: &World) -> Status;
    /// How each team is doing, higher is better. Decides placements when time runs out.
    fn standings(&self, w: &World) -> Vec<(TeamId, f64)>;
    /// Whether a dead unit may come back.
    fn may_respawn(&self, _unit: EntityId) -> bool {
        true
    }
    fn clone_box(&self) -> Box<dyn GameMode>;
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ModeConfig {
    /// Last team with a living member wins. Each unit has `lives` lives.
    Elimination {
        #[serde(default = "one")]
        lives: u32,
    },
    /// First team to `target` points wins.
    Points {
        target: f64,
        #[serde(default)]
        per_kill: f64,
        #[serde(default)]
        zones: Vec<ZoneConfig>,
    },
    /// A team is out when its objectives are destroyed.
    Nexus,
    /// Ends as soon as any of the listed modes ends.
    AnyOf { modes: Vec<ModeConfig> },
}

fn one() -> u32 {
    1
}

pub fn build(cfg: &ModeConfig) -> Box<dyn GameMode> {
    match cfg {
        ModeConfig::Elimination { lives } => Box::new(Elimination::new(*lives)),
        ModeConfig::Points {
            target,
            per_kill,
            zones,
        } => Box::new(Points::new(*target, *per_kill, zones.clone())),
        ModeConfig::Nexus => Box::new(Nexus::default()),
        ModeConfig::AnyOf { modes } => Box::new(AnyOf {
            modes: modes.iter().map(build).collect(),
        }),
    }
}

/// Groups teams by standing, best first; equal standings share a place.
pub fn rank(mut standings: Vec<(TeamId, f64)>) -> Vec<Vec<TeamId>> {
    standings.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    let mut out: Vec<Vec<TeamId>> = Vec::new();
    let mut last = f64::NAN;
    for (team, score) in standings {
        if score == last {
            out.last_mut().expect("a previous group exists").push(team);
        } else {
            out.push(vec![team]);
        }
        last = score;
    }
    out
}

/// Total vitality of a team's living units.
fn team_vitality(w: &World, units: impl Iterator<Item = EntityId>) -> f64 {
    units
        .filter(|u| w.is_alive(*u))
        .map(|u| w.ruleset.vitality(w, u))
        .sum()
}

/// Places the surviving teams by standing, then knocked-out teams with the
/// latest knock-out first. Teams knocked out at the same instant tie.
fn place_survivors_then_out(
    survivors: Vec<(TeamId, f64)>,
    out: &[(arena_kernel::SimTime, TeamId)],
) -> Vec<Vec<TeamId>> {
    let mut placements = rank(survivors);
    let mut out = out.to_vec();
    out.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut last = None;
    for (time, team) in out {
        if last == Some(time) {
            placements
                .last_mut()
                .expect("a previous group exists")
                .push(team);
        } else {
            placements.push(vec![team]);
        }
        last = Some(time);
    }
    placements
}

struct AnyOf {
    modes: Vec<Box<dyn GameMode>>,
}

impl GameMode for AnyOf {
    fn name(&self) -> &'static str {
        "any_of"
    }

    fn setup(&mut self, w: &mut World, info: &MatchInfo) {
        self.modes.iter_mut().for_each(|m| m.setup(w, info));
    }

    fn on_event(&mut self, w: &mut World, ev: &Event) {
        self.modes.iter_mut().for_each(|m| m.on_event(w, ev));
    }

    fn on_tick(&mut self, w: &mut World) {
        self.modes.iter_mut().for_each(|m| m.on_tick(w));
    }

    fn status(&self, w: &World) -> Status {
        let finished = self
            .modes
            .iter()
            .map(|m| m.status(w))
            .find(|s| *s != Status::Ongoing);
        finished.unwrap_or(Status::Ongoing)
    }

    fn standings(&self, w: &World) -> Vec<(TeamId, f64)> {
        self.modes
            .first()
            .map(|m| m.standings(w))
            .unwrap_or_default()
    }

    fn may_respawn(&self, unit: EntityId) -> bool {
        self.modes.iter().all(|m| m.may_respawn(unit))
    }

    fn clone_box(&self) -> Box<dyn GameMode> {
        Box::new(AnyOf {
            modes: self.modes.iter().map(|m| m.clone_box()).collect(),
        })
    }
}

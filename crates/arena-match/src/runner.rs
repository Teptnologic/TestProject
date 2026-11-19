//! Staging and the match loop.

use crate::scenario::{LoadoutConfig, PolicyConfig, Scenario};
use crate::stats::{MatchResult, Stats};
use crate::Content;
use arena_agents::{Idle, Policy, Random, Scripted, StagingView};
use arena_kernel::*;
use arena_modes::{rank, GameMode, MatchInfo, Outcome, Respawner, Status, TeamInfo};
use serde::{Deserialize, Serialize};

/// Everything needed to reproduce a match exactly: the seed, what each unit
/// took into the fight, and every command that was accepted.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Replay {
    pub seed: u64,
    /// Perk names per member, in scenario order.
    pub loadouts: Vec<Vec<String>>,
    /// `(tick, unit, command)`.
    pub commands: Vec<(u64, EntityId, Command)>,
}

struct Agent {
    entity: EntityId,
    policy: Box<dyn Policy>,
    /// Which tick of each decision interval this agent decides on. Drawn at
    /// random per match, like players whose reactions are not synchronised.
    phase: u64,
}

pub struct Match {
    pub world: World,
    pub info: MatchInfo,
    /// Events from the most recent tick, for logs and viewers.
    pub last_events: Vec<Event>,
    scenario_name: String,
    mode: Box<dyn GameMode>,
    respawner: Option<Respawner>,
    agents: Vec<Agent>,
    stats: Stats,
    decision_ticks: u64,
    timeout: SimTime,
    replay: Replay,
    outcome: Option<Outcome>,
}

fn make_policy(cfg: PolicyConfig, content: &Content, unit: UnitId, seed: u64) -> Box<dyn Policy> {
    match cfg {
        PolicyConfig::Scripted => match content.pack.ai.get(&unit) {
            Some(script) => Box::new(Scripted::new(script.clone())),
            None => Box::new(Scripted::new(Default::default())),
        },
        PolicyConfig::Idle => Box::new(Idle),
        PolicyConfig::Random => Box::new(Random::new(seed)),
    }
}

impl Match {
    /// Runs staging and spawns everything. Policies choose loadouts where the scenario allows.
    pub fn new(content: &Content, scenario: &Scenario, seed: u64) -> Result<Match, String> {
        Self::build(content, scenario, seed, None)
    }

    /// Rebuilds a recorded match. Step it with [`Match::run_replay`].
    pub fn from_replay(
        content: &Content,
        scenario: &Scenario,
        replay: &Replay,
    ) -> Result<Match, String> {
        Self::build(content, scenario, replay.seed, Some(&replay.loadouts))
    }

    fn build(
        content: &Content,
        scenario: &Scenario,
        seed: u64,
        recorded: Option<&[Vec<String>]>,
    ) -> Result<Match, String> {
        let defs = &content.defs;
        let cfg = WorldConfig {
            tick: TickConfig {
                tick_us: scenario.tick_us,
                snap: true,
            },
            seed,
            size: Vec2::new(scenario.map.width, scenario.map.height),
        };
        let mut world = World::new(defs.clone(), content.ruleset.clone(), cfg);
        let unit_id = |name: &str| {
            defs.unit_id(name)
                .ok_or_else(|| format!("unknown unit `{name}`"))
        };
        let perk_ids = |names: &[String]| -> Result<Vec<PerkId>, String> {
            names
                .iter()
                .map(|n| defs.perk_id(n).ok_or_else(|| format!("unknown perk `{n}`")))
                .collect()
        };

        let mut info = MatchInfo::default();
        let mut agents = Vec::new();
        let mut stats = Stats::default();
        let mut replay = Replay {
            seed,
            ..Replay::default()
        };
        let mut member_index = 0;
        for (t, team) in scenario.teams.iter().enumerate() {
            let team_id = TeamId(t as u16);
            let mut team_info = TeamInfo {
                id: team_id,
                name: team.name.clone(),
                members: Vec::new(),
                objectives: Vec::new(),
            };
            for member in &team.members {
                let unit = unit_id(&member.unit)?;
                // Each agent gets its own random stream so policies never disturb the world's.
                let policy_seed = seed ^ ((member_index as u64 + 1) << 32);
                let mut policy = make_policy(member.policy, content, unit, policy_seed);

                // Staging: a recorded choice, the scenario's choice, or the policy's.
                let perks = match (recorded, &member.loadout) {
                    (Some(all), _) => {
                        perk_ids(all.get(member_index).ok_or("replay has too few loadouts")?)?
                    }
                    (None, LoadoutConfig::Fixed(names)) => perk_ids(names)?,
                    (None, LoadoutConfig::Policy) => policy.stage(&StagingView {
                        defs,
                        ruleset: content.ruleset.as_ref(),
                        unit,
                        budget: member.gold,
                    }),
                };
                content
                    .ruleset
                    .validate_loadout(defs, defs.unit(unit), &perks, member.gold)
                    .map_err(|e| format!("team {}: {e}", team.name))?;
                replay
                    .loadouts
                    .push(perks.iter().map(|p| defs.perk(*p).name.clone()).collect());

                let jitter = |w: &mut World| (w.rng.next_f64() * 2.0 - 1.0) * scenario.spawn_jitter;
                let (jx, jy) = (jitter(&mut world), jitter(&mut world));
                let pos = Vec2::new(member.spawn[0] + jx, member.spawn[1] + jy).clamp(world.size);
                let id = world.spawn_unit(SpawnUnit {
                    unit,
                    team: team_id,
                    pos,
                    level: member.level,
                    perks,
                    ranks: None,
                });
                team_info.members.push((id, pos));
                stats.track(id, &team.name, &member.unit);
                let phase = world.rng.below(scenario.decision_ticks);
                agents.push(Agent {
                    entity: id,
                    policy,
                    phase,
                });
                member_index += 1;
            }
            for objective in &team.objectives {
                let pos = Vec2::new(objective.spawn[0], objective.spawn[1]);
                let id = world.spawn_unit(SpawnUnit {
                    unit: unit_id(&objective.unit)?,
                    team: team_id,
                    pos,
                    level: 1,
                    perks: Vec::new(),
                    ranks: None,
                });
                team_info.objectives.push(id);
                stats.track(id, &team.name, &objective.unit);
            }
            info.teams.push(team_info);
        }

        let mut mode = arena_modes::build(&scenario.mode);
        mode.setup(&mut world, &info);
        let respawner = scenario.respawn_s.map(|s| Respawner::new(s, &info));
        world.drain_events();
        Ok(Match {
            world,
            info,
            last_events: Vec::new(),
            scenario_name: scenario.name.clone(),
            mode,
            respawner,
            agents,
            stats,
            decision_ticks: scenario.decision_ticks,
            timeout: SimTime(micros(scenario.timeout_s)),
            replay,
            outcome: None,
        })
    }

    pub fn outcome(&self) -> Option<&Outcome> {
        self.outcome.as_ref()
    }

    pub fn replay(&self) -> &Replay {
        &self.replay
    }

    /// Units that take commands, in the order policies are asked.
    pub fn agents(&self) -> Vec<EntityId> {
        self.agents.iter().map(|a| a.entity).collect()
    }

    /// Advances one tick, asking each built-in policy for a command when its decision is due.
    pub fn step(&mut self) {
        let mut commands = Vec::new();
        if self.outcome.is_none() {
            let tick = self.world.tick;
            for agent in &mut self.agents {
                if (tick + agent.phase).is_multiple_of(self.decision_ticks) {
                    commands.push((agent.entity, agent.policy.act(&self.world, agent.entity)));
                }
            }
        }
        self.step_with(&commands);
    }

    /// Advances one tick with commands supplied by the caller. Illegal commands are ignored.
    pub fn step_with(&mut self, commands: &[(EntityId, Command)]) {
        if self.outcome.is_some() {
            return;
        }
        let tick = self.world.tick;
        for (entity, cmd) in commands {
            if *cmd != Command::None && self.world.submit(*entity, *cmd).is_ok() {
                self.replay.commands.push((tick, *entity, *cmd));
            }
        }
        self.world.tick();

        // Mode and respawn reactions raise events too; they are picked up on the next tick.
        self.last_events = self.world.drain_events();
        for ev in &self.last_events {
            self.stats.record(&self.world, ev);
            self.mode.on_event(&mut self.world, ev);
            if let Some(r) = &mut self.respawner {
                r.on_event(&self.world, ev, self.mode.as_ref());
            }
        }
        self.mode.on_tick(&mut self.world);
        if let Some(r) = &mut self.respawner {
            r.on_tick(&mut self.world);
        }

        if let Status::Finished(outcome) = self.mode.status(&self.world) {
            self.outcome = Some(outcome);
        } else if self.world.now >= self.timeout {
            let placements = rank(self.mode.standings(&self.world));
            self.outcome = Some(Outcome {
                placements,
                reason: "timeout".into(),
            });
        }
    }

    /// Plays to the end with the built-in policies.
    pub fn run(&mut self) -> MatchResult {
        while self.outcome.is_none() {
            self.step();
        }
        self.result()
    }

    /// Plays to the end feeding back the commands of a recording.
    pub fn run_replay(&mut self, replay: &Replay) -> MatchResult {
        let mut next = 0;
        while self.outcome.is_none() {
            let tick = self.world.tick;
            let start = next;
            while replay.commands.get(next).is_some_and(|c| c.0 == tick) {
                next += 1;
            }
            let commands: Vec<_> = replay.commands[start..next]
                .iter()
                .map(|c| (c.1, c.2))
                .collect();
            self.step_with(&commands);
        }
        self.result()
    }

    /// The report so far; final once [`Match::outcome`] is set.
    pub fn result(&self) -> MatchResult {
        let team_name = |t: &TeamId| self.info.teams[t.idx()].name.clone();
        let outcome = self.outcome.clone().unwrap_or(Outcome {
            placements: Vec::new(),
            reason: "ongoing".into(),
        });
        let placements: Vec<Vec<String>> = outcome
            .placements
            .iter()
            .map(|g| g.iter().map(team_name).collect())
            .collect();
        let winner = placements
            .first()
            .filter(|g| g.len() == 1)
            .map(|g| g[0].clone());
        let mut units = Vec::new();
        for (id, report) in &self.stats.units {
            let mut r = report.clone();
            r.alive = self.world.is_alive(*id);
            r.vitality = if r.alive {
                self.world.ruleset.vitality(&self.world, *id)
            } else {
                0.0
            };
            units.push(r);
        }
        MatchResult {
            scenario: self.scenario_name.clone(),
            seed: self.replay.seed,
            placements,
            winner,
            reason: outcome.reason,
            duration_s: self.world.now.secs(),
            ticks: self.world.tick,
            scores: self
                .world
                .scores
                .iter()
                .filter(|(t, _)| t.idx() < self.info.teams.len())
                .map(|(t, s)| (team_name(t), *s))
                .collect(),
            units,
            state_hash: self.world.state_hash(),
        }
    }

    /// A readable name for an entity, e.g. `blue/garen#1`.
    pub fn label(&self, id: EntityId) -> String {
        match (self.stats.units.get(&id), self.world.ent(id)) {
            (Some(r), _) => format!("{}/{}#{}", r.team, r.unit, id.0),
            (None, Some(e)) => format!("{:?}#{}", e.kind, id.0).to_lowercase(),
            (None, None) => format!("#{}", id.0),
        }
    }
}

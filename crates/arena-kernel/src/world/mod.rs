//! The world: all mutable simulation state plus the machinery that advances it.
//!
//! One [`World::tick`] does, in order: fire due timers (action stages, buff
//! expiry and ticks, regeneration), move units and projectiles, then update
//! region occupancy. Agents act between ticks through [`World::submit`].

mod actions;
mod buffs;
mod effects;
mod movement;

pub use buffs::BuffOpts;

use crate::defs::{Defs, Filter};
use crate::entity::{Entity, EntityKind, Region, Slot};
use crate::event::Event;
use crate::ids::*;
use crate::math::Vec2;
use crate::registry::{Ex, ResPart, StatPart, VarRef, Who};
use crate::rng::Rng;
use crate::ruleset::Ruleset;
use crate::stats::{ModSrc, Modifier, StatBlock};
use crate::time::{SimTime, TickConfig};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

/// Listener chains deeper than this are cut off.
pub const MAX_EVENT_DEPTH: u8 = 6;
const REGEN_INTERVAL_US: u64 = 500_000;

#[derive(Clone, Copy, Debug)]
pub struct WorldConfig {
    pub tick: TickConfig,
    pub seed: u64,
    /// The map is the rectangle from the origin to this point.
    pub size: Vec2,
}

/// What an effect is being applied by, to and with.
#[derive(Clone, Copy, Debug)]
pub struct Ctx {
    pub source: EntityId,
    pub target: Option<EntityId>,
    pub point: Option<Vec2>,
    pub action: Option<ActionId>,
    pub rank: u8,
    pub stacks: u32,
    pub value: f64,
    pub depth: u8,
}

impl Ctx {
    pub fn new(source: EntityId) -> Ctx {
        Ctx {
            source,
            target: None,
            point: None,
            action: None,
            rank: 0,
            stacks: 0,
            value: 0.0,
            depth: 0,
        }
    }

    pub fn with_target(mut self, target: EntityId) -> Ctx {
        self.target = Some(target);
        self
    }
}

pub struct SpawnUnit {
    pub unit: UnitId,
    pub team: TeamId,
    pub pos: Vec2,
    pub level: u32,
    pub perks: Vec<PerkId>,
    /// Ranks for the unit's own slots; `None` uses the ruleset's default.
    pub ranks: Option<Vec<u8>>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum Timer {
    Regen,
    BuffTick { entity: EntityId, uid: u64 },
    ActionStage { entity: EntityId, token: u64 },
    BuffExpire { entity: EntityId, uid: u64 },
}

impl Timer {
    /// Order among timers due at the same instant.
    fn phase(&self) -> u8 {
        match self {
            Timer::Regen => 0,
            Timer::BuffTick { .. } => 1,
            Timer::ActionStage { .. } => 2,
            Timer::BuffExpire { .. } => 3,
        }
    }
}

/// Everything that changes during a match. Cloning it is a snapshot.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldState {
    pub now: SimTime,
    pub tick: u64,
    pub tick_cfg: TickConfig,
    pub size: Vec2,
    pub rng: Rng,
    pub entities: BTreeMap<EntityId, Entity>,
    pub scores: BTreeMap<TeamId, f64>,
    timers: BTreeMap<(SimTime, u8, u64), Timer>,
    /// Units that reached a lethal state at the current instant: (victim, killer).
    pending_deaths: Vec<(EntityId, Option<EntityId>)>,
    next_entity: u32,
    next_uid: u64,
}

pub struct World {
    state: WorldState,
    pub defs: Arc<Defs>,
    pub ruleset: Arc<dyn Ruleset>,
    outbox: Vec<Event>,
}

impl Deref for World {
    type Target = WorldState;

    fn deref(&self) -> &WorldState {
        &self.state
    }
}

impl DerefMut for World {
    fn deref_mut(&mut self) -> &mut WorldState {
        &mut self.state
    }
}

impl World {
    pub fn new(defs: Arc<Defs>, ruleset: Arc<dyn Ruleset>, cfg: WorldConfig) -> World {
        let mut w = World {
            state: WorldState {
                now: SimTime::ZERO,
                tick: 0,
                tick_cfg: cfg.tick,
                size: cfg.size,
                rng: Rng::new(cfg.seed),
                entities: BTreeMap::new(),
                scores: BTreeMap::new(),
                timers: BTreeMap::new(),
                pending_deaths: Vec::new(),
                next_entity: 1,
                next_uid: 1,
            },
            defs,
            ruleset,
            outbox: Vec::new(),
        };
        w.schedule(SimTime(REGEN_INTERVAL_US), Timer::Regen);
        w
    }

    // ----- snapshots -----

    pub fn snapshot(&self) -> WorldState {
        self.state.clone()
    }

    pub fn restore(&mut self, state: WorldState) {
        self.state = state;
        self.outbox.clear();
    }

    /// Hash of the full state. Equal hashes mean identical worlds.
    pub fn state_hash(&self) -> u64 {
        let bytes = bincode::serialize(&self.state).expect("world state serialises");
        // FNV-1a, so the hash is stable across platforms and compiler versions.
        bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
            (h ^ *b as u64).wrapping_mul(0x100_0000_01b3)
        })
    }

    // ----- queries -----

    pub fn ent(&self, id: EntityId) -> Option<&Entity> {
        self.state.entities.get(&id)
    }

    pub fn ent_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.state.entities.get_mut(&id)
    }

    /// Living units, in id order.
    pub fn units(&self) -> impl Iterator<Item = &Entity> {
        self.state
            .entities
            .values()
            .filter(|e| e.is_unit() && e.alive)
    }

    pub fn is_alive(&self, id: EntityId) -> bool {
        self.ent(id).is_some_and(|e| e.alive)
    }

    pub fn stat(&self, id: EntityId, stat: StatId) -> f64 {
        self.ent(id).map_or(0.0, |e| e.stats.total(stat))
    }

    pub fn resource(&self, id: EntityId, res: ResourceId) -> f64 {
        self.ent(id)
            .and_then(|e| e.resources.get(res.idx()).copied())
            .unwrap_or(0.0)
    }

    pub fn resource_max(&self, id: EntityId, res: ResourceId) -> f64 {
        self.stat(id, self.defs.reg.resources[res.idx()].max_stat)
    }

    /// Sets a resource, clamped to `0..=max`.
    pub fn set_resource(&mut self, id: EntityId, res: ResourceId, value: f64) {
        let max = self.resource_max(id, res);
        if let Some(slot) = self
            .ent_mut(id)
            .and_then(|e| e.resources.get_mut(res.idx()))
        {
            *slot = value.max(0.0).min(max);
        }
    }

    pub fn has_flag(&self, id: EntityId, flag: FlagId) -> bool {
        let Some(e) = self.ent(id) else { return false };
        e.buffs
            .iter()
            .any(|b| self.defs.buff(b.def).flags.contains(&flag))
    }

    pub fn unit_has_tag(&self, id: EntityId, tag: TagId) -> bool {
        self.ent(id)
            .and_then(|e| e.unit)
            .is_some_and(|u| self.defs.unit(u).tags.contains(&tag))
    }

    pub fn hostile(&self, a: EntityId, b: EntityId) -> bool {
        match (self.ent(a), self.ent(b)) {
            (Some(x), Some(y)) => x.team != y.team,
            _ => false,
        }
    }

    pub fn passes(&self, filter: Filter, source: EntityId, target: EntityId) -> bool {
        match filter {
            Filter::Enemy => self.hostile(source, target),
            Filter::Ally => !self.hostile(source, target),
            Filter::Any => true,
        }
    }

    /// Distance between the edges of two entities (zero when they overlap).
    pub fn edge_distance(&self, a: EntityId, b: EntityId) -> f64 {
        match (self.ent(a), self.ent(b)) {
            (Some(x), Some(y)) => (x.pos.dist(y.pos) - x.radius - y.radius).max(0.0),
            _ => f64::INFINITY,
        }
    }

    /// Living units whose body touches the circle, in id order.
    pub fn units_in_circle(&self, center: Vec2, radius: f64) -> Vec<EntityId> {
        self.units()
            .filter(|e| e.pos.dist(center) <= radius + e.radius)
            .map(|e| e.id)
            .collect()
    }

    pub fn eval(&self, ex: &Ex, ctx: &Ctx) -> f64 {
        ex.eval(&|v| self.var(v, ctx))
    }

    fn var(&self, v: &VarRef, ctx: &Ctx) -> f64 {
        let who = |w: Who| {
            let id = match w {
                Who::Source => Some(ctx.source),
                Who::Target => ctx.target,
            };
            id.and_then(|id| self.ent(id))
        };
        match *v {
            VarRef::Stat(w, stat, part) => who(w).map_or(0.0, |e| match part {
                StatPart::Total => e.stats.total(stat),
                StatPart::Base => e.stats.base(stat),
                StatPart::Bonus => e.stats.bonus(stat),
            }),
            VarRef::Resource(w, res, part) => who(w).map_or(0.0, |e| {
                let cur = e.resources.get(res.idx()).copied().unwrap_or(0.0);
                let max = e.stats.total(self.defs.reg.resources[res.idx()].max_stat);
                match part {
                    ResPart::Current => cur,
                    ResPart::Missing => (max - cur).max(0.0),
                    ResPart::Pct if max > 0.0 => cur / max,
                    ResPart::Pct => 0.0,
                }
            }),
            VarRef::Level(w) => who(w).map_or(0.0, |e| e.level as f64),
            VarRef::BuffStacks(w, buff) => who(w)
                .and_then(|e| e.buff(buff))
                .map_or(0.0, |b| b.stacks as f64),
            VarRef::Rank => ctx.rank as f64,
            VarRef::Stacks => ctx.stacks as f64,
            VarRef::Value => ctx.value,
            VarRef::Distance => {
                let from = self.ent(ctx.source).map(|e| e.pos);
                let to = ctx
                    .target
                    .and_then(|t| self.ent(t))
                    .map(|e| e.pos)
                    .or(ctx.point);
                match (from, to) {
                    (Some(a), Some(b)) => a.dist(b),
                    _ => 0.0,
                }
            }
        }
    }

    // ----- events -----

    /// Raises an event: logs it, lets the ruleset react, then runs data-defined listeners.
    pub fn emit(&mut self, ev: Event) {
        self.outbox.push(ev.clone());
        let ruleset = self.ruleset.clone();
        ruleset.on_event(self, &ev);
        if ev.depth < MAX_EVENT_DEPTH {
            self.run_listeners(&ev);
        }
    }

    pub fn event(
        &self,
        kind: EventKind,
        source: Option<EntityId>,
        target: Option<EntityId>,
    ) -> Event {
        Event {
            kind,
            time: self.now,
            source,
            target,
            value: 0.0,
            action: None,
            depth: 0,
        }
    }

    /// Takes the events raised since the last call.
    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.outbox)
    }

    // ----- spawning and death -----

    fn alloc(&mut self) -> EntityId {
        let id = EntityId(self.next_entity);
        self.state.next_entity += 1;
        id
    }

    fn next_uid(&mut self) -> u64 {
        self.state.next_uid += 1;
        self.state.next_uid
    }

    pub fn spawn_unit(&mut self, s: SpawnUnit) -> EntityId {
        let id = self.alloc();
        let radius = self.defs.unit(s.unit).radius;
        let mut e = Entity::new(id, EntityKind::Unit, s.team, s.pos.clamp(self.size), radius);
        e.unit = Some(s.unit);
        e.level = s.level;
        e.perks = s.perks;
        e.ranks = s.ranks;
        self.state.entities.insert(id, e);
        self.state.scores.entry(s.team).or_insert(0.0);
        self.init_unit(id);
        let ev = self.event(self.defs.reg.ev.spawned, None, Some(id));
        self.emit(ev);
        id
    }

    /// Builds a unit's stats, slots, perks and passives from its definition.
    fn init_unit(&mut self, id: EntityId) {
        let defs = self.defs.clone();
        let ruleset = self.ruleset.clone();
        let Some(e) = self.ent_mut(id) else { return };
        let Some(unit) = e.unit.map(|u| defs.unit(u)) else {
            return;
        };

        e.stats = StatBlock::new(defs.reg.stats.len());
        for (stat, value) in ruleset.base_stats(&defs, unit, e.level) {
            e.stats.set_base(stat, value);
        }
        let ranks = e
            .ranks
            .clone()
            .unwrap_or_else(|| ruleset.default_ranks(&defs, unit, e.level));
        let slot = |key: &str, action, rank| Slot {
            key: key.to_string(),
            action,
            rank,
            ready_at: SimTime::ZERO,
        };
        e.slots = unit
            .slots
            .iter()
            .enumerate()
            .map(|(i, s)| slot(&s.key, s.action, ranks.get(i).copied().unwrap_or(0)))
            .collect();
        let mut buffs = unit.passives.clone();
        for (i, perk) in e.perks.iter().map(|p| defs.perk(*p)).enumerate() {
            for (stat, layer, value) in &perk.modifiers {
                let src = ModSrc::Perk(i as u16);
                e.stats.add(Modifier {
                    src,
                    stat: *stat,
                    layer: *layer,
                    value: *value,
                });
            }
            e.slots
                .extend(perk.slots.iter().map(|s| slot(&s.key, s.action, 1)));
            buffs.extend(&perk.buffs);
        }
        e.resources = vec![0.0; defs.reg.resources.len()];
        e.buffs.clear();
        e.action = None;
        e.queued = None;
        e.move_to = None;
        e.displacement = None;
        e.alive = true;

        ruleset.init_unit(self, id);
        for buff in buffs {
            self.apply_buff(id, buff, id, BuffOpts::default());
        }
    }

    /// Brings a dead unit back at `pos` in its starting condition.
    pub fn respawn(&mut self, id: EntityId, pos: Vec2) {
        let size = self.size;
        let Some(e) = self.ent_mut(id) else { return };
        e.pos = pos.clamp(size);
        self.init_unit(id);
        let ev = self.event(self.defs.reg.ev.spawned, None, Some(id));
        self.emit(ev);
    }

    /// Condemns a unit. It stays alive until everything happening at the same
    /// instant has resolved, so simultaneous blows both land and no unit gains
    /// an edge from being processed first. `died` then names the first killer.
    pub fn kill(&mut self, victim: EntityId, killer: Option<EntityId>) {
        let condemned = self.state.pending_deaths.iter().any(|(v, _)| *v == victim);
        if self.is_alive(victim) && !condemned {
            self.state.pending_deaths.push((victim, killer));
        }
    }

    /// Carries out the deaths queued by [`World::kill`]. Called by the world at
    /// the end of every instant; hosts call it after submitting commands.
    pub fn resolve_deaths(&mut self) {
        while !self.pending_deaths.is_empty() {
            for (victim, killer) in std::mem::take(&mut self.state.pending_deaths) {
                let Some(e) = self.ent_mut(victim) else {
                    continue;
                };
                e.alive = false;
                e.action = None;
                e.queued = None;
                e.move_to = None;
                e.displacement = None;
                e.buffs.clear();
                let ev = self.event(self.defs.reg.ev.died, killer, Some(victim));
                self.emit(ev);
            }
        }
    }

    pub fn spawn_region(&mut self, name: &str, team: TeamId, pos: Vec2, radius: f64) -> EntityId {
        let id = self.alloc();
        let mut e = Entity::new(id, EntityKind::Region, team, pos, radius);
        e.region = Some(Region {
            name: name.to_string(),
            inside: Vec::new(),
        });
        self.state.entities.insert(id, e);
        id
    }

    pub fn despawn(&mut self, id: EntityId) {
        self.state.entities.remove(&id);
    }

    pub fn add_score(&mut self, team: TeamId, amount: f64) {
        *self.state.scores.entry(team).or_insert(0.0) += amount;
    }

    // ----- time -----

    fn schedule(&mut self, at: SimTime, timer: Timer) {
        let at = self.tick_cfg.snap_up(at);
        let seq = self.next_uid();
        self.state.timers.insert((at, timer.phase(), seq), timer);
    }

    /// Advances the world by one tick.
    pub fn tick(&mut self) {
        self.resolve_deaths();
        let end = SimTime((self.tick + 1) * self.tick_cfg.tick_us);
        while let Some(instant) = self.state.timers.first_key_value().map(|(k, _)| k.0) {
            if instant > end {
                break;
            }
            self.state.now = instant.max(self.now);
            // Everything due at this instant happens "at once": deaths wait until all of it is done.
            while let Some(entry) = self
                .state
                .timers
                .first_entry()
                .filter(|e| e.key().0 == instant)
            {
                let timer = entry.remove();
                self.fire(timer);
            }
            self.resolve_deaths();
        }
        self.state.now = end;
        self.state.tick += 1;
        self.advance_movement(self.tick_cfg.tick_us as f64 / 1e6);
        self.resolve_deaths();
    }

    fn fire(&mut self, timer: Timer) {
        match timer {
            Timer::Regen => {
                self.regenerate(REGEN_INTERVAL_US as f64 / 1e6);
                self.schedule(self.now.plus(REGEN_INTERVAL_US), Timer::Regen);
            }
            Timer::BuffTick { entity, uid } => self.buff_tick(entity, uid),
            Timer::ActionStage { entity, token } => self.action_stage(entity, token),
            Timer::BuffExpire { entity, uid } => self.buff_timer_expired(entity, uid),
        }
    }

    fn regenerate(&mut self, secs: f64) {
        let defs = self.defs.clone();
        for e in self
            .state
            .entities
            .values_mut()
            .filter(|e| e.is_unit() && e.alive)
        {
            for (i, res) in defs.reg.resources.iter().enumerate() {
                let Some(regen) = res.regen_stat else {
                    continue;
                };
                let gain = e.stats.total(regen) * res.regen_per_sec * secs;
                let max = e.stats.total(res.max_stat);
                e.resources[i] = (e.resources[i] + gain).min(max).max(0.0);
            }
        }
    }
}

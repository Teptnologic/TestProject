//! Everything that exists in the world is an [`Entity`]: a fixed record with
//! optional parts. Plain data, so the whole world can be cloned, serialised
//! and hashed, and is always iterated in id order.

use crate::command::{Command, Target};
use crate::defs::Filter;
use crate::ids::*;
use crate::math::Vec2;
use crate::stats::StatBlock;
use crate::time::SimTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityKind {
    Unit,
    Projectile,
    Region,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Slot {
    pub key: String,
    pub action: ActionId,
    pub rank: u8,
    pub ready_at: SimTime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Stage {
    Windup,
    Recovery,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunningAction {
    pub slot: usize,
    pub target: Target,
    pub stage: Stage,
    pub ends_at: SimTime,
    /// Matches the timer that will advance this action; stale timers are ignored.
    pub token: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BuffInstance {
    pub uid: u64,
    pub def: BuffId,
    pub source: EntityId,
    pub applied_at: SimTime,
    pub expires_at: Option<SimTime>,
    pub stacks: u32,
    /// Free payload: a shield's remaining amount, a slow's strength.
    pub value: f64,
    /// Rank of the action that applied it.
    pub rank: u8,
    /// Per listener: when its internal cooldown ends.
    pub listener_ready: Vec<SimTime>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Displacement {
    pub dest: Vec2,
    pub speed: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Projectile {
    pub source: EntityId,
    pub homing: Option<EntityId>,
    pub dir: Vec2,
    /// Distance left before a non-homing projectile expires.
    pub remaining: f64,
    pub speed: f64,
    pub pierce_left: u32,
    pub filter: Filter,
    pub on_hit: EffectListId,
    pub already_hit: Vec<EntityId>,
    pub action: Option<ActionId>,
    pub rank: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Region {
    pub name: String,
    /// Units currently inside, in id order.
    pub inside: Vec<EntityId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub kind: EntityKind,
    pub team: TeamId,
    pub alive: bool,
    pub pos: Vec2,
    pub radius: f64,

    // Unit parts.
    pub unit: Option<UnitId>,
    pub level: u32,
    pub perks: Vec<PerkId>,
    /// Ranks for the unit's own slots, overriding the ruleset's default.
    pub ranks: Option<Vec<u8>>,
    pub stats: StatBlock,
    /// Current value of each registered resource.
    pub resources: Vec<f64>,
    pub buffs: Vec<BuffInstance>,
    pub slots: Vec<Slot>,
    pub action: Option<RunningAction>,
    /// One buffered command, run when the current action ends.
    pub queued: Option<Command>,
    pub move_to: Option<Vec2>,
    pub displacement: Option<Displacement>,
    /// Follows another entity's position (a carried object, a passenger).
    pub attached_to: Option<EntityId>,

    pub projectile: Option<Projectile>,
    pub region: Option<Region>,
}

impl Entity {
    pub fn new(id: EntityId, kind: EntityKind, team: TeamId, pos: Vec2, radius: f64) -> Entity {
        Entity {
            id,
            kind,
            team,
            alive: true,
            pos,
            radius,
            unit: None,
            level: 1,
            perks: Vec::new(),
            ranks: None,
            stats: StatBlock::default(),
            resources: Vec::new(),
            buffs: Vec::new(),
            slots: Vec::new(),
            action: None,
            queued: None,
            move_to: None,
            displacement: None,
            attached_to: None,
            projectile: None,
            region: None,
        }
    }

    pub fn is_unit(&self) -> bool {
        self.kind == EntityKind::Unit
    }

    pub fn buff(&self, def: BuffId) -> Option<&BuffInstance> {
        self.buffs.iter().find(|b| b.def == def)
    }

    pub fn slot_by_key(&self, key: &str) -> Option<usize> {
        self.slots.iter().position(|s| s.key == key)
    }
}

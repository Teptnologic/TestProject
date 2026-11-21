//! Immutable content definitions and the effect catalogue.
//!
//! A loader compiles data files into these types. The kernel only reads them.

use crate::ids::*;
use crate::registry::{Ex, Registry, VarRef, Who};
use crate::stats::Layer;
use arena_formula::{Const, Env, Expr, FormulaError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Which entities an effect or action may touch, relative to its source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Filter {
    Enemy,
    /// Same team, including the source itself.
    Ally,
    Any,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetKind {
    None,
    Unit(Filter),
    Point,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Center {
    Source,
    Target,
    Point,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DashTo {
    /// Up to the edge of the context's target.
    Target,
    /// Towards the context's point.
    Point,
    /// Directly away from the context's source (a knockback).
    AwayFromSource,
    /// Towards the context's source, stopping at its edge (a pull or hook).
    Source,
}

/// The building blocks data files compose. Rulesets add their own through `Custom`.
#[derive(Clone, Debug)]
pub enum Effect {
    ModifyResource {
        who: Who,
        resource: ResourceId,
        amount: Ex,
    },
    ApplyBuff {
        who: Who,
        buff: BuffId,
        duration: Option<Ex>,
        stacks: u32,
        value: Option<Ex>,
    },
    RemoveBuff {
        who: Who,
        buff: BuffId,
    },
    /// Launches a projectile from the source. Homing ones follow the context's
    /// target; others fly in a line towards the context's point.
    Projectile {
        speed: Ex,
        radius: f64,
        range: Option<Ex>,
        homing: bool,
        /// How many extra targets it passes through after the first.
        pierce: u32,
        filter: Filter,
        on_hit: EffectListId,
    },
    /// Runs `effects` once per matching unit in a circle, with that unit as the target.
    Area {
        center: Center,
        radius: Ex,
        filter: Filter,
        effects: EffectListId,
    },
    /// Moves `who`. A speed of zero is an instant blink.
    Dash {
        who: Who,
        to: DashTo,
        distance: Option<Ex>,
        speed: Ex,
    },
    If {
        cond: Ex,
        then: EffectListId,
        otherwise: Option<EffectListId>,
    },
    Emit {
        kind: EventKind,
        value: Option<Ex>,
    },
    /// Adds to the score of the source's team.
    AddScore {
        amount: Ex,
    },
    /// Makes one of the source's actions ready immediately.
    ResetCooldown {
        action: ActionId,
    },
    Custom {
        kind: CustomId,
        who: Who,
        args: Vec<Option<Ex>>,
        tags: Vec<TagId>,
    },
}

#[derive(Clone, Debug)]
pub struct SlotDef {
    pub key: String,
    pub action: ActionId,
}

#[derive(Clone, Debug)]
pub struct UnitDef {
    pub id: UnitId,
    pub name: String,
    pub radius: f64,
    pub stats: Vec<(StatId, f64)>,
    /// Per-level growth. How it applies is up to the ruleset.
    pub growth: Vec<(StatId, f64)>,
    /// Free-form numbers the ruleset interprets.
    pub params: BTreeMap<String, f64>,
    pub slots: Vec<SlotDef>,
    /// Buffs the unit always has.
    pub passives: Vec<BuffId>,
    pub tags: Vec<TagId>,
    /// Slot keys in the order ranks should be spent.
    pub rank_priority: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ActionDef {
    pub id: ActionId,
    pub name: String,
    pub tags: Vec<TagId>,
    pub max_rank: u8,
    pub target: TargetKind,
    /// Edge-to-edge for unit targets. `None` means unlimited.
    pub range: Option<Ex>,
    pub cast_time: Ex,
    pub cooldown: Ex,
    pub costs: Vec<(ResourceId, Ex)>,
    /// Extra condition for starting the action, e.g. "has four stacks".
    pub usable_when: Option<Ex>,
    /// The unit stands still while winding up.
    pub lock_move: bool,
    /// A move command during the wind-up cancels the action instead of being queued.
    pub cancel_on_move: bool,
    /// Runs when the action starts.
    pub on_cast: Option<EffectListId>,
    /// Runs when the wind-up completes.
    pub on_execute: Option<EffectListId>,
}

impl ActionDef {
    pub fn new(name: &str) -> ActionDef {
        ActionDef {
            id: ActionId(0),
            name: name.to_string(),
            tags: Vec::new(),
            max_rank: 1,
            target: TargetKind::None,
            range: None,
            cast_time: Expr::Num(0.0),
            cooldown: Expr::Num(0.0),
            costs: Vec::new(),
            usable_when: None,
            lock_move: false,
            cancel_on_move: false,
            on_cast: None,
            on_execute: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StackPolicy {
    /// Re-applying resets the duration.
    Refresh,
    /// Re-applying adds stacks (up to the maximum) and resets the duration.
    AddStack,
    /// Every application is its own instance.
    Independent,
    /// Re-applying replaces the instance only if the new value is at least as large.
    Stronger,
}

/// Whether the buff's owner must be the event's source or its target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Source,
    Target,
}

/// Runs effects when an event involves the buff's owner. In those effects the
/// owner is the source and the other party is the target.
#[derive(Clone, Debug)]
pub struct Listener {
    pub event: EventKind,
    pub role: Role,
    /// Internal cooldown in seconds.
    pub cooldown: Option<f64>,
    pub cond: Option<Ex>,
    pub effects: EffectListId,
    /// Remove the buff after the listener fires.
    pub consume: bool,
}

#[derive(Clone, Debug)]
pub struct BuffDef {
    pub id: BuffId,
    pub name: String,
    /// Seconds. `None` is permanent.
    pub duration: Option<Ex>,
    pub max_stacks: u32,
    pub policy: StackPolicy,
    /// Evaluated when applied and whenever stacks change; may read `stacks` and `value`.
    pub modifiers: Vec<(StatId, Layer, Ex)>,
    pub flags: Vec<FlagId>,
    pub tags: Vec<TagId>,
    pub on_apply: Option<EffectListId>,
    pub on_expire: Option<EffectListId>,
    /// Interval in seconds and the effects to run each time.
    pub periodic: Option<(Ex, EffectListId)>,
    pub listeners: Vec<Listener>,
}

impl BuffDef {
    pub fn new(name: &str) -> BuffDef {
        BuffDef {
            id: BuffId(0),
            name: name.to_string(),
            duration: None,
            max_stacks: 1,
            policy: StackPolicy::Refresh,
            modifiers: Vec::new(),
            flags: Vec::new(),
            tags: Vec::new(),
            on_apply: None,
            on_expire: None,
            periodic: None,
            listeners: Vec::new(),
        }
    }
}

/// Anything chosen at staging: an item, a rune, a summoner spell.
#[derive(Clone, Debug)]
pub struct PerkDef {
    pub id: PerkId,
    pub name: String,
    pub category: String,
    /// Perks sharing a group are mutually exclusive.
    pub group: Option<String>,
    pub cost: f64,
    pub modifiers: Vec<(StatId, Layer, f64)>,
    pub buffs: Vec<BuffId>,
    /// Actions the perk grants (item actives, summoner spells).
    pub slots: Vec<SlotDef>,
}

#[derive(Clone, Debug, Default)]
pub struct Defs {
    pub reg: Registry,
    pub units: Vec<UnitDef>,
    pub actions: Vec<ActionDef>,
    pub buffs: Vec<BuffDef>,
    pub perks: Vec<PerkDef>,
    pub effect_lists: Vec<Vec<Effect>>,
    unit_names: BTreeMap<String, UnitId>,
    action_names: BTreeMap<String, ActionId>,
    buff_names: BTreeMap<String, BuffId>,
    perk_names: BTreeMap<String, PerkId>,
}

impl Defs {
    pub fn add_unit(&mut self, mut def: UnitDef) -> UnitId {
        def.id = UnitId(self.units.len() as u16);
        self.unit_names.insert(def.name.clone(), def.id);
        self.units.push(def);
        UnitId(self.units.len() as u16 - 1)
    }

    pub fn add_action(&mut self, mut def: ActionDef) -> ActionId {
        def.id = ActionId(self.actions.len() as u16);
        self.action_names.insert(def.name.clone(), def.id);
        self.actions.push(def);
        ActionId(self.actions.len() as u16 - 1)
    }

    pub fn add_buff(&mut self, mut def: BuffDef) -> BuffId {
        def.id = BuffId(self.buffs.len() as u16);
        self.buff_names.insert(def.name.clone(), def.id);
        self.buffs.push(def);
        BuffId(self.buffs.len() as u16 - 1)
    }

    /// Overwrites a definition added earlier, keeping its id. Loaders register
    /// names first so that definitions can refer to each other.
    pub fn replace_action(&mut self, id: ActionId, mut def: ActionDef) {
        def.id = id;
        self.actions[id.idx()] = def;
    }

    pub fn replace_buff(&mut self, id: BuffId, mut def: BuffDef) {
        def.id = id;
        self.buffs[id.idx()] = def;
    }

    /// Compiles a formula against the registered vocabulary and known buffs.
    pub fn formula(
        &self,
        src: &str,
        constants: &BTreeMap<String, Const>,
    ) -> Result<Ex, FormulaError> {
        struct VarEnv<'a>(&'a Defs, &'a BTreeMap<String, Const>);
        impl Env<VarRef> for VarEnv<'_> {
            fn var(&mut self, name: &str) -> Result<VarRef, String> {
                self.0.reg.resolve_var(name, &|b| self.0.buff_id(b))
            }

            fn constant(&mut self, name: &str) -> Option<Const> {
                self.1.get(name).cloned()
            }
        }
        arena_formula::parse(src, &mut VarEnv(self, constants))
    }

    pub fn add_perk(&mut self, mut def: PerkDef) -> PerkId {
        def.id = PerkId(self.perks.len() as u16);
        self.perk_names.insert(def.name.clone(), def.id);
        self.perks.push(def);
        PerkId(self.perks.len() as u16 - 1)
    }

    pub fn add_effects(&mut self, list: Vec<Effect>) -> EffectListId {
        self.effect_lists.push(list);
        EffectListId(self.effect_lists.len() as u32 - 1)
    }

    pub fn unit_id(&self, name: &str) -> Option<UnitId> {
        self.unit_names.get(name).copied()
    }

    pub fn action_id(&self, name: &str) -> Option<ActionId> {
        self.action_names.get(name).copied()
    }

    pub fn buff_id(&self, name: &str) -> Option<BuffId> {
        self.buff_names.get(name).copied()
    }

    pub fn perk_id(&self, name: &str) -> Option<PerkId> {
        self.perk_names.get(name).copied()
    }

    pub fn unit(&self, id: UnitId) -> &UnitDef {
        &self.units[id.idx()]
    }

    pub fn action(&self, id: ActionId) -> &ActionDef {
        &self.actions[id.idx()]
    }

    pub fn buff(&self, id: BuffId) -> &BuffDef {
        &self.buffs[id.idx()]
    }

    pub fn perk(&self, id: PerkId) -> &PerkDef {
        &self.perks[id.idx()]
    }
}

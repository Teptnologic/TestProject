//! Game-agnostic simulation kernel.
//!
//! The kernel owns time, entities, 2D space, stats, buffs, actions, effects and
//! events. It knows nothing about any particular game: every game concept
//! (health, armor, stuns, items) is supplied from outside through three
//! contracts:
//!
//! * [`Ruleset`] — code that gives the kernel's generic machinery its meaning.
//! * [`Defs`] — immutable content (units, actions, buffs, perks) compiled from
//!   data files by a loader.
//! * [`Command`] / [`Event`] — what agents send in and what the world reports
//!   back out.

pub mod command;
pub mod defs;
pub mod entity;
pub mod event;
pub mod ids;
pub mod math;
pub mod registry;
pub mod rng;
pub mod ruleset;
pub mod stats;
pub mod time;
pub mod world;

pub use command::{Command, Illegal, Target};
pub use defs::{
    ActionDef, BuffDef, Center, DashTo, Defs, Effect, Filter, Listener, PerkDef, Role, SlotDef,
    StackPolicy, TargetKind, UnitDef,
};
pub use entity::{
    BuffInstance, Entity, EntityKind, Projectile, Region, RunningAction, Slot, Stage,
};
pub use event::Event;
pub use ids::*;
pub use math::Vec2;
pub use registry::{
    CustomSpec, Ex, Interner, Registry, ResPart, ResourceDef, StatPart, VarRef, Who,
};
pub use rng::Rng;
pub use ruleset::{ActionTiming, Ruleset};
pub use stats::{Layer, ModSrc, Modifier, StatBlock};
pub use time::{micros, SimTime, TickConfig};
pub use world::{BuffOpts, Ctx, SpawnUnit, World, WorldConfig, WorldState};

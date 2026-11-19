//! The contract between the kernel and a game's rules.
//!
//! The kernel calls these hooks whenever a decision depends on game meaning.
//! A ruleset is built by a factory `fn(&mut Defs) -> Arc<dyn Ruleset>` that
//! registers its stats, resources, flags, tags, events, custom effects and
//! built-in buffs before any content is loaded.

use crate::defs::{ActionDef, Defs, UnitDef};
use crate::event::Event;
use crate::ids::{CustomId, EntityId, PerkId, StatId, TagId};
use crate::world::{Ctx, World};

/// Durations in microseconds.
#[derive(Clone, Copy, Debug, Default)]
pub struct ActionTiming {
    /// Time from starting the action until its effects happen.
    pub windup: u64,
    /// Time after the effects during which the unit is still busy.
    pub recovery: u64,
    /// Time from starting the action until it can be started again.
    pub cooldown: u64,
}

pub trait Ruleset: Send + Sync {
    fn id(&self) -> &'static str;
    fn version(&self) -> &'static str;

    /// Base stat values for a unit at a level (before perks and buffs).
    fn base_stats(&self, defs: &Defs, unit: &UnitDef, level: u32) -> Vec<(StatId, f64)>;
    /// Rank of each of the unit's own slots at a level.
    fn default_ranks(&self, defs: &Defs, unit: &UnitDef, level: u32) -> Vec<u8>;
    /// Checks a staging choice. `budget` is in the ruleset's currency.
    fn validate_loadout(
        &self,
        defs: &Defs,
        unit: &UnitDef,
        perks: &[PerkId],
        budget: f64,
    ) -> Result<(), String>;

    /// Called after a unit is spawned or respawned with stats and perks in place.
    fn init_unit(&self, w: &mut World, e: EntityId);
    /// Health-like measure in `0..=1`, used for tiebreaks, observations and targeting.
    fn vitality(&self, w: &World, e: EntityId) -> f64;

    /// Movement speed in distance units per second.
    fn move_speed(&self, w: &World, e: EntityId) -> f64;
    fn can_move(&self, w: &World, e: EntityId) -> bool;
    fn can_use(&self, w: &World, e: EntityId, action: &ActionDef) -> bool;
    fn targetable(&self, w: &World, by: EntityId, target: EntityId) -> bool;
    fn action_timing(&self, w: &World, e: EntityId, action: &ActionDef, rank: u8) -> ActionTiming;

    /// Applies one of the ruleset's own effects (registered as a `CustomSpec`)
    /// to `target`. `args` are in the order the spec declared them.
    fn apply_custom(
        &self,
        w: &mut World,
        kind: CustomId,
        target: EntityId,
        args: &[Option<f64>],
        tags: &[TagId],
        ctx: &Ctx,
    );
    /// Sees every event before data-defined listeners do.
    fn on_event(&self, w: &mut World, ev: &Event);
}

//! Agents: things that decide what a unit does.
//!
//! A [`Policy`] sees the world read-only and returns one [`Command`] per
//! decision. It can never change the world directly, so a scripted bot, a
//! trained model and a human all go through the same door.
//!
//! [`obs`] turns the world into fixed-size numbers and back, for policies
//! that are neural networks.

pub mod obs;
mod scripted;

pub use scripted::Scripted;

use arena_kernel::{
    Command, Defs, EntityId, PerkId, Rng, Ruleset, Target, TargetKind, UnitId, World,
};

/// What a policy is offered at staging.
pub struct StagingView<'a> {
    pub defs: &'a Defs,
    pub ruleset: &'a dyn Ruleset,
    pub unit: UnitId,
    pub budget: f64,
}

pub trait Policy: Send {
    /// Chooses a loadout within the budget. Only called when the scenario leaves the choice open.
    fn stage(&mut self, _view: &StagingView) -> Vec<PerkId> {
        Vec::new()
    }

    /// Chooses what the unit does next.
    fn act(&mut self, w: &World, me: EntityId) -> Command;
}

/// Does nothing. For structures and target dummies.
pub struct Idle;

impl Policy for Idle {
    fn act(&mut self, _: &World, _: EntityId) -> Command {
        Command::None
    }
}

/// Picks uniformly among a handful of legal commands. Useful as a baseline and for fuzzing.
pub struct Random {
    rng: Rng,
}

impl Random {
    pub fn new(seed: u64) -> Random {
        Random {
            rng: Rng::new(seed),
        }
    }
}

impl Policy for Random {
    fn act(&mut self, w: &World, me: EntityId) -> Command {
        let Some(e) = w.ent(me).filter(|e| e.alive) else {
            return Command::None;
        };
        let others: Vec<EntityId> = w.units().filter(|u| u.id != me).map(|u| u.id).collect();
        let mut options = vec![Command::None];
        let step = |rng: &mut Rng| (rng.next_f64() - 0.5) * 800.0;
        let near =
            arena_kernel::Vec2::new(e.pos.x + step(&mut self.rng), e.pos.y + step(&mut self.rng));
        options.push(Command::MoveTo(near));
        for slot in 0..e.slots.len() {
            let other = others
                .get(self.rng.below(others.len() as u64) as usize)
                .copied();
            let target = match w.defs.action(e.slots[slot].action).target {
                TargetKind::None => Target::None,
                TargetKind::Point => Target::Point(near),
                TargetKind::Unit(_) => other.map_or(Target::None, Target::Entity),
            };
            options.push(Command::Use { slot, target });
        }
        options.retain(|c| w.legal(me, c).is_ok());
        options[self.rng.below(options.len() as u64) as usize]
    }
}

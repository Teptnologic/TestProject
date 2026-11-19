//! Commands: the only way agents influence the world.

use crate::ids::EntityId;
use crate::math::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Target {
    None,
    Entity(EntityId),
    Point(Vec2),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Command {
    /// Keep doing whatever is in progress.
    None,
    Stop,
    MoveTo(Vec2),
    /// Use the action in a slot (an attack, an ability, an item active).
    Use {
        slot: usize,
        target: Target,
    },
}

/// Why a command was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Illegal {
    NoSuchEntity,
    Dead,
    Displaced,
    CannotMove,
    NoSuchSlot,
    NotLearned,
    OnCooldown,
    CannotAfford,
    WrongTargetKind,
    InvalidTarget,
    OutOfRange,
    ConditionNotMet,
    Blocked,
}

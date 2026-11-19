//! Events: what the world reports. One uniform shape so that data-defined
//! listeners, rulesets, game modes, logs and stat collectors all read the same thing.

use crate::ids::{ActionId, EntityId, EventKind};
use crate::time::SimTime;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    pub kind: EventKind,
    pub time: SimTime,
    /// Who caused it (the attacker, the caster, the killer, the unit entering a region).
    pub source: Option<EntityId>,
    /// Who it happened to.
    pub target: Option<EntityId>,
    pub value: f64,
    pub action: Option<ActionId>,
    /// How many listeners deep this event was raised; guards against proc loops.
    pub depth: u8,
}

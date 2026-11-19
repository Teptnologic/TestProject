//! Content packs: data files compiled into kernel definitions.
//!
//! A pack is a directory:
//!
//! ```text
//! pack.yaml              id, and the ruleset + version it targets
//! generated/units.yaml   numbers per unit id   (written by tools, never by hand)
//! generated/perks.yaml   numbers per perk id
//! authored/units/*.yaml  one unit per file: behaviour, referring to numbers
//! authored/perks/*.yaml  a list of perks per file
//! ```
//!
//! Numbers and behaviour are merged by id, so a balance patch only touches
//! `generated/`. Everything is validated here: a pack that loads cannot fail
//! with a missing name or a bad formula in the middle of a match.

mod compile;
mod raw;

use arena_kernel::{Defs, Ex, PerkId, Ruleset, UnitId};
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

/// "Use this slot when the condition holds." The condition is evaluated with
/// the unit as source, its chosen enemy as target and the slot's rank.
#[derive(Clone, Debug)]
pub struct AiRule {
    pub slot: String,
    pub when: Option<Ex>,
}

/// Hints a pack gives to scripted agents for one unit.
#[derive(Clone, Debug, Default)]
pub struct AiScript {
    /// Perks to buy, in order, when the agent chooses its own loadout.
    pub build: Vec<PerkId>,
    /// Slot key of the unit's basic attack.
    pub attack_slot: Option<String>,
    /// Back away from the enemy between attacks.
    pub kite: bool,
    pub rules: Vec<AiRule>,
}

#[derive(Clone, Debug)]
pub struct Pack {
    pub id: String,
    pub ai: BTreeMap<UnitId, AiScript>,
}

#[derive(Clone, Debug)]
pub struct ContentError {
    pub file: PathBuf,
    /// Where in the file, e.g. `action garen_q > on_execute[0]`.
    pub at: String,
    pub message: String,
}

impl fmt::Display for ContentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.at.is_empty() {
            write!(f, "{}: {}", self.file.display(), self.message)
        } else {
            write!(f, "{}: {}: {}", self.file.display(), self.at, self.message)
        }
    }
}

impl std::error::Error for ContentError {}

/// Loads a pack into `defs`. The ruleset must already have registered its vocabulary.
pub fn load_pack(dir: &Path, defs: &mut Defs, ruleset: &dyn Ruleset) -> Result<Pack, ContentError> {
    compile::load(dir, defs, ruleset)
}

/// The id of the ruleset a pack targets, so a host can pick the right one before loading.
pub fn pack_ruleset(dir: &Path) -> Result<String, ContentError> {
    compile::pack_ruleset(dir)
}

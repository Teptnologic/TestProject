//! Matches: a scenario played from staging to a result.
//!
//! This crate wires the layers together and owns nothing game-specific:
//!
//! ```text
//! Scenario (data) ─┐
//! Content  (defs) ─┼─> Match ──> MatchResult / Replay / BatchResult
//! Policies, Mode  ─┘
//! ```
//!
//! [`Match::step`] drives built-in policies; [`Match::step_with`] lets an
//! outside driver (a training loop, a UI) supply the commands instead.

mod batch;
mod runner;
mod scenario;
mod stats;

pub use batch::{run_batch, BatchResult, TeamRecord};
pub use runner::{Match, Replay};
pub use scenario::{
    LoadoutConfig, MapConfig, MemberConfig, ObjectiveConfig, PolicyConfig, Scenario, TeamConfig,
};
pub use stats::{MatchResult, UnitReport};

use arena_content::Pack;
use arena_kernel::{Defs, Ruleset};
use std::path::Path;
use std::sync::Arc;

/// Builds a ruleset, registering its vocabulary in `defs`.
pub type RulesetFactory = fn(&mut Defs) -> Arc<dyn Ruleset>;

/// A loaded content pack together with its ruleset. Immutable and shared by every match.
pub struct Content {
    pub defs: Arc<Defs>,
    pub ruleset: Arc<dyn Ruleset>,
    pub pack: Pack,
}

impl Content {
    pub fn load(pack_dir: &Path, ruleset: RulesetFactory) -> Result<Content, String> {
        let mut defs = Defs::default();
        let ruleset = ruleset(&mut defs);
        let pack = arena_content::load_pack(pack_dir, &mut defs, ruleset.as_ref())
            .map_err(|e| e.to_string())?;
        Ok(Content {
            defs: Arc::new(defs),
            ruleset,
            pack,
        })
    }
}

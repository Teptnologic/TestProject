//! The scenario file: who fights whom, where, and what for.

use arena_modes::ModeConfig;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub name: String,
    /// Content pack directory, relative to the scenario file.
    pub pack: String,
    pub map: MapConfig,
    /// Length of a server tick in microseconds.
    #[serde(default = "default_tick_us")]
    pub tick_us: u64,
    /// Agents choose a command every this many ticks.
    #[serde(default = "default_decision_ticks")]
    pub decision_ticks: u64,
    pub timeout_s: f64,
    /// Each unit spawns up to this far from its spawn point, at random. This is
    /// what makes repeated matches between scripted bots differ.
    #[serde(default)]
    pub spawn_jitter: f64,
    /// Seconds before a dead unit returns to its spawn point. Omit for no respawns.
    pub respawn_s: Option<f64>,
    pub mode: ModeConfig,
    pub teams: Vec<TeamConfig>,
    #[serde(skip)]
    pub dir: PathBuf,
}

fn default_tick_us() -> u64 {
    33_333
}

fn default_decision_ticks() -> u64 {
    3
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapConfig {
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TeamConfig {
    pub name: String,
    pub members: Vec<MemberConfig>,
    /// Structures this team must protect (used by the nexus mode).
    #[serde(default)]
    pub objectives: Vec<ObjectiveConfig>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemberConfig {
    pub unit: String,
    pub level: u32,
    /// Budget for the staging phase.
    #[serde(default)]
    pub gold: f64,
    pub spawn: [f64; 2],
    #[serde(default)]
    pub loadout: LoadoutConfig,
    #[serde(default)]
    pub policy: PolicyConfig,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectiveConfig {
    pub unit: String,
    pub spawn: [f64; 2],
}

/// `loadout: { fixed: [long_sword, ...] }` or `loadout: policy`.
#[derive(Clone, Debug, Deserialize)]
#[serde(try_from = "RawLoadout")]
pub enum LoadoutConfig {
    /// The scenario dictates the perks.
    Fixed(Vec<String>),
    /// The unit's policy buys within the budget.
    Policy,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawLoadout {
    Word(String),
    Fixed { fixed: Vec<String> },
}

impl TryFrom<RawLoadout> for LoadoutConfig {
    type Error = String;

    fn try_from(raw: RawLoadout) -> Result<Self, String> {
        match raw {
            RawLoadout::Fixed { fixed } => Ok(LoadoutConfig::Fixed(fixed)),
            RawLoadout::Word(w) if w == "policy" => Ok(LoadoutConfig::Policy),
            RawLoadout::Word(w) => Err(format!(
                "unknown loadout `{w}`; use `policy` or `{{ fixed: [...] }}`"
            )),
        }
    }
}

impl Default for LoadoutConfig {
    fn default() -> Self {
        LoadoutConfig::Fixed(Vec::new())
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyConfig {
    /// The rule-following bot described by the pack's AI hints.
    #[default]
    Scripted,
    Idle,
    Random,
}

impl Scenario {
    pub fn load(path: &Path) -> Result<Scenario, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut s: Scenario =
            serde_yaml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        s.dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        if s.teams.len() < 2 {
            return Err(format!(
                "{}: a scenario needs at least two teams",
                path.display()
            ));
        }
        if s.tick_us == 0 || s.decision_ticks == 0 {
            return Err(format!(
                "{}: tick_us and decision_ticks must be positive",
                path.display()
            ));
        }
        Ok(s)
    }

    pub fn pack_dir(&self) -> PathBuf {
        self.dir.join(&self.pack)
    }
}

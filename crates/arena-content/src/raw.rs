//! The file formats, exactly as written on disk.

use serde::Deserialize;
use serde_yaml::Value;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawPack {
    pub id: String,
    pub ruleset: String,
    pub ruleset_version: String,
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
pub enum RawConst {
    Num(f64),
    List(Vec<f64>),
}

/// A formula: a bare number or a string.
#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum RawEx {
    Num(f64),
    Str(String),
}

/// One entry of `generated/units.yaml` or `generated/perks.yaml`.
#[derive(Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct RawNumbers {
    pub cost: Option<f64>,
    #[serde(default)]
    pub stats: BTreeMap<String, f64>,
    #[serde(default)]
    pub growth: BTreeMap<String, f64>,
    #[serde(default)]
    pub params: BTreeMap<String, f64>,
    #[serde(default)]
    pub constants: BTreeMap<String, RawConst>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawUnit {
    pub id: String,
    pub radius: f64,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub stats: BTreeMap<String, f64>,
    #[serde(default)]
    pub growth: BTreeMap<String, f64>,
    #[serde(default)]
    pub params: BTreeMap<String, f64>,
    #[serde(default)]
    pub constants: BTreeMap<String, RawConst>,
    #[serde(default)]
    pub rank_priority: Vec<String>,
    #[serde(default)]
    pub actions: Vec<RawAction>,
    #[serde(default)]
    pub buffs: Vec<RawBuff>,
    /// Buffs the unit always has.
    #[serde(default)]
    pub passives: Vec<String>,
    pub ai: Option<RawAi>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawAction {
    pub id: String,
    /// Slot key (`A`, `Q`, …). Actions without one are not placed on the unit.
    pub slot: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub max_rank: Option<u8>,
    /// `none`, `enemy`, `ally`, `any` or `point`.
    pub target: Option<String>,
    pub range: Option<RawEx>,
    pub cast_time: Option<RawEx>,
    pub cooldown: Option<RawEx>,
    #[serde(default)]
    pub costs: BTreeMap<String, RawEx>,
    /// Extra condition for starting the action.
    pub usable_when: Option<RawEx>,
    #[serde(default)]
    pub lock_move: bool,
    #[serde(default)]
    pub cancel_on_move: bool,
    #[serde(default)]
    pub on_cast: Vec<Value>,
    #[serde(default)]
    pub on_execute: Vec<Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawMod {
    pub stat: String,
    /// `flat` (default), `pct_add` or `pct_mult`.
    pub layer: Option<String>,
    pub value: RawEx,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawListener {
    /// Event name.
    pub on: String,
    /// `source` (default): the owner caused the event. `target`: it happened to the owner.
    pub role: Option<String>,
    pub cooldown: Option<f64>,
    pub when: Option<RawEx>,
    #[serde(default)]
    pub consume: bool,
    pub effects: Vec<Value>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawBuff {
    pub id: String,
    pub duration: Option<RawEx>,
    pub max_stacks: Option<u32>,
    /// `refresh` (default), `add_stack`, `independent` or `stronger`.
    pub policy: Option<String>,
    #[serde(default)]
    pub modifiers: Vec<RawMod>,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub on_apply: Vec<Value>,
    #[serde(default)]
    pub on_expire: Vec<Value>,
    /// Interval in seconds for `on_tick`.
    pub every: Option<RawEx>,
    #[serde(default)]
    pub on_tick: Vec<Value>,
    #[serde(default)]
    pub listeners: Vec<RawListener>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawPerk {
    pub id: String,
    /// `item` (default), `rune` or `spell`; the ruleset gives these meaning.
    pub category: Option<String>,
    pub group: Option<String>,
    pub cost: Option<f64>,
    /// Flat stat bonuses.
    #[serde(default)]
    pub stats: BTreeMap<String, f64>,
    #[serde(default)]
    pub modifiers: Vec<RawMod>,
    #[serde(default)]
    pub constants: BTreeMap<String, RawConst>,
    #[serde(default)]
    pub buffs: Vec<RawBuff>,
    /// Buffs the owner always has.
    #[serde(default)]
    pub passives: Vec<String>,
    #[serde(default)]
    pub actions: Vec<RawAction>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawRule {
    #[serde(rename = "use")]
    pub slot: String,
    pub when: Option<RawEx>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawAi {
    #[serde(default)]
    pub build: Vec<String>,
    pub attack: Option<String>,
    /// Back away from the enemy between attacks.
    #[serde(default)]
    pub kite: bool,
    #[serde(default)]
    pub rules: Vec<RawRule>,
}

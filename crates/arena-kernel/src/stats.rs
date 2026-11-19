//! Named stats with layered modifiers.
//!
//! `total = (base + Σ flat) × (1 + Σ pct_add) × Π (1 + pct_mult)`

use crate::ids::StatId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Layer {
    Flat,
    PctAdd,
    PctMult,
}

/// Who owns a modifier, so it can be removed again.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModSrc {
    Perk(u16),
    Buff(u64),
    Ruleset(u32),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Modifier {
    pub src: ModSrc,
    pub stat: StatId,
    pub layer: Layer,
    pub value: f64,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StatBlock {
    base: Vec<f64>,
    mods: Vec<Modifier>,
}

impl StatBlock {
    pub fn new(stat_count: usize) -> StatBlock {
        StatBlock {
            base: vec![0.0; stat_count],
            mods: Vec::new(),
        }
    }

    pub fn base(&self, stat: StatId) -> f64 {
        self.base.get(stat.idx()).copied().unwrap_or(0.0)
    }

    pub fn set_base(&mut self, stat: StatId, value: f64) {
        self.base[stat.idx()] = value;
    }

    pub fn total(&self, stat: StatId) -> f64 {
        let (mut flat, mut pct, mut mult) = (0.0, 0.0, 1.0);
        for m in self.mods.iter().filter(|m| m.stat == stat) {
            match m.layer {
                Layer::Flat => flat += m.value,
                Layer::PctAdd => pct += m.value,
                Layer::PctMult => mult *= 1.0 + m.value,
            }
        }
        (self.base(stat) + flat) * (1.0 + pct) * mult
    }

    pub fn bonus(&self, stat: StatId) -> f64 {
        self.total(stat) - self.base(stat)
    }

    pub fn add(&mut self, m: Modifier) {
        self.mods.push(m);
    }

    pub fn remove_src(&mut self, src: ModSrc) {
        self.mods.retain(|m| m.src != src);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layers_combine_in_order() {
        let s = StatId(0);
        let mut b = StatBlock::new(1);
        b.set_base(s, 100.0);
        let m = |layer, value| Modifier {
            src: ModSrc::Ruleset(0),
            stat: s,
            layer,
            value,
        };
        b.add(m(Layer::Flat, 50.0));
        b.add(m(Layer::PctAdd, 0.2));
        b.add(m(Layer::PctAdd, 0.1));
        b.add(m(Layer::PctMult, 0.5));
        assert!((b.total(s) - 150.0 * 1.3 * 1.5).abs() < 1e-9);
        assert!((b.bonus(s) - (150.0 * 1.3 * 1.5 - 100.0)).abs() < 1e-9);
        b.remove_src(ModSrc::Ruleset(0));
        assert_eq!(b.total(s), 100.0);
    }
}

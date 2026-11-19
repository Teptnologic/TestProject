//! The League of Legends ruleset.
//!
//! Gives the kernel's generic machinery its LoL meaning: stat growth, attack
//! timing, ability haste, the damage pipeline, crowd control, movement speed
//! and loadout rules. Content packs that target this ruleset use the names
//! registered in [`Lol::new`].

mod combat;
mod vocab;

pub use vocab::Vocab;

use arena_kernel::*;
use std::sync::Arc;

pub const ID: &str = "lol";
pub const VERSION: &str = "0.1";

/// Attack speed limits, in attacks per second.
const MIN_ATTACK_SPEED: f64 = 0.2;
const MAX_ATTACK_SPEED: f64 = 2.5;
/// Wind-up share of an attack when a unit does not specify `windup_pct`.
const DEFAULT_WINDUP_PCT: f64 = 0.25;
const DEFAULT_CRIT_DAMAGE: f64 = 1.75;
const MAX_ITEMS: usize = 6;
const MAX_SPELLS: usize = 2;
const MAX_RUNES: usize = 1;

pub struct Lol {
    pub v: Vocab,
}

/// Registers the ruleset's vocabulary in `defs` and returns the ruleset.
/// Call this before loading any content.
pub fn build(defs: &mut Defs) -> Arc<dyn Ruleset> {
    Arc::new(Lol::new(defs))
}

impl Lol {
    pub fn new(defs: &mut Defs) -> Lol {
        Lol {
            v: Vocab::register(defs),
        }
    }

    /// Attacks per second: `base + ratio × bonus`, clamped.
    pub fn attack_speed(&self, w: &World, e: EntityId) -> f64 {
        let Some(unit) = w.ent(e).and_then(|x| x.unit).map(|u| w.defs.unit(u)) else {
            return 0.0;
        };
        let base = unit.params.get("base_as").copied().unwrap_or(0.625);
        let ratio = unit.params.get("as_ratio").copied().unwrap_or(base);
        (base + ratio * w.stat(e, self.v.attack_speed)).clamp(MIN_ATTACK_SPEED, MAX_ATTACK_SPEED)
    }

    /// Riot's level curve: growth applies in slightly increasing steps.
    pub fn stat_at_level(base: f64, growth: f64, level: u32) -> f64 {
        let n = level.max(1) as f64 - 1.0;
        base + growth * n * (0.7025 + 0.0175 * n)
    }

    /// Movement speed after slows and the soft caps.
    pub fn capped_move_speed(raw: f64, slow: f64) -> f64 {
        let ms = raw * (1.0 - slow.clamp(0.0, 1.0));
        if ms > 490.0 {
            ms * 0.5 + 230.0
        } else if ms > 415.0 {
            ms * 0.8 + 83.0
        } else if ms < 220.0 {
            ms * 0.5 + 110.0
        } else {
            ms
        }
    }

    fn has_tag(def: &ActionDef, tag: TagId) -> bool {
        def.tags.contains(&tag)
    }
}

impl Ruleset for Lol {
    fn id(&self) -> &'static str {
        ID
    }

    fn version(&self) -> &'static str {
        VERSION
    }

    fn base_stats(&self, _: &Defs, unit: &UnitDef, level: u32) -> Vec<(StatId, f64)> {
        let growth = |stat| {
            unit.growth
                .iter()
                .find(|(s, _)| *s == stat)
                .map_or(0.0, |(_, g)| *g)
        };
        let mut out: Vec<(StatId, f64)> = unit
            .stats
            .iter()
            .map(|(s, base)| (*s, Self::stat_at_level(*base, growth(*s), level)))
            .collect();
        // Stats that only grow (attack speed) have no base entry.
        for (s, g) in &unit.growth {
            if !out.iter().any(|(o, _)| o == s) {
                out.push((*s, Self::stat_at_level(0.0, *g, level)));
            }
        }
        if !out.iter().any(|(s, _)| *s == self.v.crit_damage) {
            out.push((self.v.crit_damage, DEFAULT_CRIT_DAMAGE));
        }
        out
    }

    /// The ultimate gets a rank at levels 6, 11 and 16. Every other ability
    /// gets one point, then the rest go to abilities in the unit's priority
    /// order, each capped at half the level rounded up.
    fn default_ranks(&self, defs: &Defs, unit: &UnitDef, level: u32) -> Vec<u8> {
        let action = |i: usize| defs.action(unit.slots[i].action);
        let mut ranks = vec![0u8; unit.slots.len()];
        let mut points = level;
        let mut order: Vec<usize> = unit
            .rank_priority
            .iter()
            .filter_map(|k| unit.slots.iter().position(|s| &s.key == k))
            .collect();
        for (i, rank) in ranks.iter_mut().enumerate() {
            let def = action(i);
            if !Self::has_tag(def, self.v.ability) {
                *rank = 1;
            } else if Self::has_tag(def, self.v.ultimate) {
                let earned = [6, 11, 16].iter().filter(|l| level >= **l).count() as u8;
                *rank = earned.min(def.max_rank);
                points = points.saturating_sub(*rank as u32);
            } else if !order.contains(&i) {
                order.push(i);
            }
        }
        order.retain(|i| !Self::has_tag(action(*i), self.v.ultimate));
        let cap = level.div_ceil(2) as u8;
        for pass in 0..2 {
            for i in &order {
                let limit = if pass == 0 {
                    1
                } else {
                    cap.min(action(*i).max_rank)
                };
                while ranks[*i] < limit && points > 0 {
                    ranks[*i] += 1;
                    points -= 1;
                }
            }
        }
        ranks
    }

    fn validate_loadout(
        &self,
        defs: &Defs,
        unit: &UnitDef,
        perks: &[PerkId],
        budget: f64,
    ) -> Result<(), String> {
        let perks: Vec<&PerkDef> = perks.iter().map(|p| defs.perk(*p)).collect();
        let count = |cat: &str| perks.iter().filter(|p| p.category == cat).count();
        for (cat, max) in [
            ("item", MAX_ITEMS),
            ("spell", MAX_SPELLS),
            ("rune", MAX_RUNES),
        ] {
            if count(cat) > max {
                return Err(format!(
                    "{}: at most {max} perks of category `{cat}`",
                    unit.name
                ));
            }
        }
        for (i, p) in perks.iter().enumerate() {
            let clash = perks[..i].iter().find(|q| match (&p.group, &q.group) {
                (Some(a), Some(b)) => a == b,
                _ => p.category != "item" && p.id == q.id,
            });
            if let Some(q) = clash {
                return Err(format!(
                    "{}: `{}` cannot be combined with `{}`",
                    unit.name, p.name, q.name
                ));
            }
        }
        let cost: f64 = perks.iter().map(|p| p.cost).sum();
        if cost > budget {
            return Err(format!(
                "{}: loadout costs {cost} gold but the budget is {budget}",
                unit.name
            ));
        }
        Ok(())
    }

    fn init_unit(&self, w: &mut World, e: EntityId) {
        for res in [self.v.hp, self.v.mana] {
            let max = w.resource_max(e, res);
            w.set_resource(e, res, max);
        }
    }

    fn vitality(&self, w: &World, e: EntityId) -> f64 {
        let max = w.resource_max(e, self.v.hp);
        if max > 0.0 {
            w.resource(e, self.v.hp) / max
        } else {
            0.0
        }
    }

    fn move_speed(&self, w: &World, e: EntityId) -> f64 {
        let Some(ent) = w.ent(e) else { return 0.0 };
        let slow = ent
            .buffs
            .iter()
            .filter(|b| w.defs.buff(b.def).tags.contains(&self.v.slow))
            .map(|b| b.value)
            .fold(0.0, f64::max);
        let raw = w.stat(e, self.v.move_speed);
        if raw <= 0.0 {
            return 0.0;
        }
        Self::capped_move_speed(raw, slow)
    }

    fn can_move(&self, w: &World, e: EntityId) -> bool {
        !w.has_flag(e, self.v.stunned) && !w.has_flag(e, self.v.rooted)
    }

    fn can_use(&self, w: &World, e: EntityId, action: &ActionDef) -> bool {
        if w.has_flag(e, self.v.stunned) {
            return false;
        }
        if Self::has_tag(action, self.v.basic_attack) && w.has_flag(e, self.v.disarmed) {
            return false;
        }
        !(Self::has_tag(action, self.v.ability) && w.has_flag(e, self.v.silenced))
    }

    fn targetable(&self, w: &World, _: EntityId, target: EntityId) -> bool {
        w.is_alive(target) && !w.has_flag(target, self.v.untargetable)
    }

    fn action_timing(&self, w: &World, e: EntityId, action: &ActionDef, rank: u8) -> ActionTiming {
        if Self::has_tag(action, self.v.basic_attack) {
            let unit = w.ent(e).and_then(|x| x.unit).map(|u| w.defs.unit(u));
            let windup_pct = unit
                .and_then(|u| u.params.get("windup_pct").copied())
                .unwrap_or(DEFAULT_WINDUP_PCT);
            let period = 1.0 / self.attack_speed(w, e);
            return ActionTiming {
                windup: micros(period * windup_pct),
                recovery: 0,
                cooldown: micros(period),
            };
        }
        let mut ctx = Ctx::new(e);
        ctx.rank = rank;
        let mut cooldown = w.eval(&action.cooldown, &ctx);
        if Self::has_tag(action, self.v.ability) {
            cooldown *= 100.0 / (100.0 + w.stat(e, self.v.ability_haste).max(0.0));
        }
        ActionTiming {
            windup: micros(w.eval(&action.cast_time, &ctx)),
            recovery: 0,
            cooldown: micros(cooldown),
        }
    }

    fn apply_custom(
        &self,
        w: &mut World,
        kind: CustomId,
        target: EntityId,
        args: &[Option<f64>],
        tags: &[TagId],
        ctx: &Ctx,
    ) {
        let arg = |i: usize| args.get(i).copied().flatten().unwrap_or(0.0);
        let c = &self.v;
        if kind == c.fx_damage {
            self.damage(w, target, arg(0), arg(1) > 0.0, tags, ctx);
        } else if kind == c.fx_heal {
            self.heal(w, target, arg(0), ctx);
        } else if kind == c.fx_shield {
            self.shield(w, target, arg(0), arg(1), ctx);
        } else if kind == c.fx_stun {
            self.crowd_control(w, target, c.b_stun, arg(0), 0.0, ctx);
        } else if kind == c.fx_root {
            self.crowd_control(w, target, c.b_root, arg(0), 0.0, ctx);
        } else if kind == c.fx_silence {
            self.crowd_control(w, target, c.b_silence, arg(0), 0.0, ctx);
        } else if kind == c.fx_slow {
            self.crowd_control(w, target, c.b_slow, arg(1), arg(0), ctx);
        } else if kind == c.fx_grievous {
            let opts = BuffOpts {
                duration: Some(arg(0)),
                depth: ctx.depth,
                ..BuffOpts::default()
            };
            w.apply_buff(target, c.b_grievous, ctx.source, opts);
        }
    }

    fn on_event(&self, w: &mut World, ev: &Event) {
        // The kernel reports every executed action; data listens for abilities specifically.
        if ev.kind == w.defs.reg.ev.action_executed {
            let is_ability = ev
                .action
                .is_some_and(|a| Self::has_tag(w.defs.action(a), self.v.ability));
            if is_ability {
                let mut cast = ev.clone();
                cast.kind = self.v.ev_ability_cast;
                w.emit(cast);
            }
        }
    }
}

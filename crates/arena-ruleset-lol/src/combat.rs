//! The damage pipeline, healing, shields and crowd control.

use crate::Lol;
use arena_kernel::*;

/// Healing lost while under Grievous Wounds.
const GRIEVOUS_REDUCTION: f64 = 0.4;
const MAX_DAMAGE_REDUCTION: f64 = 0.9;

impl Lol {
    /// Damage multiplier for a resistance value (which may be negative).
    pub fn resist_multiplier(resist: f64) -> f64 {
        if resist >= 0.0 {
            100.0 / (100.0 + resist)
        } else {
            2.0 - 100.0 / (100.0 - resist)
        }
    }

    /// Resistance after penetration: percentage first, then flat (never below zero).
    pub fn effective_resist(resist: f64, pct_pen: f64, flat_pen: f64) -> f64 {
        if resist <= 0.0 {
            return resist;
        }
        (resist * (1.0 - pct_pen.clamp(0.0, 1.0)) - flat_pen.max(0.0)).max(0.0)
    }

    /// Stages: source amplification → crit → resistances → target reduction →
    /// shields → health → `damage_dealt` → on-hit → vamp → death.
    pub(crate) fn damage(
        &self,
        w: &mut World,
        target: EntityId,
        raw: f64,
        can_crit: bool,
        tags: &[TagId],
        ctx: &Ctx,
    ) {
        let v = &self.v;
        let src = ctx.source;
        if raw <= 0.0 || !w.is_alive(target) || w.has_flag(target, v.invulnerable) {
            return;
        }
        let mut dmg = raw * (1.0 + w.stat(src, v.damage_amp));
        let crit_chance = w.stat(src, v.crit_chance).min(1.0);
        if can_crit && w.rng.chance(crit_chance) {
            dmg *= w.stat(src, v.crit_damage);
        }
        if tags.contains(&v.physical) {
            let r = w.stat(target, v.armor);
            let r = Self::effective_resist(r, w.stat(src, v.armor_pen), w.stat(src, v.lethality));
            dmg *= Self::resist_multiplier(r);
        } else if tags.contains(&v.magic) {
            let r = w.stat(target, v.mr);
            let r =
                Self::effective_resist(r, w.stat(src, v.magic_pen_pct), w.stat(src, v.magic_pen));
            dmg *= Self::resist_multiplier(r);
        }
        dmg *= 1.0
            - w.stat(target, v.damage_reduction)
                .clamp(0.0, MAX_DAMAGE_REDUCTION);

        let to_health = self.absorb_with_shields(w, target, dmg);
        let hp = w.resource(target, v.hp) - to_health;
        w.set_resource(target, v.hp, hp);

        let is_attack = tags.contains(&v.basic_attack);
        let event = |w: &World, kind, value| {
            let mut ev = w.event(kind, Some(src), Some(target));
            ev.value = value;
            ev.action = ctx.action;
            ev.depth = ctx.depth;
            ev
        };
        let ev = event(w, v.ev_damage, dmg);
        w.emit(ev);
        if is_attack && !tags.contains(&v.proc_) {
            let ev = event(w, v.ev_on_hit, dmg);
            w.emit(ev);
        }
        let vamp = w.stat(src, v.omnivamp)
            + if is_attack {
                w.stat(src, v.life_steal)
            } else {
                0.0
            };
        if vamp > 0.0 && w.is_alive(src) {
            self.heal(
                w,
                src,
                dmg * vamp,
                &Ctx {
                    target: Some(src),
                    ..*ctx
                },
            );
        }
        if w.is_alive(target) && w.resource(target, v.hp) <= 0.0 {
            w.kill(target, Some(src));
        }
    }

    /// Spends shields that expire soonest first. Returns the damage left over.
    fn absorb_with_shields(&self, w: &mut World, target: EntityId, mut dmg: f64) -> f64 {
        let Some(e) = w.ent(target) else { return dmg };
        let mut shields: Vec<(SimTime, u64)> = e
            .buffs
            .iter()
            .filter(|b| w.defs.buff(b.def).tags.contains(&self.v.shield))
            .map(|b| (b.expires_at.unwrap_or(SimTime(u64::MAX)), b.uid))
            .collect();
        shields.sort();
        for (_, uid) in shields {
            if dmg <= 0.0 {
                break;
            }
            let Some(b) = w
                .ent_mut(target)
                .and_then(|e| e.buffs.iter_mut().find(|b| b.uid == uid))
            else {
                continue;
            };
            let used = b.value.min(dmg);
            b.value -= used;
            dmg -= used;
            if b.value <= 0.0 {
                w.remove_buff_uid(target, uid, false);
            }
        }
        dmg
    }

    pub(crate) fn heal(&self, w: &mut World, target: EntityId, amount: f64, ctx: &Ctx) {
        let v = &self.v;
        if amount <= 0.0 || !w.is_alive(target) {
            return;
        }
        let cut = if w.has_flag(target, v.grievous) {
            GRIEVOUS_REDUCTION
        } else {
            0.0
        };
        let before = w.resource(target, v.hp);
        w.set_resource(target, v.hp, before + amount * (1.0 - cut));
        let healed = w.resource(target, v.hp) - before;
        if healed > 0.0 {
            let mut ev = w.event(v.ev_healed, Some(ctx.source), Some(target));
            ev.value = healed;
            ev.depth = ctx.depth;
            w.emit(ev);
        }
    }

    pub(crate) fn shield(
        &self,
        w: &mut World,
        target: EntityId,
        amount: f64,
        secs: f64,
        ctx: &Ctx,
    ) {
        if amount <= 0.0 {
            return;
        }
        let value = amount * (1.0 + w.stat(ctx.source, self.v.heal_shield_power));
        let opts = BuffOpts {
            duration: Some(secs),
            value,
            depth: ctx.depth,
            ..BuffOpts::default()
        };
        w.apply_buff(target, self.v.b_shield, ctx.source, opts);
    }

    /// Applies a crowd-control buff, shortened by the target's tenacity.
    pub(crate) fn crowd_control(
        &self,
        w: &mut World,
        target: EntityId,
        buff: BuffId,
        secs: f64,
        value: f64,
        ctx: &Ctx,
    ) {
        let v = &self.v;
        if secs <= 0.0 || !w.is_alive(target) {
            return;
        }
        let secs = secs * (1.0 - w.stat(target, v.tenacity).clamp(0.0, 1.0));
        let opts = BuffOpts {
            duration: Some(secs),
            value,
            depth: ctx.depth,
            ..BuffOpts::default()
        };
        w.apply_buff(target, buff, ctx.source, opts);
        if buff == v.b_stun {
            w.interrupt(target);
        } else if buff == v.b_root {
            if let Some(e) = w.ent_mut(target) {
                e.move_to = None;
            }
        }
        let mut ev = w.event(v.ev_cc, Some(ctx.source), Some(target));
        ev.value = secs;
        ev.depth = ctx.depth;
        w.emit(ev);
    }
}

//! Runs the effect building blocks.

use super::{BuffOpts, Ctx, World};
use crate::defs::{Center, DashTo, Effect};
use crate::entity::{Displacement, Entity, EntityKind, Projectile};
use crate::ids::*;
use crate::math::Vec2;
use crate::registry::Who;

impl World {
    pub fn run_effects(&mut self, list: EffectListId, ctx: &Ctx) {
        let defs = self.defs.clone();
        for effect in &defs.effect_lists[list.idx()] {
            self.run_effect(effect, ctx);
        }
    }

    fn who(ctx: &Ctx, who: Who) -> Option<EntityId> {
        match who {
            Who::Source => Some(ctx.source),
            Who::Target => ctx.target,
        }
    }

    fn run_effect(&mut self, effect: &Effect, ctx: &Ctx) {
        match effect {
            Effect::ModifyResource {
                who,
                resource,
                amount,
            } => {
                let Some(id) = Self::who(ctx, *who) else {
                    return;
                };
                let value = self.resource(id, *resource) + self.eval(amount, ctx);
                self.set_resource(id, *resource, value);
            }
            Effect::ApplyBuff {
                who,
                buff,
                duration,
                stacks,
                value,
            } => {
                let Some(id) = Self::who(ctx, *who) else {
                    return;
                };
                let opts = BuffOpts {
                    duration: duration.as_ref().map(|d| self.eval(d, ctx)),
                    stacks: *stacks,
                    value: value.as_ref().map_or(0.0, |v| self.eval(v, ctx)),
                    rank: ctx.rank,
                    depth: ctx.depth,
                };
                self.apply_buff(id, *buff, ctx.source, opts);
            }
            Effect::RemoveBuff { who, buff } => {
                if let Some(id) = Self::who(ctx, *who) {
                    self.remove_buff(id, *buff);
                }
            }
            Effect::Projectile {
                speed,
                radius,
                range,
                homing,
                pierce,
                filter,
                on_hit,
            } => {
                let Some(src) = self.ent(ctx.source) else {
                    return;
                };
                let (from, team) = (src.pos, src.team);
                let aim = ctx
                    .point
                    .or(ctx.target.and_then(|t| self.ent(t)).map(|e| e.pos));
                let Some(aim) = aim else { return };
                if *homing && ctx.target.is_none() {
                    return;
                }
                let id = self.alloc();
                let mut e = Entity::new(id, EntityKind::Projectile, team, from, *radius);
                e.projectile = Some(Projectile {
                    source: ctx.source,
                    homing: if *homing { ctx.target } else { None },
                    dir: aim.sub(from).normalized_or(Vec2::new(1.0, 0.0)),
                    remaining: range.as_ref().map_or(f64::MAX, |r| self.eval(r, ctx)),
                    speed: self.eval(speed, ctx),
                    pierce_left: *pierce,
                    filter: *filter,
                    on_hit: *on_hit,
                    already_hit: Vec::new(),
                    action: ctx.action,
                    rank: ctx.rank,
                });
                self.entities.insert(id, e);
            }
            Effect::Area {
                center,
                radius,
                filter,
                effects,
            } => {
                let at = match center {
                    Center::Source => self.ent(ctx.source).map(|e| e.pos),
                    Center::Target => ctx.target.and_then(|t| self.ent(t)).map(|e| e.pos),
                    Center::Point => ctx.point,
                };
                let Some(at) = at else { return };
                let r = self.eval(radius, ctx);
                for id in self.units_in_circle(at, r) {
                    let valid = self.passes(*filter, ctx.source, id)
                        && (id == ctx.source || self.ruleset.targetable(self, ctx.source, id));
                    if valid {
                        self.run_effects(*effects, &ctx.with_target(id));
                    }
                }
            }
            Effect::Dash {
                who,
                to,
                distance,
                speed,
            } => {
                let Some(id) = Self::who(ctx, *who) else {
                    return;
                };
                let Some(me) = self.ent(id) else { return };
                let (pos, my_radius) = (me.pos, me.radius);
                let max = distance.as_ref().map(|d| self.eval(d, ctx));
                let dest = match to {
                    DashTo::Target => {
                        let Some(t) = ctx.target.and_then(|t| self.ent(t)) else {
                            return;
                        };
                        let gap = (pos.dist(t.pos) - my_radius - t.radius).max(0.0);
                        pos.step_towards(t.pos, max.map_or(gap, |m| m.min(gap))).0
                    }
                    DashTo::Point => {
                        let Some(p) = ctx.point else { return };
                        pos.step_towards(p, max.unwrap_or(f64::MAX)).0
                    }
                    DashTo::AwayFromSource => {
                        let Some(src) = self.ent(ctx.source) else {
                            return;
                        };
                        let dir = pos.sub(src.pos).normalized_or(Vec2::new(1.0, 0.0));
                        pos.add(dir.scale(max.unwrap_or(0.0)))
                    }
                };
                let dest = dest.clamp(self.size);
                let speed = self.eval(speed, ctx);
                if id != ctx.source {
                    self.interrupt(id);
                }
                let Some(e) = self.ent_mut(id) else { return };
                e.move_to = None;
                if speed <= 0.0 {
                    e.pos = dest;
                } else {
                    e.displacement = Some(Displacement { dest, speed });
                }
            }
            Effect::If {
                cond,
                then,
                otherwise,
            } => {
                if self.eval(cond, ctx) != 0.0 {
                    self.run_effects(*then, ctx);
                } else if let Some(list) = otherwise {
                    self.run_effects(*list, ctx);
                }
            }
            Effect::Emit { kind, value } => {
                let mut ev = self.event(*kind, Some(ctx.source), ctx.target);
                ev.value = value.as_ref().map_or(0.0, |v| self.eval(v, ctx));
                ev.action = ctx.action;
                ev.depth = ctx.depth + 1;
                self.emit(ev);
            }
            Effect::AddScore { amount } => {
                let Some(team) = self.ent(ctx.source).map(|e| e.team) else {
                    return;
                };
                let amount = self.eval(amount, ctx);
                self.add_score(team, amount);
            }
            Effect::ResetCooldown { action } => {
                let now = self.now;
                let Some(e) = self.ent_mut(ctx.source) else {
                    return;
                };
                for s in e.slots.iter_mut().filter(|s| s.action == *action) {
                    s.ready_at = now;
                }
            }
            Effect::Custom {
                kind,
                who,
                args,
                tags,
            } => {
                let Some(id) = Self::who(ctx, *who) else {
                    return;
                };
                let args: Vec<Option<f64>> = args
                    .iter()
                    .map(|a| a.as_ref().map(|ex| self.eval(ex, ctx)))
                    .collect();
                let ruleset = self.ruleset.clone();
                ruleset.apply_custom(self, *kind, id, &args, tags, ctx);
            }
        }
    }
}

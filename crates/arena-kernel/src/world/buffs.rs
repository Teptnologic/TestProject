//! Applying, stacking, ticking and expiring buffs, and running their listeners.

use super::{Ctx, Timer, World};
use crate::defs::{Role, StackPolicy};
use crate::entity::BuffInstance;
use crate::event::Event;
use crate::ids::*;
use crate::stats::{ModSrc, Modifier};
use crate::time::{micros, SimTime};

#[derive(Clone, Copy, Debug)]
pub struct BuffOpts {
    /// Seconds; overrides the definition's duration.
    pub duration: Option<f64>,
    pub stacks: u32,
    pub value: f64,
    pub rank: u8,
    pub depth: u8,
}

impl Default for BuffOpts {
    fn default() -> Self {
        BuffOpts {
            duration: None,
            stacks: 1,
            value: 0.0,
            rank: 0,
            depth: 0,
        }
    }
}

impl World {
    /// Applies a buff, following its stacking policy. Returns the instance's uid.
    pub fn apply_buff(
        &mut self,
        target: EntityId,
        buff: BuffId,
        source: EntityId,
        opts: BuffOpts,
    ) -> Option<u64> {
        let defs = self.defs.clone();
        let def = defs.buff(buff);
        if !self.is_alive(target) {
            return None;
        }
        let mut ctx = Ctx::new(source).with_target(target);
        ctx.rank = opts.rank;
        ctx.value = opts.value;
        ctx.depth = opts.depth;
        let secs = opts
            .duration
            .or_else(|| def.duration.as_ref().map(|d| self.eval(d, &ctx)));
        let expires_at = secs.map(|s| self.now.plus(micros(s)));
        let now = self.now;

        let existing = match def.policy {
            StackPolicy::Independent => None,
            _ => self.ent(target)?.buffs.iter().position(|b| b.def == buff),
        };
        let (uid, fresh) = match existing {
            Some(i) => {
                let b = &mut self.ent_mut(target)?.buffs[i];
                if def.policy == StackPolicy::Stronger && opts.value < b.value {
                    return Some(b.uid);
                }
                if def.policy == StackPolicy::AddStack {
                    b.stacks = (b.stacks + opts.stacks).min(def.max_stacks);
                }
                if def.policy == StackPolicy::Stronger {
                    b.value = opts.value;
                }
                b.expires_at = expires_at;
                b.source = source;
                b.rank = opts.rank;
                (b.uid, false)
            }
            None => {
                let uid = self.next_uid();
                self.ent_mut(target)?.buffs.push(BuffInstance {
                    uid,
                    def: buff,
                    source,
                    applied_at: now,
                    expires_at,
                    stacks: opts.stacks.clamp(1, def.max_stacks),
                    value: opts.value,
                    rank: opts.rank,
                    listener_ready: vec![SimTime::ZERO; def.listeners.len()],
                });
                (uid, true)
            }
        };
        self.refresh_buff_modifiers(target, uid);
        if let Some(at) = expires_at {
            self.schedule(
                at,
                Timer::BuffExpire {
                    entity: target,
                    uid,
                },
            );
        }
        if fresh {
            if let Some((interval, _)) = &def.periodic {
                let at = now.plus(micros(self.eval(interval, &ctx)));
                self.schedule(
                    at,
                    Timer::BuffTick {
                        entity: target,
                        uid,
                    },
                );
            }
            if let Some(list) = def.on_apply {
                let ctx = self.buff_ctx(target, uid, opts.depth).unwrap_or(ctx);
                self.run_effects(list, &ctx);
            }
        }
        let mut ev = self.event(defs.reg.ev.buff_applied, Some(source), Some(target));
        ev.value = buff.0 as f64;
        ev.depth = opts.depth + 1;
        self.emit(ev);
        Some(uid)
    }

    /// Removes every instance of a buff without running its expiry effects.
    pub fn remove_buff(&mut self, target: EntityId, buff: BuffId) {
        let Some(e) = self.ent(target) else { return };
        let uids: Vec<u64> = e
            .buffs
            .iter()
            .filter(|b| b.def == buff)
            .map(|b| b.uid)
            .collect();
        for uid in uids {
            self.remove_buff_uid(target, uid, false);
        }
    }

    pub fn remove_buff_uid(&mut self, target: EntityId, uid: u64, run_expire: bool) {
        let ctx = self.buff_ctx(target, uid, 0);
        let Some(e) = self.ent_mut(target) else {
            return;
        };
        let Some(i) = e.buffs.iter().position(|b| b.uid == uid) else {
            return;
        };
        let inst = e.buffs.remove(i);
        e.stats.remove_src(ModSrc::Buff(uid));
        let defs = self.defs.clone();
        if let (true, Some(list), Some(ctx)) = (run_expire, defs.buff(inst.def).on_expire, ctx) {
            self.run_effects(list, &ctx);
        }
        let mut ev = self.event(defs.reg.ev.buff_expired, Some(inst.source), Some(target));
        ev.value = inst.def.0 as f64;
        ev.depth = 1;
        self.emit(ev);
    }

    /// The context a buff's own effects run in: its applier is the source, its owner the target.
    fn buff_ctx(&self, owner: EntityId, uid: u64, depth: u8) -> Option<Ctx> {
        let b = self.ent(owner)?.buffs.iter().find(|b| b.uid == uid)?;
        let mut ctx = Ctx::new(b.source).with_target(owner);
        ctx.rank = b.rank;
        ctx.stacks = b.stacks;
        ctx.value = b.value;
        ctx.depth = depth;
        Some(ctx)
    }

    fn refresh_buff_modifiers(&mut self, owner: EntityId, uid: u64) {
        let defs = self.defs.clone();
        let Some(ctx) = self.buff_ctx(owner, uid, 0) else {
            return;
        };
        let Some(def) = self
            .ent(owner)
            .and_then(|e| e.buffs.iter().find(|b| b.uid == uid))
            .map(|b| b.def)
        else {
            return;
        };
        let src = ModSrc::Buff(uid);
        let mods: Vec<Modifier> = defs
            .buff(def)
            .modifiers
            .iter()
            .map(|(stat, layer, ex)| Modifier {
                src,
                stat: *stat,
                layer: *layer,
                value: self.eval(ex, &ctx),
            })
            .collect();
        if let Some(e) = self.ent_mut(owner) {
            e.stats.remove_src(src);
            mods.into_iter().for_each(|m| e.stats.add(m));
        }
    }

    pub(super) fn buff_timer_expired(&mut self, owner: EntityId, uid: u64) {
        let now = self.now;
        let due = self
            .ent(owner)
            .and_then(|e| e.buffs.iter().find(|b| b.uid == uid))
            .is_some_and(|b| b.expires_at.is_some_and(|t| t <= now));
        // A refreshed buff has a later expiry and its own timer; this one is stale.
        if due {
            self.remove_buff_uid(owner, uid, true);
        }
    }

    pub(super) fn buff_tick(&mut self, owner: EntityId, uid: u64) {
        let defs = self.defs.clone();
        let Some(ctx) = self.buff_ctx(owner, uid, 1) else {
            return;
        };
        let Some(def) = self
            .ent(owner)
            .and_then(|e| e.buffs.iter().find(|b| b.uid == uid))
            .map(|b| b.def)
        else {
            return;
        };
        let Some((interval, list)) = &defs.buff(def).periodic else {
            return;
        };
        self.run_effects(*list, &ctx);
        if self.buff_ctx(owner, uid, 0).is_some() {
            let at = self.now.plus(micros(self.eval(interval, &ctx)).max(1));
            self.schedule(at, Timer::BuffTick { entity: owner, uid });
        }
    }

    /// Runs the listeners of buffs owned by the event's source and target.
    pub(super) fn run_listeners(&mut self, ev: &Event) {
        let defs = self.defs.clone();
        let now = self.now;
        let mut due: Vec<(EntityId, u64, usize)> = Vec::new();
        let parties = [
            (Role::Source, ev.source, ev.target),
            (Role::Target, ev.target, ev.source),
        ];
        for (role, owner, _) in parties {
            let Some(e) = owner.and_then(|o| self.ent(o)) else {
                continue;
            };
            if !e.alive {
                continue;
            }
            for b in &e.buffs {
                for (i, l) in defs.buff(b.def).listeners.iter().enumerate() {
                    if l.event == ev.kind && l.role == role && b.listener_ready[i] <= now {
                        due.push((e.id, b.uid, i));
                    }
                }
            }
        }
        for (owner, uid, i) in due {
            // Earlier listeners may have removed this buff.
            let Some(b) = self
                .ent(owner)
                .and_then(|e| e.buffs.iter().find(|b| b.uid == uid))
            else {
                continue;
            };
            let listener = &defs.buff(b.def).listeners[i];
            let mut ctx = Ctx::new(owner);
            ctx.target = if ev.source == Some(owner) {
                ev.target
            } else {
                ev.source
            };
            ctx.rank = b.rank;
            ctx.stacks = b.stacks;
            ctx.value = ev.value;
            ctx.action = ev.action;
            ctx.depth = ev.depth + 1;
            if listener
                .cond
                .as_ref()
                .is_some_and(|c| self.eval(c, &ctx) == 0.0)
            {
                continue;
            }
            if let Some(cd) = listener.cooldown {
                let ready = now.plus(micros(cd));
                if let Some(b) = self
                    .ent_mut(owner)
                    .and_then(|e| e.buffs.iter_mut().find(|b| b.uid == uid))
                {
                    b.listener_ready[i] = ready;
                }
            }
            if listener.consume {
                self.remove_buff_uid(owner, uid, false);
            }
            self.run_effects(listener.effects, &ctx);
        }
    }
}

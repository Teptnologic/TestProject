//! Commands, legality and the action state machine.
//!
//! An action runs as: start (pay costs, start cooldown, `on_cast`) → wind-up →
//! execute (`on_execute`) → recovery → done. One command can be buffered
//! while a wind-up is in progress.

use super::{Ctx, Timer, World};
use crate::command::{Command, Illegal, Target};
use crate::defs::TargetKind;
use crate::entity::{RunningAction, Stage};
use crate::ids::*;

impl World {
    /// Whether a command would be accepted right now. Agents use this as their action mask.
    pub fn legal(&self, id: EntityId, cmd: &Command) -> Result<(), Illegal> {
        let e = self.ent(id).ok_or(Illegal::NoSuchEntity)?;
        if !e.alive {
            return Err(Illegal::Dead);
        }
        match *cmd {
            Command::None | Command::Stop => Ok(()),
            Command::MoveTo(_) => {
                if e.displacement.is_some() {
                    Err(Illegal::Displaced)
                } else if !self.ruleset.can_move(self, id) {
                    Err(Illegal::CannotMove)
                } else {
                    Ok(())
                }
            }
            Command::Use { slot, target } => {
                let s = e.slots.get(slot).ok_or(Illegal::NoSuchSlot)?;
                let def = self.defs.action(s.action);
                if s.rank == 0 {
                    return Err(Illegal::NotLearned);
                }
                if e.displacement.is_some() {
                    return Err(Illegal::Displaced);
                }
                if s.ready_at > self.now {
                    return Err(Illegal::OnCooldown);
                }
                let mut ctx = Ctx::new(id);
                ctx.rank = s.rank;
                for (res, cost) in &def.costs {
                    if self.resource(id, *res) < self.eval(cost, &ctx) {
                        return Err(Illegal::CannotAfford);
                    }
                }
                let range = def.range.as_ref().map(|r| self.eval(r, &ctx));
                match (def.target, target) {
                    (TargetKind::None, _) | (TargetKind::Point, Target::Point(_)) => {}
                    (TargetKind::Unit(filter), Target::Entity(t)) => {
                        let ok = self.ent(t).is_some_and(|x| x.is_unit() && x.alive)
                            && self.passes(filter, id, t)
                            && (t == id || self.ruleset.targetable(self, id, t));
                        if !ok {
                            return Err(Illegal::InvalidTarget);
                        }
                        if range.is_some_and(|r| self.edge_distance(id, t) > r) {
                            return Err(Illegal::OutOfRange);
                        }
                    }
                    _ => return Err(Illegal::WrongTargetKind),
                }
                if let Some(cond) = &def.usable_when {
                    if let Target::Entity(t) = target {
                        ctx.target = Some(t);
                    }
                    if self.eval(cond, &ctx) == 0.0 {
                        return Err(Illegal::ConditionNotMet);
                    }
                }
                if !self.ruleset.can_use(self, id, def) {
                    return Err(Illegal::Blocked);
                }
                Ok(())
            }
        }
    }

    /// Issues a command. Commands given during a wind-up are buffered.
    pub fn submit(&mut self, id: EntityId, cmd: Command) -> Result<(), Illegal> {
        self.legal(id, &cmd)?;
        let defs = self.defs.clone();
        let size = self.size;
        let running = self.ent(id).and_then(|e| e.action.clone());
        let winding_up = running.as_ref().is_some_and(|r| r.stage == Stage::Windup);
        let running_def = running
            .as_ref()
            .map(|r| defs.action(self.ent(id).expect("checked").slots[r.slot].action));
        match cmd {
            Command::None => {}
            Command::Stop => {
                if let Some(e) = self.ent_mut(id) {
                    e.move_to = None;
                }
            }
            Command::MoveTo(p) => {
                if let Some(def) = running_def {
                    if winding_up && def.lock_move && !def.cancel_on_move {
                        self.ent_mut(id).expect("checked").queued = Some(cmd);
                        return Ok(());
                    }
                    if winding_up && def.lock_move {
                        self.cancel_action(id);
                    } else if !winding_up {
                        // Moving during recovery ends it early.
                        self.ent_mut(id).expect("checked").action = None;
                    }
                }
                self.ent_mut(id).expect("checked").move_to = Some(p.clamp(size));
            }
            Command::Use { slot, target } => {
                if winding_up {
                    self.ent_mut(id).expect("checked").queued = Some(cmd);
                    return Ok(());
                }
                self.start_action(id, slot, target);
            }
        }
        Ok(())
    }

    /// Whether the unit is committed to a wind-up that a move command would interrupt.
    pub fn is_winding_up(&self, id: EntityId) -> bool {
        self.ent(id)
            .and_then(|e| e.action.as_ref())
            .is_some_and(|r| r.stage == Stage::Windup)
    }

    /// Stops whatever the unit is doing (used by rulesets for crowd control).
    /// A cancelled wind-up refunds the cooldown but not the costs.
    pub fn interrupt(&mut self, id: EntityId) {
        if self.is_winding_up(id) {
            self.cancel_action(id);
        }
        if let Some(e) = self.ent_mut(id) {
            e.action = None;
            e.queued = None;
            e.move_to = None;
        }
    }

    fn cancel_action(&mut self, id: EntityId) {
        let now = self.now;
        let Some(e) = self.ent_mut(id) else { return };
        if let Some(r) = e.action.take() {
            e.slots[r.slot].ready_at = now;
        }
    }

    fn action_ctx(&self, id: EntityId, slot: usize, target: Target) -> Ctx {
        let e = self.ent(id).expect("action owner exists");
        let s = &e.slots[slot];
        let mut ctx = Ctx::new(id);
        ctx.rank = s.rank;
        ctx.action = Some(s.action);
        match target {
            Target::Entity(t) => {
                ctx.target = Some(t);
                ctx.point = self.ent(t).map(|x| x.pos);
            }
            Target::Point(p) => ctx.point = Some(p),
            Target::None => {}
        }
        ctx
    }

    fn start_action(&mut self, id: EntityId, slot: usize, target: Target) {
        let defs = self.defs.clone();
        let ctx = self.action_ctx(id, slot, target);
        let def = defs.action(ctx.action.expect("set by action_ctx"));
        let timing = self.ruleset.action_timing(self, id, def, ctx.rank);
        for (res, cost) in &def.costs {
            let left = self.resource(id, *res) - self.eval(cost, &ctx);
            self.set_resource(id, *res, left);
        }
        let now = self.now;
        let token = self.next_uid();
        {
            let e = self.ent_mut(id).expect("action owner exists");
            e.slots[slot].ready_at = now.plus(timing.cooldown);
            if def.lock_move {
                e.move_to = None;
            }
            let ends_at = now.plus(timing.windup);
            e.action = Some(RunningAction {
                slot,
                target,
                stage: Stage::Windup,
                ends_at,
                token,
            });
        }
        let mut ev = self.event(defs.reg.ev.action_started, Some(id), ctx.target);
        ev.action = ctx.action;
        self.emit(ev);
        if let Some(list) = def.on_cast {
            self.run_effects(list, &ctx);
        }
        if timing.windup == 0 {
            self.action_stage(id, token);
        } else {
            self.schedule(
                now.plus(timing.windup),
                Timer::ActionStage { entity: id, token },
            );
        }
    }

    /// Advances a running action when its stage timer fires.
    pub(super) fn action_stage(&mut self, id: EntityId, token: u64) {
        let defs = self.defs.clone();
        let Some(run) = self.ent(id).and_then(|e| e.action.clone()) else {
            return;
        };
        if run.token != token || !self.is_alive(id) {
            return;
        }
        if run.stage == Stage::Recovery {
            return self.finish_action(id);
        }
        let ctx = self.action_ctx(id, run.slot, run.target);
        let def = defs.action(ctx.action.expect("set by action_ctx"));
        // The target may have died or become untargetable during the wind-up.
        let fizzled = match run.target {
            Target::Entity(t) => {
                !self.is_alive(t) || (t != id && !self.ruleset.targetable(self, id, t))
            }
            _ => false,
        };
        if !fizzled {
            if let Some(list) = def.on_execute {
                self.run_effects(list, &ctx);
            }
            let mut ev = self.event(defs.reg.ev.action_executed, Some(id), ctx.target);
            ev.action = ctx.action;
            self.emit(ev);
        }
        // The action's own effects may have killed, displaced or interrupted the unit.
        let still_running = self
            .ent(id)
            .and_then(|e| e.action.as_ref())
            .is_some_and(|r| r.token == token);
        if !still_running {
            return;
        }
        let recovery = self.ruleset.action_timing(self, id, def, ctx.rank).recovery;
        if recovery == 0 {
            return self.finish_action(id);
        }
        let ends_at = self.now.plus(recovery);
        if let Some(r) = self.ent_mut(id).and_then(|e| e.action.as_mut()) {
            r.stage = Stage::Recovery;
            r.ends_at = ends_at;
        }
        self.schedule(ends_at, Timer::ActionStage { entity: id, token });
    }

    fn finish_action(&mut self, id: EntityId) {
        let Some(e) = self.ent_mut(id) else { return };
        e.action = None;
        if let Some(cmd) = e.queued.take() {
            // A buffered command that is no longer legal is simply dropped.
            let _ = self.submit(id, cmd);
        }
    }
}

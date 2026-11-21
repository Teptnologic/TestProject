//! A rule-following bot driven by the hints a content pack gives for each unit.

use crate::{Policy, StagingView};
use arena_content::AiScript;
use arena_kernel::{Command, Ctx, EntityId, Illegal, PerkId, Target, TargetKind, Vec2, World};

/// How far a kiting unit backs off between attacks.
const KITE_STEP: f64 = 150.0;

/// Each decision: pick the nearest enemy, use the first rule whose condition
/// holds and whose action is legal, otherwise attack, otherwise close in.
pub struct Scripted {
    script: AiScript,
}

impl Scripted {
    pub fn new(script: AiScript) -> Scripted {
        Scripted { script }
    }

    fn nearest_enemy(w: &World, me: EntityId) -> Option<EntityId> {
        let pos = w.ent(me)?.pos;
        w.units()
            .filter(|u| w.hostile(me, u.id) && w.ruleset.targetable(w, me, u.id))
            .min_by(|a, b| {
                a.pos
                    .dist(pos)
                    .total_cmp(&b.pos.dist(pos))
                    .then(a.id.cmp(&b.id))
            })
            .map(|u| u.id)
    }

    /// The ally an ally-targeted action goes to: the most injured unit on the
    /// team (itself included) within the action's range, preferring a
    /// teammate over itself when they are equally hurt. Structures are skipped.
    fn ally(w: &World, me: EntityId, slot: usize) -> EntityId {
        let Some(e) = w.ent(me) else { return me };
        let mut ctx = Ctx::new(me);
        ctx.rank = e.slots[slot].rank;
        let range = w
            .defs
            .action(e.slots[slot].action)
            .range
            .as_ref()
            .map(|r| w.eval(r, &ctx));
        w.units()
            .filter(|u| u.alive && !u.slots.is_empty() && !w.hostile(me, u.id))
            .filter(|u| u.id == me || range.is_none_or(|r| w.edge_distance(me, u.id) <= r))
            .min_by(|a, b| {
                let va = w.ruleset.vitality(w, a.id);
                let vb = w.ruleset.vitality(w, b.id);
                va.total_cmp(&vb)
                    .then((a.id == me).cmp(&(b.id == me)))
                    .then(a.id.cmp(&b.id))
            })
            .map_or(me, |u| u.id)
    }

    /// Aims a slot's action the obvious way for its targeting kind.
    fn aim(w: &World, me: EntityId, slot: usize, enemy: EntityId) -> Command {
        let action = w
            .ent(me)
            .map(|e| e.slots[slot].action)
            .expect("caller checked the slot");
        let target = match w.defs.action(action).target {
            TargetKind::None => Target::None,
            TargetKind::Point => w.ent(enemy).map_or(Target::None, |e| Target::Point(e.pos)),
            TargetKind::Unit(arena_kernel::Filter::Ally) => Target::Entity(Self::ally(w, me, slot)),
            TargetKind::Unit(_) => Target::Entity(enemy),
        };
        Command::Use { slot, target }
    }
}

impl Policy for Scripted {
    /// Buys the pack's suggested build in order, skipping whatever does not fit.
    fn stage(&mut self, view: &StagingView) -> Vec<PerkId> {
        let unit = view.defs.unit(view.unit);
        let mut chosen: Vec<PerkId> = Vec::new();
        for perk in &self.script.build {
            chosen.push(*perk);
            if view
                .ruleset
                .validate_loadout(view.defs, unit, &chosen, view.budget)
                .is_err()
            {
                chosen.pop();
            }
        }
        chosen
    }

    fn act(&mut self, w: &World, me: EntityId) -> Command {
        let Some(e) = w.ent(me).filter(|e| e.alive) else {
            return Command::None;
        };
        // Moving now would throw away an attack or cast that is about to land.
        if w.is_winding_up(me) {
            return Command::None;
        }
        let Some(enemy) = Self::nearest_enemy(w, me) else {
            return Command::Stop;
        };
        let enemy_pos = w.ent(enemy).map_or(e.pos, |x| x.pos);

        for rule in &self.script.rules {
            let Some(slot) = e.slot_by_key(&rule.slot) else {
                continue;
            };
            let mut ctx = Ctx::new(me).with_target(enemy);
            ctx.rank = e.slots[slot].rank;
            if rule.when.as_ref().is_some_and(|c| w.eval(c, &ctx) == 0.0) {
                continue;
            }
            let cmd = Self::aim(w, me, slot, enemy);
            if w.legal(me, &cmd).is_ok() {
                return cmd;
            }
        }

        let attack_slot = self
            .script
            .attack_slot
            .as_deref()
            .and_then(|k| e.slot_by_key(k));
        let Some(slot) = attack_slot.or(if e.slots.is_empty() { None } else { Some(0) }) else {
            return Command::None;
        };
        let attack = Self::aim(w, me, slot, enemy);
        match w.legal(me, &attack) {
            Ok(()) => attack,
            Err(Illegal::OnCooldown) => {
                let mut ctx = Ctx::new(me);
                ctx.rank = e.slots[slot].rank;
                let range = w
                    .defs
                    .action(e.slots[slot].action)
                    .range
                    .as_ref()
                    .map(|r| w.eval(r, &ctx));
                let in_range = range.is_none_or(|r| w.edge_distance(me, enemy) <= r);
                if !in_range {
                    Command::MoveTo(enemy_pos)
                } else if self.script.kite {
                    let away = e.pos.sub(enemy_pos).normalized_or(Vec2::new(1.0, 0.0));
                    Command::MoveTo(e.pos.add(away.scale(KITE_STEP)))
                } else {
                    // Hold position between attacks.
                    Command::None
                }
            }
            Err(_) => Command::MoveTo(enemy_pos),
        }
    }
}

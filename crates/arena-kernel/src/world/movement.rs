//! Per-tick movement: displacements, walking, attachments, projectiles and regions.

use super::{Ctx, World};
use crate::entity::EntityKind;
use crate::ids::*;
use crate::math::segment_distance;

impl World {
    pub(super) fn advance_movement(&mut self, dt: f64) {
        let ids: Vec<EntityId> = self.entities.keys().copied().collect();
        for id in &ids {
            let Some(e) = self.ent(*id) else { continue };
            if e.is_unit() && e.alive {
                self.move_unit(*id, dt);
            }
        }
        for id in &ids {
            if self
                .ent(*id)
                .is_some_and(|e| e.kind == EntityKind::Projectile)
            {
                self.move_projectile(*id, dt);
            }
        }
        for id in &ids {
            if self.ent(*id).is_some_and(|e| e.kind == EntityKind::Region) {
                self.update_region(*id);
            }
        }
    }

    fn move_unit(&mut self, id: EntityId, dt: f64) {
        let defs = self.defs.clone();
        let size = self.size;
        let e = self.ent(id).expect("caller checked");
        if let Some(carrier) = e.attached_to {
            match self.ent(carrier).filter(|c| c.alive).map(|c| c.pos) {
                Some(pos) => self.ent_mut(id).expect("caller checked").pos = pos,
                None => self.ent_mut(id).expect("caller checked").attached_to = None,
            }
            return;
        }
        if let Some(d) = e.displacement.clone() {
            let e = self.ent_mut(id).expect("caller checked");
            let (pos, arrived) = e.pos.step_towards(d.dest, d.speed * dt);
            e.pos = pos;
            if arrived {
                e.displacement = None;
            }
            return;
        }
        let Some(dest) = e.move_to else { return };
        let rooted_by_action = e.action.as_ref().is_some_and(|r| {
            r.stage == crate::entity::Stage::Windup && defs.action(e.slots[r.slot].action).lock_move
        });
        if rooted_by_action || !self.ruleset.can_move(self, id) {
            return;
        }
        let speed = self.ruleset.move_speed(self, id);
        let e = self.ent_mut(id).expect("caller checked");
        let (pos, arrived) = e.pos.step_towards(dest, speed * dt);
        e.pos = pos.clamp(size);
        if arrived {
            e.move_to = None;
        }
    }

    fn move_projectile(&mut self, id: EntityId, dt: f64) {
        let size = self.size;
        let Some(e) = self.ent(id) else { return };
        let (from, radius) = (e.pos, e.radius);
        let mut p = e.projectile.clone().expect("projectile entity");
        let step = p.speed * dt;
        let mut hits: Vec<EntityId> = Vec::new();
        let mut done = false;
        let to;

        if let Some(target) = p.homing {
            let Some(t) = self.ent(target).filter(|t| t.alive) else {
                return self.despawn(id);
            };
            let reach = (from.dist(t.pos) - t.radius - radius).max(0.0);
            to = from.step_towards(t.pos, step).0;
            if reach <= step {
                hits.push(target);
                done = true;
            }
        } else {
            let travel = step.min(p.remaining);
            to = from.add(p.dir.scale(travel));
            p.remaining -= travel;
            // Sweep the whole step so fast projectiles cannot pass through a target.
            let mut touched: Vec<(f64, EntityId)> = self
                .units()
                .filter(|u| u.id != p.source && !p.already_hit.contains(&u.id))
                .filter(|u| self.passes(p.filter, p.source, u.id))
                .filter(|u| self.ruleset.targetable(self, p.source, u.id))
                .filter_map(|u| {
                    let (dist, along) = segment_distance(from, to, u.pos);
                    (dist <= radius + u.radius).then_some((along, u.id))
                })
                .collect();
            touched.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            for (_, unit) in touched {
                hits.push(unit);
                if p.pierce_left == 0 {
                    done = true;
                    break;
                }
                p.pierce_left -= 1;
            }
            let out_of_bounds = to.x < 0.0 || to.y < 0.0 || to.x > size.x || to.y > size.y;
            done = done || p.remaining <= 0.0 || out_of_bounds;
        }

        p.already_hit.extend(&hits);
        let (source, on_hit, action, rank) = (p.source, p.on_hit, p.action, p.rank);
        if done {
            self.despawn(id);
        } else if let Some(e) = self.ent_mut(id) {
            e.pos = to;
            e.projectile = Some(p);
        }
        for target in hits {
            let mut ctx = Ctx::new(source).with_target(target);
            ctx.action = action;
            ctx.rank = rank;
            ctx.point = Some(to);
            let mut ev = self.event(self.defs.reg.ev.projectile_hit, Some(source), Some(target));
            ev.action = action;
            self.emit(ev);
            self.run_effects(on_hit, &ctx);
        }
    }

    fn update_region(&mut self, id: EntityId) {
        let Some(e) = self.ent(id) else { return };
        let now_inside = self.units_in_circle(e.pos, e.radius);
        let before = e
            .region
            .as_ref()
            .map(|r| r.inside.clone())
            .unwrap_or_default();
        if now_inside == before {
            return;
        }
        let ev = self.defs.reg.ev;
        for unit in now_inside.iter().filter(|u| !before.contains(u)) {
            let event = self.event(ev.region_entered, Some(*unit), Some(id));
            self.emit(event);
        }
        for unit in before.iter().filter(|u| !now_inside.contains(u)) {
            let event = self.event(ev.region_exited, Some(*unit), Some(id));
            self.emit(event);
        }
        if let Some(r) = self.ent_mut(id).and_then(|e| e.region.as_mut()) {
            r.inside = now_inside;
        }
    }
}

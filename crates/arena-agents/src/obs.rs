//! Fixed-size observations and actions for learned policies.
//!
//! The layout depends only on the ruleset's registered resources and the
//! [`ObsSpec`], never on which units are in the match, so one network can
//! play any content.
//!
//! Features, in order:
//! * self: alive, x, y, vitality, level, winding up, then each resource as a fraction
//! * per slot (`max_slots`): present, ready, seconds until ready (capped at 1 = 30 s)
//! * per other unit (`max_others`, nearest first): present, hostile, dx, dy, vitality, distance

use arena_kernel::{Command, EntityId, Target, TargetKind, Vec2, World};

const SELF_FEATURES: usize = 6;
const SLOT_FEATURES: usize = 3;
const OTHER_FEATURES: usize = 6;
const COOLDOWN_SCALE_S: f64 = 30.0;
const LEVEL_SCALE: f64 = 18.0;
/// How far a `Move` action walks, in distance units.
const MOVE_STEP: f64 = 300.0;

#[derive(Clone, Copy, Debug)]
pub struct ObsSpec {
    pub max_slots: usize,
    pub max_others: usize,
}

impl Default for ObsSpec {
    fn default() -> Self {
        ObsSpec {
            max_slots: 8,
            max_others: 9,
        }
    }
}

impl ObsSpec {
    pub fn feature_count(&self, w: &World) -> usize {
        SELF_FEATURES
            + w.defs.reg.resources.len()
            + self.max_slots * SLOT_FEATURES
            + self.max_others * OTHER_FEATURES
    }
}

#[derive(Clone, Debug)]
pub struct Observation {
    pub features: Vec<f32>,
    /// Per slot: whether using it on at least one target is legal right now.
    pub slot_mask: Vec<bool>,
    pub can_move: bool,
    /// The other units, in the order they appear in the features.
    pub others: Vec<EntityId>,
}

/// What a learned policy outputs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AgentAction {
    Noop,
    Stop,
    /// Walk in a direction (each component in `-1..=1`).
    Move {
        dx: f64,
        dy: f64,
    },
    /// Use a slot on the `target`-th other unit, or towards a direction for point-targeted actions.
    Use {
        slot: usize,
        target: usize,
        dx: f64,
        dy: f64,
    },
}

pub fn observe(w: &World, me: EntityId, spec: &ObsSpec) -> Observation {
    let mut f: Vec<f32> = Vec::with_capacity(spec.feature_count(w));
    let mut push = |v: f64| f.push(v as f32);
    let Some(e) = w.ent(me) else {
        return Observation {
            features: vec![0.0; spec.feature_count(w)],
            slot_mask: vec![false; spec.max_slots],
            can_move: false,
            others: Vec::new(),
        };
    };
    let size = w.size;
    push(e.alive as u8 as f64);
    push(e.pos.x / size.x);
    push(e.pos.y / size.y);
    push(w.ruleset.vitality(w, me));
    push(e.level as f64 / LEVEL_SCALE);
    push(w.is_winding_up(me) as u8 as f64);
    for i in 0..w.defs.reg.resources.len() {
        let res = arena_kernel::ResourceId(i as u16);
        let max = w.resource_max(me, res);
        push(if max > 0.0 {
            w.resource(me, res) / max
        } else {
            0.0
        });
    }

    let mut others: Vec<&arena_kernel::Entity> = w.units().filter(|u| u.id != me).collect();
    others.sort_by(|a, b| {
        a.pos
            .dist(e.pos)
            .total_cmp(&b.pos.dist(e.pos))
            .then(a.id.cmp(&b.id))
    });
    others.truncate(spec.max_others);
    let other_ids: Vec<EntityId> = others.iter().map(|u| u.id).collect();

    let mut slot_mask = vec![false; spec.max_slots];
    for (i, usable) in slot_mask.iter_mut().enumerate() {
        let Some(slot) = e.slots.get(i) else {
            (0..SLOT_FEATURES).for_each(|_| push(0.0));
            continue;
        };
        let wait = slot.ready_at.0.saturating_sub(w.now.0) as f64 / 1e6;
        push(1.0);
        push((wait == 0.0) as u8 as f64);
        push((wait / COOLDOWN_SCALE_S).min(1.0));
        *usable = match w.defs.action(slot.action).target {
            TargetKind::None => w
                .legal(
                    me,
                    &Command::Use {
                        slot: i,
                        target: Target::None,
                    },
                )
                .is_ok(),
            TargetKind::Point => w
                .legal(
                    me,
                    &Command::Use {
                        slot: i,
                        target: Target::Point(e.pos),
                    },
                )
                .is_ok(),
            TargetKind::Unit(_) => other_ids.iter().chain([&me]).any(|t| {
                w.legal(
                    me,
                    &Command::Use {
                        slot: i,
                        target: Target::Entity(*t),
                    },
                )
                .is_ok()
            }),
        };
    }
    for i in 0..spec.max_others {
        let Some(o) = others.get(i) else {
            (0..OTHER_FEATURES).for_each(|_| push(0.0));
            continue;
        };
        push(1.0);
        push(w.hostile(me, o.id) as u8 as f64);
        push((o.pos.x - e.pos.x) / size.x);
        push((o.pos.y - e.pos.y) / size.y);
        push(w.ruleset.vitality(w, o.id));
        push(o.pos.dist(e.pos) / size.x.max(size.y));
    }
    let can_move = e.alive && w.legal(me, &Command::MoveTo(e.pos)).is_ok();
    Observation {
        features: f,
        slot_mask,
        can_move,
        others: other_ids,
    }
}

/// Turns a policy's output into a command for the unit the observation was made for.
pub fn decode(w: &World, me: EntityId, obs: &Observation, action: AgentAction) -> Command {
    let Some(e) = w.ent(me) else {
        return Command::None;
    };
    let offset = |dx: f64, dy: f64| {
        e.pos
            .add(Vec2::new(dx.clamp(-1.0, 1.0), dy.clamp(-1.0, 1.0)).scale(MOVE_STEP))
    };
    match action {
        AgentAction::Noop => Command::None,
        AgentAction::Stop => Command::Stop,
        AgentAction::Move { dx, dy } => Command::MoveTo(offset(dx, dy)),
        AgentAction::Use {
            slot,
            target,
            dx,
            dy,
        } => {
            let Some(s) = e.slots.get(slot) else {
                return Command::None;
            };
            let target = match w.defs.action(s.action).target {
                TargetKind::None => Target::None,
                TargetKind::Point => Target::Point(offset(dx, dy)),
                TargetKind::Unit(_) => obs
                    .others
                    .get(target)
                    .map_or(Target::Entity(me), |t| Target::Entity(*t)),
            };
            Command::Use { slot, target }
        }
    }
}

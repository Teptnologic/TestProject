//! Brings dead team members back at their spawn point after a delay.

use crate::{GameMode, MatchInfo};
use arena_kernel::{micros, EntityId, Event, SimTime, Vec2, World};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Respawner {
    delay_us: u64,
    homes: BTreeMap<EntityId, Vec2>,
    pending: Vec<(SimTime, EntityId)>,
}

impl Respawner {
    pub fn new(delay_s: f64, info: &MatchInfo) -> Respawner {
        let homes = info
            .teams
            .iter()
            .flat_map(|t| t.members.iter().copied())
            .collect();
        Respawner {
            delay_us: micros(delay_s),
            homes,
            pending: Vec::new(),
        }
    }

    pub fn on_event(&mut self, w: &World, ev: &Event, mode: &dyn GameMode) {
        if ev.kind != w.defs.reg.ev.died {
            return;
        }
        let Some(unit) = ev.target.filter(|u| self.homes.contains_key(u)) else {
            return;
        };
        if mode.may_respawn(unit) {
            self.pending.push((ev.time.plus(self.delay_us), unit));
        }
    }

    pub fn on_tick(&mut self, w: &mut World) {
        let now = w.now;
        let (due, later): (Vec<_>, Vec<_>) = self.pending.iter().partition(|(at, _)| *at <= now);
        self.pending = later;
        for (_, unit) in due {
            w.respawn(unit, self.homes[&unit]);
        }
    }
}

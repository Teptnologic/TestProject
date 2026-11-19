//! The vocabulary a ruleset registers: names for stats, resources, flags, tags,
//! events and custom effects. Content refers to these by name; the kernel uses ids.

use crate::ids::*;
use arena_formula::Expr;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct Interner {
    names: Vec<String>,
    index: BTreeMap<String, u16>,
}

impl Interner {
    pub fn intern(&mut self, name: &str) -> u16 {
        if let Some(id) = self.index.get(name) {
            return *id;
        }
        let id = self.names.len() as u16;
        self.names.push(name.to_string());
        self.index.insert(name.to_string(), id);
        id
    }

    pub fn get(&self, name: &str) -> Option<u16> {
        self.index.get(name).copied()
    }

    pub fn name(&self, id: u16) -> &str {
        &self.names[id as usize]
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }
}

/// A pool such as health or mana: a current value capped by a stat.
#[derive(Clone, Debug)]
pub struct ResourceDef {
    pub name: String,
    pub max_stat: StatId,
    /// Stat that regenerates this resource, and how much of it applies per second.
    pub regen_stat: Option<StatId>,
    pub regen_per_sec: f64,
}

/// An effect implemented by the ruleset, usable from data by name.
#[derive(Clone, Debug)]
pub struct CustomSpec {
    pub name: String,
    /// Names of the formula arguments, in the order `apply_custom` receives them.
    pub args: Vec<String>,
}

/// Events the kernel itself raises.
#[derive(Clone, Copy, Debug)]
pub struct KernelEvents {
    pub spawned: EventKind,
    pub died: EventKind,
    pub action_started: EventKind,
    pub action_executed: EventKind,
    pub buff_applied: EventKind,
    pub buff_expired: EventKind,
    pub region_entered: EventKind,
    pub region_exited: EventKind,
    pub projectile_hit: EventKind,
}

#[derive(Clone, Debug)]
pub struct Registry {
    pub stats: Interner,
    pub flags: Interner,
    pub tags: Interner,
    pub events: Interner,
    pub resources: Vec<ResourceDef>,
    pub customs: Vec<CustomSpec>,
    pub ev: KernelEvents,
}

impl Default for Registry {
    fn default() -> Self {
        let mut events = Interner::default();
        let mut e = |n: &str| EventKind(events.intern(n));
        let ev = KernelEvents {
            spawned: e("spawned"),
            died: e("died"),
            action_started: e("action_started"),
            action_executed: e("action_executed"),
            buff_applied: e("buff_applied"),
            buff_expired: e("buff_expired"),
            region_entered: e("region_entered"),
            region_exited: e("region_exited"),
            projectile_hit: e("projectile_hit"),
        };
        Registry {
            stats: Interner::default(),
            flags: Interner::default(),
            tags: Interner::default(),
            events,
            resources: Vec::new(),
            customs: Vec::new(),
            ev,
        }
    }
}

impl Registry {
    pub fn stat(&mut self, name: &str) -> StatId {
        StatId(self.stats.intern(name))
    }

    pub fn flag(&mut self, name: &str) -> FlagId {
        FlagId(self.flags.intern(name))
    }

    pub fn tag(&mut self, name: &str) -> TagId {
        TagId(self.tags.intern(name))
    }

    pub fn event(&mut self, name: &str) -> EventKind {
        EventKind(self.events.intern(name))
    }

    pub fn resource(&mut self, def: ResourceDef) -> ResourceId {
        self.resources.push(def);
        ResourceId(self.resources.len() as u16 - 1)
    }

    pub fn custom(&mut self, name: &str, args: &[&str]) -> CustomId {
        self.customs.push(CustomSpec {
            name: name.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
        });
        CustomId(self.customs.len() as u16 - 1)
    }

    pub fn find_stat(&self, name: &str) -> Option<StatId> {
        self.stats.get(name).map(StatId)
    }

    pub fn find_resource(&self, name: &str) -> Option<ResourceId> {
        self.resources
            .iter()
            .position(|r| r.name == name)
            .map(|i| ResourceId(i as u16))
    }

    pub fn find_custom(&self, name: &str) -> Option<CustomId> {
        self.customs
            .iter()
            .position(|c| c.name == name)
            .map(|i| CustomId(i as u16))
    }

    /// Resolves a formula variable name.
    ///
    /// Forms, each optionally prefixed with `source.` (the default) or `target.`:
    /// `rank`, `stacks`, `value`, `distance`, `level`, `buff.<name>` (stack
    /// count), `<stat>`, `base_<stat>`, `bonus_<stat>`, `<resource>`,
    /// `missing_<resource>`, `<resource>_pct`.
    pub fn resolve_var(
        &self,
        name: &str,
        buff: &dyn Fn(&str) -> Option<BuffId>,
    ) -> Result<VarRef, String> {
        let (who, rest) = match name.split_once('.') {
            Some(("target", r)) => (Who::Target, r),
            Some(("source", r)) | Some(("self", r)) => (Who::Source, r),
            _ => (Who::Source, name),
        };
        match rest {
            "rank" => return Ok(VarRef::Rank),
            "stacks" => return Ok(VarRef::Stacks),
            "value" => return Ok(VarRef::Value),
            "distance" => return Ok(VarRef::Distance),
            "level" => return Ok(VarRef::Level(who)),
            _ => {}
        }
        if let Some(b) = rest.strip_prefix("buff.") {
            return buff(b)
                .map(|id| VarRef::BuffStacks(who, id))
                .ok_or_else(|| format!("unknown buff `{b}` in `{name}`"));
        }
        if let Some(s) = self.find_stat(rest) {
            return Ok(VarRef::Stat(who, s, StatPart::Total));
        }
        if let Some(r) = self.find_resource(rest) {
            return Ok(VarRef::Resource(who, r, ResPart::Current));
        }
        let stat_parts = [("base_", StatPart::Base), ("bonus_", StatPart::Bonus)];
        for (prefix, part) in stat_parts {
            if let Some(s) = rest.strip_prefix(prefix).and_then(|n| self.find_stat(n)) {
                return Ok(VarRef::Stat(who, s, part));
            }
        }
        if let Some(r) = rest
            .strip_prefix("missing_")
            .and_then(|n| self.find_resource(n))
        {
            return Ok(VarRef::Resource(who, r, ResPart::Missing));
        }
        if let Some(r) = rest
            .strip_suffix("_pct")
            .and_then(|n| self.find_resource(n))
        {
            return Ok(VarRef::Resource(who, r, ResPart::Pct));
        }
        Err(format!("unknown variable `{name}`"))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Who {
    Source,
    Target,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatPart {
    Total,
    Base,
    Bonus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResPart {
    Current,
    Missing,
    /// Current as a fraction of the maximum, `0..=1`.
    Pct,
}

/// A variable a formula can read, resolved against an effect context.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VarRef {
    Stat(Who, StatId, StatPart),
    Resource(Who, ResourceId, ResPart),
    Level(Who),
    BuffStacks(Who, BuffId),
    Rank,
    Stacks,
    Value,
    Distance,
}

/// A compiled formula over kernel variables.
pub type Ex = Expr<VarRef>;

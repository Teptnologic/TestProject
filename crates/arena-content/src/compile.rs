//! Turns raw files into kernel definitions, resolving every name and formula.

use crate::raw::*;
use crate::{AiRule, AiScript, ContentError, Pack};
use arena_formula::Const;
use arena_kernel::*;
use serde::de::DeserializeOwned;
use serde_yaml::{Mapping, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

type Res<T> = Result<T, ContentError>;
type Consts = BTreeMap<String, Const>;

fn err<T>(file: &Path, at: &str, message: impl Into<String>) -> Res<T> {
    Err(ContentError {
        file: file.to_path_buf(),
        at: at.to_string(),
        message: message.into(),
    })
}

fn read<T: DeserializeOwned>(path: &Path) -> Res<T> {
    let text = std::fs::read_to_string(path).or_else(|e| err(path, "", e.to_string()))?;
    serde_yaml::from_str(&text).or_else(|e| err(path, "", e.to_string()))
}

fn yaml_files(dir: &Path) -> Res<Vec<PathBuf>> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(Vec::new());
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "yaml" || x == "yml"))
        .collect();
    // Directory order differs between machines; ids must not.
    files.sort();
    Ok(files)
}

fn read_numbers(path: &Path) -> Res<BTreeMap<String, RawNumbers>> {
    if path.exists() {
        read(path)
    } else {
        Ok(BTreeMap::new())
    }
}

fn consts(generated: &BTreeMap<String, RawConst>, authored: &BTreeMap<String, RawConst>) -> Consts {
    generated
        .iter()
        .chain(authored)
        .map(|(k, v)| {
            let c = match v {
                RawConst::Num(n) => Const::Num(*n),
                RawConst::List(l) => Const::List(l.clone()),
            };
            (k.clone(), c)
        })
        .collect()
}

/// Authored values override generated ones.
fn merged(
    generated: &BTreeMap<String, f64>,
    authored: &BTreeMap<String, f64>,
) -> BTreeMap<String, f64> {
    generated
        .iter()
        .chain(authored)
        .map(|(k, v)| (k.clone(), *v))
        .collect()
}

pub fn pack_ruleset(dir: &Path) -> Res<String> {
    Ok(read::<RawPack>(&dir.join("pack.yaml"))?.ruleset)
}

pub fn load(dir: &Path, defs: &mut Defs, ruleset: &dyn Ruleset) -> Res<Pack> {
    let pack_file = dir.join("pack.yaml");
    let pack: RawPack = read(&pack_file)?;
    if pack.ruleset != ruleset.id() || pack.ruleset_version != ruleset.version() {
        return err(
            &pack_file,
            "",
            format!(
                "pack targets ruleset `{}` {} but the engine has `{}` {}",
                pack.ruleset,
                pack.ruleset_version,
                ruleset.id(),
                ruleset.version()
            ),
        );
    }
    let unit_numbers = read_numbers(&dir.join("generated/units.yaml"))?;
    let perk_numbers = read_numbers(&dir.join("generated/perks.yaml"))?;

    let mut units: Vec<(PathBuf, RawUnit)> = Vec::new();
    for file in yaml_files(&dir.join("authored/units"))? {
        units.push((file.clone(), read(&file)?));
    }
    let mut perks: Vec<(PathBuf, RawPerk)> = Vec::new();
    for file in yaml_files(&dir.join("authored/perks"))? {
        let list: Vec<RawPerk> = read(&file)?;
        perks.extend(list.into_iter().map(|p| (file.clone(), p)));
    }

    // Pass 1: register every name, so definitions can refer to each other.
    for (file, buffs, actions) in units
        .iter()
        .map(|(f, u)| (f, &u.buffs, &u.actions))
        .chain(perks.iter().map(|(f, p)| (f, &p.buffs, &p.actions)))
    {
        for b in buffs {
            if defs.buff_id(&b.id).is_some() {
                return err(file, &format!("buff {}", b.id), "duplicate buff id");
            }
            defs.add_buff(BuffDef::new(&b.id));
        }
        for a in actions {
            if defs.action_id(&a.id).is_some() {
                return err(file, &format!("action {}", a.id), "duplicate action id");
            }
            defs.add_action(ActionDef::new(&a.id));
        }
    }

    // Pass 2: compile.
    let mut ai_raw: Vec<(PathBuf, UnitId, RawAi, Consts)> = Vec::new();
    for (file, unit) in units {
        let numbers = unit_numbers.get(&unit.id).cloned().unwrap_or_default();
        let mut c = Compiler {
            defs,
            file: &file,
            consts: consts(&numbers.constants, &unit.constants),
        };
        for b in &unit.buffs {
            c.buff(b)?;
        }
        for a in &unit.actions {
            c.action(a)?;
        }
        let consts = c.consts.clone();
        let id = c.unit(&unit, &numbers)?;
        if let Some(ai) = unit.ai {
            ai_raw.push((file, id, ai, consts));
        }
    }
    for (file, perk) in perks {
        let numbers = perk_numbers.get(&perk.id).cloned().unwrap_or_default();
        let mut c = Compiler {
            defs,
            file: &file,
            consts: consts(&numbers.constants, &perk.constants),
        };
        for b in &perk.buffs {
            c.buff(b)?;
        }
        for a in &perk.actions {
            c.action(a)?;
        }
        c.perk(&perk, &numbers)?;
    }

    // AI hints come last: builds refer to perks.
    let mut ai = BTreeMap::new();
    for (file, id, raw, consts) in ai_raw {
        let mut c = Compiler {
            defs,
            file: &file,
            consts,
        };
        ai.insert(id, c.ai(id, &raw)?);
    }
    Ok(Pack { id: pack.id, ai })
}

struct Compiler<'a> {
    defs: &'a mut Defs,
    file: &'a Path,
    consts: Consts,
}

/// Reads the fields of one YAML mapping and rejects any it was not asked for.
struct Fields<'v> {
    map: Option<&'v Mapping>,
    used: Vec<String>,
}

impl<'v> Fields<'v> {
    fn get(&mut self, key: &str) -> Option<&'v Value> {
        self.used.push(key.to_string());
        self.map.and_then(|m| m.get(key))
    }

    fn unknown(&self) -> Option<String> {
        let keys = self.map?.keys().filter_map(|k| k.as_str());
        keys.filter(|k| !self.used.iter().any(|u| u == k))
            .map(str::to_string)
            .next()
    }
}

impl Compiler<'_> {
    fn fail<T>(&self, at: &str, message: impl Into<String>) -> Res<T> {
        err(self.file, at, message)
    }

    fn ex_str(&self, at: &str, src: &str) -> Res<Ex> {
        self.defs
            .formula(src, &self.consts)
            .or_else(|e| self.fail(at, e.to_string()))
    }

    fn ex(&self, at: &str, raw: &RawEx) -> Res<Ex> {
        match raw {
            RawEx::Num(n) => Ok(Ex::Num(*n)),
            RawEx::Str(s) => self.ex_str(at, s),
        }
    }

    fn ex_or(&self, at: &str, raw: &Option<RawEx>, default: f64) -> Res<Ex> {
        raw.as_ref()
            .map_or(Ok(Ex::Num(default)), |r| self.ex(at, r))
    }

    fn ex_value(&self, at: &str, v: &Value) -> Res<Ex> {
        match v {
            Value::Number(n) => Ok(Ex::Num(n.as_f64().unwrap_or(0.0))),
            Value::Bool(b) => Ok(Ex::Num(if *b { 1.0 } else { 0.0 })),
            Value::String(s) => self.ex_str(at, s),
            _ => self.fail(at, "expected a number or a formula"),
        }
    }

    fn stat(&self, at: &str, name: &str) -> Res<StatId> {
        self.defs
            .reg
            .find_stat(name)
            .map_or_else(|| self.fail(at, format!("unknown stat `{name}`")), Ok)
    }

    fn tag(&self, at: &str, name: &str) -> Res<TagId> {
        let id = self.defs.reg.tags.get(name).map(TagId);
        id.map_or_else(|| self.fail(at, format!("unknown tag `{name}`")), Ok)
    }

    fn tags(&self, at: &str, names: &[String]) -> Res<Vec<TagId>> {
        names.iter().map(|n| self.tag(at, n)).collect()
    }

    fn event(&self, at: &str, name: &str) -> Res<EventKind> {
        let id = self.defs.reg.events.get(name).map(EventKind);
        id.map_or_else(|| self.fail(at, format!("unknown event `{name}`")), Ok)
    }

    fn buff_ref(&self, at: &str, name: &str) -> Res<BuffId> {
        self.defs
            .buff_id(name)
            .map_or_else(|| self.fail(at, format!("unknown buff `{name}`")), Ok)
    }

    fn action_ref(&self, at: &str, name: &str) -> Res<ActionId> {
        self.defs
            .action_id(name)
            .map_or_else(|| self.fail(at, format!("unknown action `{name}`")), Ok)
    }

    fn layer(&self, at: &str, name: Option<&str>) -> Res<Layer> {
        match name.unwrap_or("flat") {
            "flat" => Ok(Layer::Flat),
            "pct_add" => Ok(Layer::PctAdd),
            "pct_mult" => Ok(Layer::PctMult),
            other => self.fail(
                at,
                format!("unknown layer `{other}` (flat, pct_add, pct_mult)"),
            ),
        }
    }

    fn filter(&self, at: &str, name: Option<&str>) -> Res<Filter> {
        match name.unwrap_or("enemy") {
            "enemy" => Ok(Filter::Enemy),
            "ally" => Ok(Filter::Ally),
            "any" => Ok(Filter::Any),
            other => self.fail(at, format!("unknown filter `{other}` (enemy, ally, any)")),
        }
    }

    fn who(&self, at: &str, name: Option<&str>) -> Res<Who> {
        match name.unwrap_or("target") {
            "target" => Ok(Who::Target),
            "source" | "self" => Ok(Who::Source),
            other => self.fail(at, format!("unknown subject `{other}` (source, target)")),
        }
    }

    // ----- effects -----

    fn effects(&mut self, at: &str, list: &[Value]) -> Res<Option<EffectListId>> {
        if list.is_empty() {
            return Ok(None);
        }
        let mut out = Vec::new();
        for (i, v) in list.iter().enumerate() {
            out.push(self.effect(&format!("{at}[{i}]"), v)?);
        }
        Ok(Some(self.defs.add_effects(out)))
    }

    fn nested(&mut self, at: &str, f: &mut Fields, key: &'static str) -> Res<Option<EffectListId>> {
        match f.get(key) {
            None => Ok(None),
            Some(Value::Sequence(list)) => self.effects(&format!("{at} > {key}"), list),
            Some(_) => self.fail(at, format!("`{key}` must be a list of effects")),
        }
    }

    fn effect(&mut self, at: &str, v: &Value) -> Res<Effect> {
        let entry = v
            .as_mapping()
            .filter(|m| m.len() == 1)
            .and_then(|m| m.iter().next());
        let Some((Value::String(kind), body)) = entry else {
            return self.fail(at, "an effect is a single `name: {fields}` entry");
        };
        let at = &format!("{at} {kind}");
        let mut f = Fields {
            map: body.as_mapping(),
            used: Vec::new(),
        };
        if f.map.is_none() && !body.is_null() {
            return self.fail(at, "effect fields must be a mapping");
        }
        // Small readers over the field set; each records the key as known.
        let text = |f: &mut Fields, key| f.get(key).and_then(Value::as_str).map(str::to_string);
        let ex = |c: &Self, f: &mut Fields, key| f.get(key).map(|v| c.ex_value(at, v)).transpose();
        let need = |c: &Self, f: &mut Fields, key: &'static str| {
            ex(c, f, key)?.map_or_else(|| c.fail(at, format!("missing `{key}`")), Ok)
        };
        let need_text = |c: &Self, f: &mut Fields, key: &'static str| {
            text(f, key).map_or_else(|| c.fail(at, format!("missing `{key}`")), Ok)
        };

        let effect = match kind.as_str() {
            "apply_buff" => Effect::ApplyBuff {
                buff: self.buff_ref(at, &need_text(self, &mut f, "buff")?)?,
                who: self.who(at, text(&mut f, "to").as_deref())?,
                duration: ex(self, &mut f, "duration")?,
                stacks: f.get("stacks").and_then(Value::as_u64).unwrap_or(1) as u32,
                value: ex(self, &mut f, "value")?,
            },
            "remove_buff" => Effect::RemoveBuff {
                buff: self.buff_ref(at, &need_text(self, &mut f, "buff")?)?,
                who: self.who(at, text(&mut f, "from").as_deref())?,
            },
            "resource" => {
                let name = need_text(self, &mut f, "name")?;
                let Some(resource) = self.defs.reg.find_resource(&name) else {
                    return self.fail(at, format!("unknown resource `{name}`"));
                };
                Effect::ModifyResource {
                    resource,
                    who: self.who(at, text(&mut f, "to").as_deref())?,
                    amount: need(self, &mut f, "amount")?,
                }
            }
            "projectile" => Effect::Projectile {
                speed: need(self, &mut f, "speed")?,
                radius: f.get("radius").and_then(Value::as_f64).unwrap_or(0.0),
                range: ex(self, &mut f, "range")?,
                homing: f.get("homing").and_then(Value::as_bool).unwrap_or(false),
                pierce: f.get("pierce").and_then(Value::as_u64).unwrap_or(0) as u32,
                filter: self.filter(at, text(&mut f, "filter").as_deref())?,
                on_hit: match self.nested(at, &mut f, "on_hit")? {
                    Some(list) => list,
                    None => return self.fail(at, "missing `on_hit`"),
                },
            },
            "area" => Effect::Area {
                center: match text(&mut f, "center").as_deref().unwrap_or("target") {
                    "source" | "self" => Center::Source,
                    "target" => Center::Target,
                    "point" => Center::Point,
                    other => return self.fail(at, format!("unknown center `{other}`")),
                },
                radius: need(self, &mut f, "radius")?,
                filter: self.filter(at, text(&mut f, "filter").as_deref())?,
                effects: match self.nested(at, &mut f, "effects")? {
                    Some(list) => list,
                    None => return self.fail(at, "missing `effects`"),
                },
            },
            "dash" => Effect::Dash {
                who: self.who(at, Some(text(&mut f, "who").as_deref().unwrap_or("source")))?,
                to: match text(&mut f, "to").as_deref().unwrap_or("target") {
                    "target" => DashTo::Target,
                    "point" => DashTo::Point,
                    "away" => DashTo::AwayFromSource,
                    other => return self.fail(at, format!("unknown destination `{other}`")),
                },
                distance: ex(self, &mut f, "distance")?,
                speed: ex(self, &mut f, "speed")?.unwrap_or(Ex::Num(0.0)),
            },
            "if" => Effect::If {
                cond: need(self, &mut f, "cond")?,
                then: match self.nested(at, &mut f, "then")? {
                    Some(list) => list,
                    None => return self.fail(at, "missing `then`"),
                },
                otherwise: self.nested(at, &mut f, "else")?,
            },
            "emit" => Effect::Emit {
                kind: self.event(at, &need_text(self, &mut f, "event")?)?,
                value: ex(self, &mut f, "value")?,
            },
            "score" => Effect::AddScore {
                amount: need(self, &mut f, "amount")?,
            },
            "reset_cooldown" => Effect::ResetCooldown {
                action: self.action_ref(at, &need_text(self, &mut f, "action")?)?,
            },
            custom => {
                let Some(id) = self.defs.reg.find_custom(custom) else {
                    return self.fail(at, format!("unknown effect `{custom}`"));
                };
                let who = self.who(at, text(&mut f, "to").as_deref())?;
                let mut tags = Vec::new();
                // `type: physical` is shorthand for a tag.
                if let Some(t) = text(&mut f, "type") {
                    tags.push(self.tag(at, &t)?);
                }
                if let Some(list) = f.get("tags") {
                    let Some(list) = list.as_sequence() else {
                        return self.fail(at, "`tags` must be a list");
                    };
                    for t in list {
                        let Some(name) = t.as_str() else {
                            return self.fail(at, "tags are names");
                        };
                        tags.push(self.tag(at, name)?);
                    }
                }
                let spec = self.defs.reg.customs[id.idx()].clone();
                let mut args = Vec::new();
                for name in &spec.args {
                    args.push(f.get(name).map(|v| self.ex_value(at, v)).transpose()?);
                }
                Effect::Custom {
                    kind: id,
                    who,
                    args,
                    tags,
                }
            }
        };
        if let Some(key) = f.unknown() {
            return self.fail(at, format!("unknown field `{key}`"));
        }
        Ok(effect)
    }

    // ----- definitions -----

    fn modifiers(&self, at: &str, raw: &[RawMod]) -> Res<Vec<(StatId, Layer, Ex)>> {
        raw.iter()
            .map(|m| {
                Ok((
                    self.stat(at, &m.stat)?,
                    self.layer(at, m.layer.as_deref())?,
                    self.ex(at, &m.value)?,
                ))
            })
            .collect()
    }

    fn buff(&mut self, raw: &RawBuff) -> Res<BuffId> {
        let at = &format!("buff {}", raw.id);
        let id = self.buff_ref(at, &raw.id)?;
        let mut def = BuffDef::new(&raw.id);
        def.duration = raw.duration.as_ref().map(|d| self.ex(at, d)).transpose()?;
        def.max_stacks = raw.max_stacks.unwrap_or(1).max(1);
        def.policy = match raw.policy.as_deref().unwrap_or("refresh") {
            "refresh" => StackPolicy::Refresh,
            "add_stack" => StackPolicy::AddStack,
            "independent" => StackPolicy::Independent,
            "stronger" => StackPolicy::Stronger,
            other => return self.fail(at, format!("unknown policy `{other}`")),
        };
        def.modifiers = self.modifiers(at, &raw.modifiers)?;
        for flag in &raw.flags {
            let Some(id) = self.defs.reg.flags.get(flag) else {
                return self.fail(at, format!("unknown flag `{flag}`"));
            };
            def.flags.push(FlagId(id));
        }
        def.tags = self.tags(at, &raw.tags)?;
        def.on_apply = self.effects(&format!("{at} > on_apply"), &raw.on_apply)?;
        def.on_expire = self.effects(&format!("{at} > on_expire"), &raw.on_expire)?;
        match (
            &raw.every,
            self.effects(&format!("{at} > on_tick"), &raw.on_tick)?,
        ) {
            (Some(every), Some(list)) => def.periodic = Some((self.ex(at, every)?, list)),
            (None, None) => {}
            _ => return self.fail(at, "`every` and `on_tick` must be given together"),
        }
        for (i, l) in raw.listeners.iter().enumerate() {
            let at = &format!("{at} > listeners[{i}]");
            let Some(effects) = self.effects(at, &l.effects)? else {
                return self.fail(at, "a listener needs at least one effect");
            };
            def.listeners.push(Listener {
                event: self.event(at, &l.on)?,
                role: match l.role.as_deref().unwrap_or("source") {
                    "source" => Role::Source,
                    "target" => Role::Target,
                    other => return self.fail(at, format!("unknown role `{other}`")),
                },
                cooldown: l.cooldown,
                cond: l.when.as_ref().map(|w| self.ex(at, w)).transpose()?,
                effects,
                consume: l.consume,
            });
        }
        self.defs.replace_buff(id, def);
        Ok(id)
    }

    fn action(&mut self, raw: &RawAction) -> Res<ActionId> {
        let at = &format!("action {}", raw.id);
        let id = self.action_ref(at, &raw.id)?;
        let mut def = ActionDef::new(&raw.id);
        def.tags = self.tags(at, &raw.tags)?;
        def.max_rank = raw.max_rank.unwrap_or(1);
        def.target = match raw.target.as_deref().unwrap_or("none") {
            "none" => TargetKind::None,
            "point" => TargetKind::Point,
            other => TargetKind::Unit(self.filter(at, Some(other))?),
        };
        def.range = raw.range.as_ref().map(|r| self.ex(at, r)).transpose()?;
        def.cast_time = self.ex_or(at, &raw.cast_time, 0.0)?;
        def.cooldown = self.ex_or(at, &raw.cooldown, 0.0)?;
        for (name, cost) in &raw.costs {
            let Some(res) = self.defs.reg.find_resource(name) else {
                return self.fail(at, format!("unknown resource `{name}`"));
            };
            def.costs.push((res, self.ex(at, cost)?));
        }
        def.usable_when = raw
            .usable_when
            .as_ref()
            .map(|w| self.ex(at, w))
            .transpose()?;
        def.lock_move = raw.lock_move;
        def.cancel_on_move = raw.cancel_on_move;
        def.on_cast = self.effects(&format!("{at} > on_cast"), &raw.on_cast)?;
        def.on_execute = self.effects(&format!("{at} > on_execute"), &raw.on_execute)?;
        self.defs.replace_action(id, def);
        Ok(id)
    }

    fn slots(&self, actions: &[RawAction]) -> Res<Vec<SlotDef>> {
        let mut slots: Vec<SlotDef> = Vec::new();
        for a in actions {
            let Some(key) = &a.slot else { continue };
            let at = &format!("action {}", a.id);
            if slots.iter().any(|s| &s.key == key) {
                return self.fail(at, format!("slot `{key}` is used twice"));
            }
            slots.push(SlotDef {
                key: key.clone(),
                action: self.action_ref(at, &a.id)?,
            });
        }
        Ok(slots)
    }

    fn stat_map(&self, at: &str, map: &BTreeMap<String, f64>) -> Res<Vec<(StatId, f64)>> {
        map.iter()
            .map(|(name, v)| Ok((self.stat(at, name)?, *v)))
            .collect()
    }

    fn unit(&mut self, raw: &RawUnit, numbers: &RawNumbers) -> Res<UnitId> {
        let at = &format!("unit {}", raw.id);
        if self.defs.unit_id(&raw.id).is_some() {
            return self.fail(at, "duplicate unit id");
        }
        let slots = self.slots(&raw.actions)?;
        for key in &raw.rank_priority {
            if !slots.iter().any(|s| &s.key == key) {
                return self.fail(at, format!("rank_priority names unknown slot `{key}`"));
            }
        }
        let def = UnitDef {
            id: UnitId(0),
            name: raw.id.clone(),
            radius: raw.radius,
            stats: self.stat_map(at, &merged(&numbers.stats, &raw.stats))?,
            growth: self.stat_map(at, &merged(&numbers.growth, &raw.growth))?,
            params: merged(&numbers.params, &raw.params),
            slots,
            passives: raw
                .passives
                .iter()
                .map(|b| self.buff_ref(at, b))
                .collect::<Res<_>>()?,
            tags: self.tags(at, &raw.tags)?,
            rank_priority: raw.rank_priority.clone(),
        };
        Ok(self.defs.add_unit(def))
    }

    fn perk(&mut self, raw: &RawPerk, numbers: &RawNumbers) -> Res<PerkId> {
        let at = &format!("perk {}", raw.id);
        if self.defs.perk_id(&raw.id).is_some() {
            return self.fail(at, "duplicate perk id");
        }
        let mut modifiers: Vec<(StatId, Layer, f64)> = self
            .stat_map(at, &merged(&numbers.stats, &raw.stats))?
            .into_iter()
            .map(|(s, v)| (s, Layer::Flat, v))
            .collect();
        for (stat, layer, ex) in self.modifiers(at, &raw.modifiers)? {
            let Some(value) = ex.as_const() else {
                return self.fail(
                    at,
                    "perk modifiers must be plain numbers; use a buff for formulas",
                );
            };
            modifiers.push((stat, layer, value));
        }
        let def = PerkDef {
            id: PerkId(0),
            name: raw.id.clone(),
            category: raw.category.clone().unwrap_or_else(|| "item".into()),
            group: raw.group.clone(),
            cost: raw.cost.or(numbers.cost).unwrap_or(0.0),
            modifiers,
            buffs: raw
                .passives
                .iter()
                .map(|b| self.buff_ref(at, b))
                .collect::<Res<_>>()?,
            slots: self.slots(&raw.actions)?,
        };
        Ok(self.defs.add_perk(def))
    }

    fn ai(&mut self, unit: UnitId, raw: &RawAi) -> Res<AiScript> {
        let at = "ai";
        let slots: Vec<String> = self
            .defs
            .unit(unit)
            .slots
            .iter()
            .map(|s| s.key.clone())
            .collect();
        let mut script = AiScript {
            attack_slot: raw.attack.clone(),
            kite: raw.kite,
            ..AiScript::default()
        };
        if let Some(key) = raw.attack.as_ref().filter(|k| !slots.contains(k)) {
            return self.fail(at, format!("attack names unknown slot `{key}`"));
        }
        for name in &raw.build {
            let Some(perk) = self.defs.perk_id(name) else {
                return self.fail(at, format!("build names unknown perk `{name}`"));
            };
            script.build.push(perk);
        }
        for (i, rule) in raw.rules.iter().enumerate() {
            let at = &format!("ai > rules[{i}]");
            if !slots.contains(&rule.slot) {
                return self.fail(at, format!("rule names unknown slot `{}`", rule.slot));
            }
            script.rules.push(AiRule {
                slot: rule.slot.clone(),
                when: rule.when.as_ref().map(|w| self.ex(at, w)).transpose()?,
            });
        }
        Ok(script)
    }
}

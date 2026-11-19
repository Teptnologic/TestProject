//! Checks the ruleset against known League of Legends numbers.
//! Formula sources: the League of Legends Wiki pages named in each test.

use arena_kernel::*;
use arena_ruleset_lol::Lol;
use std::collections::BTreeMap;
use std::sync::Arc;

struct Fixture {
    w: World,
    lol: Arc<Lol>,
    a: EntityId,
    b: EntityId,
}

/// Two identical dummies with 1000 health, 100 AD and an attack in slot 0.
fn fixture(extra_stats: &[(&str, f64)]) -> Fixture {
    let mut defs = Defs::default();
    let lol = Arc::new(Lol::new(&mut defs));
    let v = &lol.v;
    let none = BTreeMap::new();

    let amount = Some(defs.formula("ad", &none).unwrap());
    let hit = defs.add_effects(vec![Effect::Custom {
        kind: v.fx_damage,
        who: Who::Target,
        args: vec![amount, None],
        tags: vec![v.physical, v.basic_attack],
    }]);
    let mut attack = ActionDef::new("attack");
    attack.tags = vec![v.basic_attack];
    attack.target = TargetKind::Unit(Filter::Enemy);
    attack.range = Some(defs.formula("attack_range", &none).unwrap());
    attack.lock_move = true;
    attack.on_execute = Some(hit);
    let attack = defs.add_action(attack);

    let mut stats = vec![
        (v.max_hp, 1000.0),
        (v.ad, 100.0),
        (v.move_speed, 340.0),
        (v.attack_range, 500.0),
    ];
    for (name, value) in extra_stats {
        let id = defs.reg.find_stat(name).unwrap();
        stats.retain(|(s, _)| *s != id);
        stats.push((id, *value));
    }
    let params = BTreeMap::from([
        ("base_as".to_string(), 0.625),
        ("windup_pct".to_string(), 0.2),
    ]);
    let dummy = defs.add_unit(UnitDef {
        id: UnitId(0),
        name: "dummy".into(),
        radius: 50.0,
        stats,
        growth: vec![(v.max_hp, 98.0)],
        params,
        slots: vec![SlotDef {
            key: "A".into(),
            action: attack,
        }],
        passives: vec![],
        tags: vec![],
        rank_priority: vec![],
    });

    let cfg = WorldConfig {
        tick: TickConfig::default(),
        seed: 1,
        size: Vec2::new(2000.0, 2000.0),
    };
    let mut w = World::new(Arc::new(defs), lol.clone(), cfg);
    let mut spawn = |team, x| {
        w.spawn_unit(SpawnUnit {
            unit: dummy,
            team: TeamId(team),
            pos: Vec2::new(x, 500.0),
            level: 1,
            perks: vec![],
            ranks: None,
        })
    };
    let (a, b) = (spawn(0, 500.0), spawn(1, 800.0));
    Fixture { w, lol, a, b }
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

/// Applies `amount` raw damage of a type from `a` to `b`; returns the health lost.
fn hit(f: &mut Fixture, amount: f64, tag: TagId) -> f64 {
    let before = f.w.resource(f.b, f.lol.v.hp);
    let ctx = Ctx::new(f.a).with_target(f.b);
    let (kind, b) = (f.lol.v.fx_damage, f.b);
    f.lol
        .clone()
        .apply_custom(&mut f.w, kind, b, &[Some(amount), None], &[tag], &ctx);
    before - f.w.resource(f.b, f.lol.v.hp)
}

#[test]
fn stat_growth_follows_the_level_curve() {
    // Wiki "Champion statistic": stat = base + growth × (n−1) × (0.7025 + 0.0175 × (n−1)).
    // Garen: 690 health, +98 per level.
    assert!(close(Lol::stat_at_level(690.0, 98.0, 1), 690.0));
    assert!(close(Lol::stat_at_level(690.0, 98.0, 6), 1077.1));
    assert!(close(Lol::stat_at_level(690.0, 98.0, 18), 2356.0));
}

#[test]
fn resistances_and_penetration() {
    // Wiki "Armor": multiplier = 100 / (100 + armor), or 2 − 100 / (100 − armor) when negative.
    assert!(close(Lol::resist_multiplier(100.0), 0.5));
    assert!(close(Lol::resist_multiplier(-50.0), 2.0 - 100.0 / 150.0));
    // Wiki "Armor penetration": percentage penetration applies before lethality.
    assert!(close(Lol::effective_resist(100.0, 0.4, 10.0), 50.0));
    assert!(close(Lol::effective_resist(20.0, 0.0, 50.0), 0.0));
    assert!(close(Lol::effective_resist(-20.0, 0.4, 10.0), -20.0));

    let mut f = fixture(&[("armor", 100.0), ("mr", 50.0)]);
    let (physical, magic, true_damage) = (f.lol.v.physical, f.lol.v.magic, f.lol.v.true_damage);
    assert!(close(hit(&mut f, 100.0, physical), 50.0));
    assert!(close(hit(&mut f, 150.0, magic), 100.0));
    assert!(close(hit(&mut f, 70.0, true_damage), 70.0));
}

#[test]
fn movement_speed_soft_caps_and_slows() {
    // Wiki "Movement speed": above 490 → ×0.5 + 230; 415–490 → ×0.8 + 83; below 220 → ×0.5 + 110.
    assert!(close(Lol::capped_move_speed(340.0, 0.0), 340.0));
    assert!(close(Lol::capped_move_speed(450.0, 0.0), 443.0));
    assert!(close(Lol::capped_move_speed(600.0, 0.0), 530.0));
    assert!(close(Lol::capped_move_speed(340.0, 0.5), 195.0));

    // Only the strongest slow applies.
    let mut f = fixture(&[]);
    let ctx = Ctx::new(f.a).with_target(f.b);
    let (slow, b) = (f.lol.v.fx_slow, f.b);
    for amount in [0.2, 0.3] {
        f.lol
            .clone()
            .apply_custom(&mut f.w, slow, b, &[Some(amount), Some(2.0)], &[], &ctx);
    }
    assert!(close(f.lol.move_speed(&f.w, f.b), 340.0 * 0.7));
}

#[test]
fn attack_timing_scales_with_attack_speed() {
    // Wiki "Basic attack": period = 1 / attack speed; wind-up is a fixed share of it.
    let f = fixture(&[]);
    let attack = f.w.defs.action(ActionId(0)).clone();
    let t = f.lol.action_timing(&f.w, f.a, &attack, 1);
    assert_eq!((t.cooldown, t.windup), (1_600_000, 320_000));

    // +100% bonus attack speed doubles 0.625 to 1.25 attacks per second.
    let f = fixture(&[("attack_speed", 1.0)]);
    let t = f.lol.action_timing(&f.w, f.a, &attack, 1);
    assert_eq!((t.cooldown, t.windup), (800_000, 160_000));

    // Attack speed is capped at 2.5.
    let f = fixture(&[("attack_speed", 10.0)]);
    assert_eq!(f.lol.action_timing(&f.w, f.a, &attack, 1).cooldown, 400_000);
}

#[test]
fn attacks_land_after_the_windup_and_repeat_on_the_period() {
    let mut f = fixture(&[]);
    let attack = Command::Use {
        slot: 0,
        target: Target::Entity(f.b),
    };
    let mut landed = Vec::new();
    for _ in 0..150 {
        if f.w.legal(f.a, &attack).is_ok() {
            f.w.submit(f.a, attack).unwrap();
        }
        f.w.tick();
        let on_hit = f.lol.v.ev_on_hit;
        landed.extend(
            f.w.drain_events()
                .iter()
                .filter(|e| e.kind == on_hit)
                .map(|e| e.time.0),
        );
    }
    // Commands and timers land on 33.333 ms ticks: the 0.32 s wind-up takes 10
    // ticks and the 1.6 s period becomes 49 ticks.
    assert_eq!(landed, vec![333_330, 1_966_647, 3_599_964]);
    assert!(close(f.w.resource(f.b, f.lol.v.hp), 700.0));
}

#[test]
fn shields_absorb_before_health_and_soonest_expiry_first() {
    let mut f = fixture(&[]);
    let ctx = Ctx::new(f.b).with_target(f.b);
    let (shield, b, tag) = (f.lol.v.fx_shield, f.b, f.lol.v.true_damage);
    f.lol
        .clone()
        .apply_custom(&mut f.w, shield, b, &[Some(100.0), Some(5.0)], &[], &ctx);
    f.lol
        .clone()
        .apply_custom(&mut f.w, shield, b, &[Some(50.0), Some(1.0)], &[], &ctx);
    assert!(close(hit(&mut f, 80.0, tag), 0.0));
    let left: Vec<f64> =
        f.w.ent(f.b)
            .unwrap()
            .buffs
            .iter()
            .map(|s| s.value)
            .collect();
    assert_eq!(left, vec![70.0]);
    assert!(close(hit(&mut f, 100.0, tag), 30.0));
}

#[test]
fn grievous_wounds_cut_healing_by_forty_percent() {
    let mut f = fixture(&[]);
    let (hp, heal, grievous, b, tag) = (
        f.lol.v.hp,
        f.lol.v.fx_heal,
        f.lol.v.fx_grievous,
        f.b,
        f.lol.v.true_damage,
    );
    hit(&mut f, 500.0, tag);
    let ctx = Ctx::new(f.b).with_target(f.b);
    f.lol
        .clone()
        .apply_custom(&mut f.w, heal, b, &[Some(100.0)], &[], &ctx);
    assert!(close(f.w.resource(b, hp), 600.0));
    f.lol
        .clone()
        .apply_custom(&mut f.w, grievous, b, &[Some(3.0)], &[], &ctx);
    f.lol
        .clone()
        .apply_custom(&mut f.w, heal, b, &[Some(100.0)], &[], &ctx);
    assert!(close(f.w.resource(b, hp), 660.0));
}

#[test]
fn stuns_block_actions_and_tenacity_shortens_them() {
    let mut f = fixture(&[("tenacity", 0.3)]);
    let ctx = Ctx::new(f.a).with_target(f.b);
    let (stun, b) = (f.lol.v.fx_stun, f.b);
    f.lol
        .clone()
        .apply_custom(&mut f.w, stun, b, &[Some(2.0)], &[], &ctx);
    let attack = Command::Use {
        slot: 0,
        target: Target::Entity(f.a),
    };
    assert_eq!(f.w.legal(b, &attack), Err(Illegal::Blocked));
    assert_eq!(
        f.w.legal(b, &Command::MoveTo(Vec2::ZERO)),
        Err(Illegal::CannotMove)
    );
    // 2 s × (1 − 0.3) = 1.4 s = 42 ticks.
    for _ in 0..41 {
        f.w.tick();
    }
    assert_eq!(f.w.legal(b, &attack), Err(Illegal::Blocked));
    f.w.tick();
    f.w.tick();
    assert_eq!(f.w.legal(b, &attack), Ok(()));
}

#[test]
fn lethal_damage_kills_and_credits_the_killer() {
    let mut f = fixture(&[]);
    let tag = f.lol.v.true_damage;
    hit(&mut f, 5000.0, tag);
    // Death is carried out once the instant has fully resolved.
    assert!(f.w.is_alive(f.b));
    f.w.resolve_deaths();
    assert!(!f.w.is_alive(f.b));
    let died = f.w.defs.reg.ev.died;
    let ev =
        f.w.drain_events()
            .into_iter()
            .find(|e| e.kind == died)
            .unwrap();
    assert_eq!((ev.source, ev.target), (Some(f.a), Some(f.b)));
}

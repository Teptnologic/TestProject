//! Drives the kernel with a made-up game that shares nothing with any real
//! one. If this passes, the kernel carries no game-specific knowledge.

use arena_kernel::*;
use std::collections::BTreeMap;
use std::sync::Arc;

struct Toy {
    life: ResourceId,
    max_life: StatId,
    speed: StatId,
    hurt: CustomId,
    frozen: FlagId,
}

impl Ruleset for Toy {
    fn id(&self) -> &'static str {
        "toy"
    }

    fn version(&self) -> &'static str {
        "0.1"
    }

    fn base_stats(&self, _: &Defs, unit: &UnitDef, _: u32) -> Vec<(StatId, f64)> {
        unit.stats.clone()
    }

    fn default_ranks(&self, _: &Defs, unit: &UnitDef, _: u32) -> Vec<u8> {
        vec![1; unit.slots.len()]
    }

    fn validate_loadout(&self, _: &Defs, _: &UnitDef, _: &[PerkId], _: f64) -> Result<(), String> {
        Ok(())
    }

    fn init_unit(&self, w: &mut World, e: EntityId) {
        let max = w.stat(e, self.max_life);
        w.set_resource(e, self.life, max);
    }

    fn vitality(&self, w: &World, e: EntityId) -> f64 {
        w.resource(e, self.life) / w.resource_max(e, self.life).max(1.0)
    }

    fn move_speed(&self, w: &World, e: EntityId) -> f64 {
        w.stat(e, self.speed)
    }

    fn can_move(&self, w: &World, e: EntityId) -> bool {
        !w.has_flag(e, self.frozen)
    }

    fn can_use(&self, w: &World, e: EntityId, _: &ActionDef) -> bool {
        !w.has_flag(e, self.frozen)
    }

    fn targetable(&self, w: &World, _: EntityId, target: EntityId) -> bool {
        w.is_alive(target)
    }

    fn action_timing(&self, w: &World, e: EntityId, a: &ActionDef, rank: u8) -> ActionTiming {
        let mut ctx = Ctx::new(e);
        ctx.rank = rank;
        ActionTiming {
            windup: micros(w.eval(&a.cast_time, &ctx)),
            recovery: 0,
            cooldown: micros(w.eval(&a.cooldown, &ctx)),
        }
    }

    fn apply_custom(
        &self,
        w: &mut World,
        kind: CustomId,
        target: EntityId,
        args: &[Option<f64>],
        _: &[TagId],
        ctx: &Ctx,
    ) {
        assert_eq!(kind, self.hurt);
        let left = w.resource(target, self.life) - args[0].unwrap_or(0.0);
        w.set_resource(target, self.life, left);
        if left <= 0.0 {
            w.kill(target, Some(ctx.source));
        }
    }

    fn on_event(&self, _: &mut World, _: &Event) {}
}

struct Game {
    defs: Arc<Defs>,
    ruleset: Arc<dyn Ruleset>,
    blob: UnitId,
    chill: BuffId,
    pile: BuffId,
}

fn game() -> Game {
    let mut defs = Defs::default();
    let max_life = defs.reg.stat("max_life");
    let speed = defs.reg.stat("speed");
    let life = defs.reg.resource(ResourceDef {
        name: "life".into(),
        max_stat: max_life,
        regen_stat: None,
        regen_per_sec: 0.0,
    });
    let hurt = defs.reg.custom("hurt", &["amount"]);
    let frozen = defs.reg.flag("frozen");
    let none = BTreeMap::new();
    let f = |defs: &Defs, s: &str| defs.formula(s, &none).unwrap();

    let mut chill = BuffDef::new("chill");
    chill.duration = Some(f(&defs, "1"));
    chill.flags = vec![frozen];
    let chill = defs.add_buff(chill);

    let mut pile = BuffDef::new("pile");
    pile.duration = Some(f(&defs, "2"));
    pile.max_stacks = 3;
    pile.policy = StackPolicy::AddStack;
    pile.modifiers = vec![(speed, Layer::Flat, f(&defs, "10 * stacks"))];
    let pile = defs.add_buff(pile);

    let hurt_fx = |defs: &mut Defs, amount: &str| {
        let amount = Some(f(defs, amount));
        defs.add_effects(vec![Effect::Custom {
            kind: hurt,
            who: Who::Target,
            args: vec![amount],
            tags: vec![],
        }])
    };

    let mut poke = ActionDef::new("poke");
    poke.target = TargetKind::Unit(Filter::Enemy);
    poke.range = Some(f(&defs, "50"));
    poke.cast_time = f(&defs, "0.2");
    poke.cooldown = f(&defs, "1");
    poke.lock_move = true;
    poke.on_execute = Some(hurt_fx(&mut defs, "10 + 5 * rank"));
    let poke = defs.add_action(poke);

    let on_hit = hurt_fx(&mut defs, "25");
    let mut bolt = ActionDef::new("bolt");
    bolt.target = TargetKind::Point;
    bolt.cooldown = f(&defs, "3");
    bolt.on_execute = Some(defs.add_effects(vec![Effect::Projectile {
        speed: f(&defs, "20000"),
        radius: 5.0,
        range: Some(f(&defs, "2000")),
        homing: false,
        pierce: 0,
        filter: Filter::Enemy,
        on_hit,
    }]));
    let bolt = defs.add_action(bolt);

    let blob = defs.add_unit(UnitDef {
        id: UnitId(0),
        name: "blob".into(),
        radius: 20.0,
        stats: vec![(max_life, 100.0), (speed, 300.0)],
        growth: vec![],
        params: BTreeMap::new(),
        slots: vec![
            SlotDef {
                key: "poke".into(),
                action: poke,
            },
            SlotDef {
                key: "bolt".into(),
                action: bolt,
            },
        ],
        passives: vec![],
        tags: vec![],
        rank_priority: vec![],
    });
    let ruleset = Arc::new(Toy {
        life,
        max_life,
        speed,
        hurt,
        frozen,
    });
    Game {
        defs: Arc::new(defs),
        ruleset,
        blob,
        chill,
        pile,
    }
}

fn world(g: &Game, seed: u64) -> (World, EntityId, EntityId) {
    let cfg = WorldConfig {
        tick: TickConfig::default(),
        seed,
        size: Vec2::new(2000.0, 2000.0),
    };
    let mut w = World::new(g.defs.clone(), g.ruleset.clone(), cfg);
    let mut spawn = |team, x| {
        w.spawn_unit(SpawnUnit {
            unit: g.blob,
            team: TeamId(team),
            pos: Vec2::new(x, 1000.0),
            level: 1,
            perks: vec![],
            ranks: None,
        })
    };
    let (a, b) = (spawn(0, 500.0), spawn(1, 1500.0));
    (w, a, b)
}

/// Both units walk at each other and poke whenever they can.
fn brawl_step(w: &mut World, a: EntityId, b: EntityId) {
    for (me, foe) in [(a, b), (b, a)] {
        if !w.is_alive(me) || !w.is_alive(foe) || w.is_winding_up(me) {
            continue;
        }
        let poke = Command::Use {
            slot: 0,
            target: Target::Entity(foe),
        };
        let cmd = match w.legal(me, &poke) {
            Ok(()) => poke,
            Err(Illegal::OutOfRange) => Command::MoveTo(w.ent(foe).unwrap().pos),
            Err(_) => Command::None,
        };
        w.submit(me, cmd).unwrap();
    }
    w.tick();
}

#[test]
fn a_fight_runs_to_a_kill_and_is_deterministic() {
    let g = game();
    let run = |seed| {
        let (mut w, a, b) = world(&g, seed);
        for _ in 0..2000 {
            brawl_step(&mut w, a, b);
        }
        (w.state_hash(), w.is_alive(a), w.is_alive(b))
    };
    let first = run(7);
    assert_eq!(first, run(7));
    // The fight is perfectly symmetric, so the final pokes land at the same
    // instant and both units fall: nobody wins by being processed first.
    assert_eq!((first.1, first.2), (false, false));
}

#[test]
fn snapshot_restore_continues_identically() {
    let g = game();
    let (mut w, a, b) = world(&g, 3);
    for _ in 0..150 {
        brawl_step(&mut w, a, b);
    }
    let snap = w.snapshot();
    for _ in 0..300 {
        brawl_step(&mut w, a, b);
    }
    let straight = w.state_hash();

    let (mut w2, _, _) = world(&g, 999);
    w2.restore(snap);
    for _ in 0..300 {
        brawl_step(&mut w2, a, b);
    }
    assert_eq!(straight, w2.state_hash());
}

#[test]
fn fast_projectiles_do_not_tunnel() {
    let g = game();
    let (mut w, a, b) = world(&g, 1);
    // 20,000 units/s is ~667 units per tick, far more than the target is wide.
    w.submit(
        a,
        Command::Use {
            slot: 1,
            target: Target::Point(Vec2::new(1500.0, 1000.0)),
        },
    )
    .unwrap();
    for _ in 0..5 {
        w.tick();
    }
    let life = w.resource(b, ResourceId(0));
    assert_eq!(life, 75.0);
    let hit = g.defs.reg.ev.projectile_hit;
    assert_eq!(w.drain_events().iter().filter(|e| e.kind == hit).count(), 1);
}

#[test]
fn actions_respect_windup_cooldown_and_range() {
    let g = game();
    let (mut w, a, b) = world(&g, 1);
    let poke = Command::Use {
        slot: 0,
        target: Target::Entity(b),
    };
    assert_eq!(w.legal(a, &poke), Err(Illegal::OutOfRange));
    w.ent_mut(a).unwrap().pos = Vec2::new(1420.0, 1000.0);
    w.submit(a, poke).unwrap();
    assert!(w.is_winding_up(a));
    assert_eq!(w.legal(a, &poke), Err(Illegal::OnCooldown));
    // 0.2 s wind-up is 6 ticks of 33.3 ms once snapped up to a tick boundary.
    for _ in 0..5 {
        w.tick();
    }
    assert_eq!(w.resource(b, ResourceId(0)), 100.0);
    w.tick();
    w.tick();
    assert_eq!(w.resource(b, ResourceId(0)), 85.0);
    assert!(!w.is_winding_up(a));
}

#[test]
fn buffs_stack_modify_stats_set_flags_and_expire() {
    let g = game();
    let (mut w, a, b) = world(&g, 1);
    let speed = g.defs.reg.find_stat("speed").unwrap();
    for _ in 0..5 {
        w.apply_buff(a, g.pile, a, BuffOpts::default());
    }
    assert_eq!(w.ent(a).unwrap().buff(g.pile).unwrap().stacks, 3);
    assert_eq!(w.stat(a, speed), 330.0);

    w.apply_buff(b, g.chill, a, BuffOpts::default());
    assert_eq!(
        w.legal(b, &Command::MoveTo(Vec2::ZERO)),
        Err(Illegal::CannotMove)
    );
    for _ in 0..31 {
        w.tick();
    }
    assert_eq!(w.legal(b, &Command::MoveTo(Vec2::ZERO)), Ok(()));
    for _ in 0..31 {
        w.tick();
    }
    assert_eq!(w.stat(a, speed), 300.0);
}

#[test]
fn regions_report_entering_and_leaving_once() {
    let g = game();
    let (mut w, a, _) = world(&g, 1);
    let zone = w.spawn_region("mid", TeamId(9), Vec2::new(1000.0, 1000.0), 100.0);
    w.submit(a, Command::MoveTo(Vec2::new(1400.0, 1000.0)))
        .unwrap();
    for _ in 0..120 {
        w.tick();
    }
    let ev = g.defs.reg.ev;
    let events = w.drain_events();
    let count = |k| {
        events
            .iter()
            .filter(|e| e.kind == k && e.target == Some(zone))
            .count()
    };
    assert_eq!((count(ev.region_entered), count(ev.region_exited)), (1, 1));
}

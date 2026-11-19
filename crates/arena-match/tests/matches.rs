//! End-to-end checks on the shipped scenarios: every layer working together.

use arena_agents::obs::{decode, observe, AgentAction, ObsSpec};
use arena_kernel::{Command, Rng};
use arena_match::{run_batch, Content, LoadoutConfig, Match, PolicyConfig, Scenario};
use std::path::Path;

fn load(name: &str) -> (Content, Scenario) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scenarios")
        .join(name);
    let scenario = Scenario::load(&path).unwrap();
    let content = Content::load(&scenario.pack_dir(), arena_ruleset_lol::build).unwrap();
    (content, scenario)
}

const SCENARIOS: [&str; 5] = [
    "1v1_garen_vs_ashe.yaml",
    "1v1_annie_mirror.yaml",
    "3v3_points.yaml",
    "2v2_nexus.yaml",
    "ffa_three_teams.yaml",
];

#[test]
fn every_scenario_finishes_and_is_deterministic() {
    for name in SCENARIOS {
        let (content, scenario) = load(name);
        let run = |seed| Match::new(&content, &scenario, seed).unwrap().run();
        let (a, b, c) = (run(11), run(11), run(12));
        assert_eq!(
            a.state_hash, b.state_hash,
            "{name}: same seed must give the same match"
        );
        assert_ne!(
            a.state_hash, c.state_hash,
            "{name}: another seed should give another match"
        );
        let placed: usize = a.placements.iter().map(Vec::len).sum();
        assert_eq!(placed, scenario.teams.len(), "{name}: every team is placed");
        assert!(
            a.duration_s <= scenario.timeout_s + 0.1,
            "{name}: ran past its timeout"
        );
    }
}

#[test]
fn a_replay_reproduces_the_match_exactly() {
    for name in ["1v1_garen_vs_ashe.yaml", "3v3_points.yaml"] {
        let (content, scenario) = load(name);
        let mut original = Match::new(&content, &scenario, 5).unwrap();
        let result = original.run();
        let replay = original.replay().clone();
        assert!(!replay.commands.is_empty());

        let mut again = Match::from_replay(&content, &scenario, &replay).unwrap();
        let replayed = again.run_replay(&replay);
        assert_eq!(result.state_hash, replayed.state_hash, "{name}");
        assert_eq!(result.winner, replayed.winner, "{name}");
    }
}

#[test]
fn modes_end_matches_for_their_own_reason() {
    let reason = |name: &str| {
        let (content, scenario) = load(name);
        Match::new(&content, &scenario, 2).unwrap().run().reason
    };
    assert_eq!(reason("1v1_garen_vs_ashe.yaml"), "elimination");
    assert_eq!(reason("3v3_points.yaml"), "points");
    assert_eq!(reason("2v2_nexus.yaml"), "nexus destroyed");
}

#[test]
fn a_mirror_match_is_fair() {
    let (content, scenario) = load("1v1_annie_mirror.yaml");
    let batch = run_batch(&content, &scenario, 1, 300).unwrap();
    let (blue, red) = (&batch.teams["blue"], &batch.teams["red"]);
    assert_eq!(blue.wins + red.wins + batch.draws, 300);
    // Neither side may be favoured: their win-rate intervals must overlap.
    assert!(blue.win_rate_low <= red.win_rate_high && red.win_rate_low <= blue.win_rate_high);
    assert!(
        blue.wins > 30 && red.wins > 30,
        "blue {} red {}",
        blue.wins,
        red.wins
    );
}

#[test]
fn batches_are_reproducible() {
    let (content, scenario) = load("ffa_three_teams.yaml");
    let wins = |b: arena_match::BatchResult| b.teams.values().map(|t| t.wins).collect::<Vec<_>>();
    let a = wins(run_batch(&content, &scenario, 40, 60).unwrap());
    assert_eq!(a, wins(run_batch(&content, &scenario, 40, 60).unwrap()));
    assert!(a.iter().sum::<u32>() <= 60);
}

#[test]
fn staging_rejects_illegal_loadouts() {
    let (content, mut scenario) = load("1v1_garen_vs_ashe.yaml");
    let mut error_for = |gold: f64, perks: Vec<&str>| {
        let member = &mut scenario.teams[0].members[0];
        member.gold = gold;
        member.loadout = LoadoutConfig::Fixed(perks.into_iter().map(String::from).collect());
        Match::new(&content, &scenario, 1).err().unwrap()
    };
    let err = error_for(3000.0, vec!["infinity_edge", "black_cleaver"]);
    assert!(err.contains("budget"), "{err}");
    let err = error_for(100_000.0, vec!["berserkers_greaves"; 2]);
    assert!(err.contains("cannot be combined"), "{err}");
    let err = error_for(100_000.0, vec!["long_sword"; 7]);
    assert!(err.contains("at most 6"), "{err}");
    let err = error_for(100_000.0, vec!["no_such_item"]);
    assert!(err.contains("unknown perk"), "{err}");
}

#[test]
fn policies_spend_their_budget_at_staging() {
    let (content, scenario) = load("3v3_points.yaml");
    let m = Match::new(&content, &scenario, 1).unwrap();
    for (loadout, member) in m
        .replay()
        .loadouts
        .iter()
        .zip(scenario.teams.iter().flat_map(|t| &t.members))
    {
        let cost: f64 = loadout
            .iter()
            .map(|p| content.defs.perk(content.defs.perk_id(p).unwrap()).cost)
            .sum();
        assert!(
            !loadout.is_empty() && cost <= member.gold,
            "{}: {loadout:?} costs {cost}",
            member.unit
        );
    }
}

#[test]
fn random_agents_never_break_the_world() {
    let (content, mut scenario) = load("3v3_points.yaml");
    for member in scenario.teams.iter_mut().flat_map(|t| &mut t.members) {
        member.policy = PolicyConfig::Random;
    }
    scenario.timeout_s = 40.0;
    for seed in 0..5 {
        let result = Match::new(&content, &scenario, seed).unwrap().run();
        assert!(result.ticks > 0);
    }
}

#[test]
fn an_outside_driver_can_play_through_observations() {
    let (content, scenario) = load("3v3_points.yaml");
    let mut m = Match::new(&content, &scenario, 9).unwrap();
    let spec = ObsSpec::default();
    let width = spec.feature_count(&m.world);
    let mut rng = Rng::new(1);
    let mut accepted = 0;
    for _ in 0..600 {
        let mut commands = Vec::new();
        for agent in m.agents() {
            let obs = observe(&m.world, agent, &spec);
            assert_eq!(obs.features.len(), width);
            assert!(obs.features.iter().all(|f| f.is_finite()));
            let slot = rng.below(spec.max_slots as u64) as usize;
            let (dx, dy) = (rng.next_f64() * 2.0 - 1.0, rng.next_f64() * 2.0 - 1.0);
            let action = if obs.slot_mask[slot] {
                AgentAction::Use {
                    slot,
                    target: rng.below(3) as usize,
                    dx,
                    dy,
                }
            } else if obs.can_move {
                AgentAction::Move { dx, dy }
            } else {
                AgentAction::Noop
            };
            let cmd = decode(&m.world, agent, &obs, action);
            accepted += (cmd != Command::None && m.world.legal(agent, &cmd).is_ok()) as u32;
            commands.push((agent, cmd));
        }
        m.step_with(&commands);
    }
    assert!(accepted > 1000, "only {accepted} commands were legal");
}

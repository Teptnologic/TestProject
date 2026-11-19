//! `arena`: validate content, run a match, or run many for win rates.

use arena_match::{run_batch, Content, Match, RulesetFactory, Scenario};
use clap::{Parser, Subcommand};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "arena", version, about = "Combat sandbox simulator")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Load a content pack and report any problem in it.
    Validate { pack: PathBuf },
    /// Play one match and print what happened.
    Run {
        scenario: PathBuf,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        /// Write every event as one JSON object per line.
        #[arg(long)]
        events: Option<PathBuf>,
        /// Print the full result as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Play many matches with consecutive seeds and print win rates.
    Batch {
        scenario: PathBuf,
        #[arg(long, default_value_t = 200)]
        runs: u32,
        #[arg(long, default_value_t = 1)]
        seed: u64,
        #[arg(long)]
        json: bool,
    },
}

/// Rulesets this build knows, by the id a pack names in `pack.yaml`.
fn ruleset(id: &str) -> Result<RulesetFactory, String> {
    match id {
        arena_ruleset_lol::ID => Ok(arena_ruleset_lol::build),
        other => Err(format!("this build has no ruleset called `{other}`")),
    }
}

fn load_content(pack: &Path) -> Result<Content, String> {
    let id = arena_content::pack_ruleset(pack).map_err(|e| e.to_string())?;
    Content::load(pack, ruleset(&id)?)
}

fn load(scenario: &Path) -> Result<(Content, Scenario), String> {
    let scenario = Scenario::load(scenario)?;
    Ok((load_content(&scenario.pack_dir())?, scenario))
}

fn json<T: serde::Serialize>(value: &T) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).map_err(|e| e.to_string())?
    );
    Ok(())
}

fn validate(pack: &Path) -> Result<(), String> {
    let c = load_content(pack)?;
    println!(
        "{}: ok ({} units, {} actions, {} buffs, {} perks) for ruleset {} {}",
        c.pack.id,
        c.defs.units.len(),
        c.defs.actions.len(),
        c.defs.buffs.len(),
        c.defs.perks.len(),
        c.ruleset.id(),
        c.ruleset.version()
    );
    Ok(())
}

fn run(path: &Path, seed: u64, events: Option<&Path>, as_json: bool) -> Result<(), String> {
    let (content, scenario) = load(path)?;
    let mut m = Match::new(&content, &scenario, seed)?;
    let mut log = match events {
        Some(p) => Some(std::fs::File::create(p).map_err(|e| format!("{}: {e}", p.display()))?),
        None => None,
    };
    while m.outcome().is_none() {
        m.step();
        let Some(file) = &mut log else { continue };
        for ev in &m.last_events {
            let line = serde_json::json!({
                "t": ev.time.secs(),
                "event": content.defs.reg.events.name(ev.kind.0),
                "source": ev.source.map(|e| m.label(e)),
                "target": ev.target.map(|e| m.label(e)),
                "value": ev.value,
                "action": ev.action.map(|a| content.defs.action(a).name.clone()),
            });
            writeln!(file, "{line}").map_err(|e| e.to_string())?;
        }
    }
    let result = m.result();
    if as_json {
        return json(&result);
    }
    println!("{} (seed {})", result.scenario, result.seed);
    match &result.winner {
        Some(w) => println!(
            "winner: {w} by {} after {:.1} s",
            result.reason, result.duration_s
        ),
        None => println!(
            "no single winner ({}) after {:.1} s",
            result.reason, result.duration_s
        ),
    }
    println!(
        "{:<8} {:<10} {:>6} {:>5} {:>6} {:>12} {:>12}",
        "team", "unit", "health", "kills", "deaths", "damage dealt", "damage taken"
    );
    for u in &result.units {
        let total = |m: &std::collections::BTreeMap<String, f64>| {
            m.get("damage_dealt").copied().unwrap_or(0.0)
        };
        println!(
            "{:<8} {:<10} {:>5.0}% {:>5} {:>6} {:>12.0} {:>12.0}",
            u.team,
            u.unit,
            u.vitality * 100.0,
            u.kills,
            u.deaths,
            total(&u.dealt),
            total(&u.taken)
        );
    }
    if !result.scores.values().all(|s| *s == 0.0) {
        let scores: Vec<String> = result
            .scores
            .iter()
            .map(|(t, s)| format!("{t} {s:.1}"))
            .collect();
        println!("scores: {}", scores.join(", "));
    }
    Ok(())
}

fn batch(path: &Path, runs: u32, seed: u64, as_json: bool) -> Result<(), String> {
    let (content, scenario) = load(path)?;
    let started = std::time::Instant::now();
    let result = run_batch(&content, &scenario, seed, runs)?;
    if as_json {
        return json(&result);
    }
    println!(
        "{}: {} matches (seeds {}..{})",
        result.scenario,
        runs,
        seed,
        seed + runs as u64
    );
    println!(
        "{:<8} {:>5} {:>9} {:>17} {:>16}",
        "team", "wins", "win rate", "95% interval", "avg damage dealt"
    );
    for (name, t) in &result.teams {
        println!(
            "{:<8} {:>5} {:>8.1}% {:>7.1}% – {:>5.1}% {:>16.0}",
            name,
            t.wins,
            t.win_rate * 100.0,
            t.win_rate_low * 100.0,
            t.win_rate_high * 100.0,
            t.avg_dealt.get("damage_dealt").copied().unwrap_or(0.0)
        );
    }
    println!(
        "draws: {}, average match {:.1} s, simulated in {:.2} s",
        result.draws,
        result.avg_duration_s,
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

fn main() -> ExitCode {
    let outcome = match Cli::parse().command {
        Cmd::Validate { pack } => validate(&pack),
        Cmd::Run {
            scenario,
            seed,
            events,
            json,
        } => run(&scenario, seed, events.as_deref(), json),
        Cmd::Batch {
            scenario,
            runs,
            seed,
            json,
        } => batch(&scenario, runs, seed, json),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

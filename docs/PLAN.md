# Arena — build plan

A deterministic, real-time combat sandbox engine. League of Legends is the first
content pack; other games and custom units plug in later. `arena` is a working
name used for crate prefixes — renaming is a search-and-replace.

This plan is written to be executed top to bottom. Each milestone lists its
tasks, the files it creates, and a "done when" check that can be run.

---

## 0. Status

The MVP is built: milestones M0–M7 exist in a first, working form, and
[ARCHITECTURE.md](ARCHITECTURE.md) describes the layers and their contracts.
M8 (Python), M9 (WASM and viewer) and M10 (Luau) are not started.

What the MVP does differently from the milestone text below:

| Planned | Built | Why |
|---|---|---|
| `hecs` for world storage | Entities are plain records in an id-ordered map | Snapshots are a clone, iteration order is fixed, and nothing needs a component registry. Revisit if entity kinds multiply. |
| `rand_chacha` | A small built-in generator (SplitMix64) | Serialisable, identical everywhere, no dependency |
| Pure event queue | Timers on a queue; movement and projectiles advance once per tick | Equivalent while times snap to ticks, and far simpler. Projectiles still sweep their step. |
| `arena-match` not in the layout | Added: scenario, staging, match loop, replay, batch | The CLI should stay a thin shell |
| Respawn inside each mode | A separate `Respawner` that modes can veto | Lets modes be combined without double respawns |

Known gaps inside M0–M7, in rough priority order:

- **Map**: an open rectangle only. No obstacles, pathfinding, brush or vision.
- **Ingest**: writes stats, growth, costs, cooldowns, ranges and item constants.
  Per-rank ability damage numbers are still typed by hand in `authored/`.
- **Content**: Garen, Ashe and Annie (each with stated simplifications), a
  nexus, 13 items and Ignite. No runes yet.
- **Ruleset**: no lifesaving effects, stasis, assists, conditional modifiers
  evaluated on read, wind-up modifier, or multiplicative tenacity.
- **Scripted AI**: nearest-target, rule list, optional kiting. No skillshot
  leading or dodging, no focus fire, no retreating.
- **Kernel**: no spatial index (fine at this scale), no attachment effect in
  data, team relations are simply "different team = hostile".
- **Snapshots**: the world serialises; mode and policy state only clone.
- **Golden numbers**: formulas are tested against the wiki's stated formulas,
  not yet against timings measured in the Practice Tool.

---

## 1. Decisions already made

| Topic | Decision |
|---|---|
| Purpose | Matchup analysis and AI-vs-AI training in a sandbox with X teams × Y members |
| Scope | Generic engine; LoL is content pack #1 |
| Data vs code | Hybrid: data composes engine building blocks; rare escape-hatch scripts |
| Time | Discrete-event, snapped to server ticks by default (33,333 µs) |
| Space | 2D |
| Progression | None. Level is fixed by the scenario |
| Staging | Units fixed by scenario and visible to all; each unit spends a gold budget on items; teams buy simultaneously |
| Builds | Per team: `fixed` (given in the scenario) or `policy` (an agent buys) |
| Win conditions | Elimination, first to X points, destroy the nexus; combinable; richer modes (e.g. football) later |
| Core language | Rust |
| World storage | Plain entity records in an id-ordered map (see Status) |
| Determinism | `f64` with all non-trivial maths through `libm`; integer time; seeded RNG |
| Scripting | Luau via `mlua`, for escape hatches and scripted game modes |
| Training | Python bindings (PyO3 + maturin), vectorised multi-agent environment |
| Browser | WASM build (`wasm-bindgen`) driving the existing React app as viewer and staging UI |
| Data ingestion | Stays in Node (`scripts/fetch-data.js`); the boundary is the canonical data files |

## 2. Architecture

```
Agents / Policies   scripted AI, trained models — they only issue commands
Game mode           win conditions, scoring, respawn, mode-specific entities and phases
Scenario            map, teams, members, level, gold, mode, seed        (data)
Content pack        units, abilities, items, runes                       (data + Luau)
Ruleset             stat formulas, damage pipeline, CC table, loadout rules (Rust)
Kernel              time, entities, space, stats, buffs, events, RNG, snapshots (Rust)
```

Dependency rule: each layer may only depend on the layers below it. The kernel
never mentions armor, mana, stuns, items or champions.

## 3. Repository layout

The current React app stays at the repo root, untouched, until milestone 9, so
the Vercel deploy keeps working.

```
Cargo.toml                 workspace
crates/
  arena-kernel/            time, world, space, stats, buffs, events, actions, snapshots
  arena-formula/           formula parser + evaluator
  arena-content/           canonical schema, loader, validation, effect catalogue
  arena-ruleset-lol/       the LoL ruleset
  arena-modes/             GameMode trait, elimination / points / nexus, respawn
  arena-agents/            Policy trait, observation + action spec, scripted policy
  arena-script/            Luau host (milestone 10)
  arena-match/             scenario, staging, match loop, replay, batch
  arena-cli/               `arena` binary: validate, run, batch
  arena-py/                Python bindings (milestone 8)
  arena-wasm/              browser bindings (milestone 9)
content/lol/
  pack.yaml                pack id, ruleset id + version it targets
  generated/               numbers layer, written by tools/ingest (never hand-edited)
  authored/                behaviour layer: units/, items/, runes/, policies/
maps/                      map files
scenarios/                 scenario files
tools/ingest/              Node: Riot data -> content/lol/generated
python/arena_gym/          PettingZoo wrapper, training scripts
tests/golden/              golden scenarios with expected state hashes and numbers
docs/                      this plan, schema reference, building-block catalogue
```

## 4. Engineering rules (apply to every milestone)

1. **Determinism.** Same scenario + seed + commands gives an identical state
   hash. To keep that true:
   - Time is `SimTime(u64)` microseconds. No float time.
   - Never call `f64::sin`, `cos`, `atan2`, `powf`, `exp`, `ln`; use the
     `arena_kernel::math` wrappers over `libm`. A clippy `disallowed_methods`
     list enforces this.
   - Never iterate a `HashMap` where order affects
     results. Sort by `EntityId`, or use `BTreeMap` / `IndexMap`.
   - One seeded RNG owned by the world, serialised in
     snapshots. No `thread_rng`.
2. **Everything is snapshot-able.** Every component and every piece of mode,
   ruleset and policy state implements `Serialize + Deserialize + Clone`.
3. **Fail at load, not mid-fight.** Unknown stat names, bad formulas, missing
   references and schema violations are load-time errors with file and field.
4. **Tests.** Unit tests beside the code; golden tests in `tests/golden/`. CI
   runs `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`.
5. **Versioning.** The kernel building-block catalogue and each ruleset carry a
   semantic version; `pack.yaml` declares what it targets; mismatch fails load.

---

## 5. Milestones

Each milestone ends with a runnable check. Do not start the next one until the
check passes.

### M0 — Workspace bootstrap

Tasks
1. Add root `Cargo.toml` workspace, `rust-toolchain.toml` (pin stable),
   `clippy.toml` with the disallowed-methods list, and `/target` in `.gitignore`.
2. Create empty `arena-kernel` and `arena-cli` crates.
3. Add `.github/workflows/rust.yml`: fmt, clippy, test.

Done when: `cargo test --workspace` and `cargo run -p arena-cli -- --version`
pass locally and in CI, and `npm run build` still works.

### M1 — Kernel: time, events, world, snapshots

Files: `arena-kernel/src/{time,queue,world,rng,snapshot,hash}.rs`

Tasks
1. `SimTime(u64)`, `Duration`, and `TickConfig { tick_us, snap: bool }` with a
   `snap_up(t)` helper.
2. Event queue: binary heap keyed by `(time, phase, seq)`. `phase` is a small
   enum fixing same-time order: `Control → Movement → Effects → Damage → Death →
   Mode → Decision`. `seq` is a monotonically increasing counter.
3. `World`: wraps `hecs::World`, the queue, the RNG, the clock, and a stable
   `EntityId(u32)` allocator (do not expose `hecs::Entity` ids).
4. `World::step_until(t)` and `World::step_event()`.
5. Snapshots: `World::snapshot() -> Snapshot` and `World::restore(&Snapshot)`
   via a component registry (each component type registers serialise / clone
   functions). Format: `bincode`.
6. `World::state_hash() -> u64`: hash of the canonical snapshot bytes.

Done when
- Property test: 1,000 random event schedules, each run twice with the same
  seed, give equal state hashes.
- Snapshot at a random point, restore into a fresh world, continue: final hash
  equals the uninterrupted run.

### M2 — Kernel: space and movement

Files: `arena-kernel/src/{math,space,movement,projectile,region,attach}.rs`

Tasks
1. `Vec2` (`f64`) and `math` wrappers over `libm`.
2. Components: `Position`, `Radius`, `Velocity`, `Team(TeamId)`, `Mover`
   (current path, speed source).
3. Map v1: rectangular bounds plus static circle and convex-polygon obstacles.
   Movement is straight-line with sliding along obstacles. Navmesh pathfinding
   is deferred (backlog).
4. Uniform-grid spatial index with queries: `in_circle`, `in_cone`,
   `in_capsule`, `nearest`. Results always sorted by `EntityId`.
5. Displacements: `dash`, `blink`, `knockback`, `pull`, forced-walk.
6. Projectiles as entities: linear and homing, with speed, width, max range,
   pierce count, and swept circle-vs-capsule collision (exact, not per-step).
7. Regions (circle / polygon) with `RegionEntered` / `RegionExited` events.
8. Attachment: `AttachedTo(EntityId)` (carrier relation), with detach rules.

Done when
- Unit tests: a projectile at any speed never tunnels through a target; a
  homing projectile reaches a moving target at the analytically expected time
  (± one tick); region enter/exit fire exactly once per crossing.
- Determinism test from M1 still passes with 50 moving units and projectiles.

### M3 — Kernel: stats, buffs, events, actions, formulas

Files: `arena-kernel/src/{stats,modifier,buff,bus,action,command,resource,targeting}.rs`,
`arena-formula/`

Tasks
1. Stat registry: rulesets register named stats (`StatId`) with a combine rule.
   Modifier layers: `base`, `flat`, `percent_add`, `percent_mult`, `override`,
   plus an optional condition evaluated on read. Dirty-flag caching.
2. `arena-formula`: parse strings such as
   `[60,85,110][rank] + 0.5*ap + 0.02*target.max_hp` into a typed tree; resolve
   stat names at load; evaluate against a `(source, target, context)` scope.
3. Buffs: id, source, owner, duration, stacks, stacking policy (`refresh`,
   `extend`, `add_stack`, `independent`, `replace_if_stronger`), tags, and
   effects (stat modifiers, flags, periodic ticks, listeners, `on_expire`).
4. Event bus: typed events, listener priority, per-listener internal cooldown,
   and a recursion guard keyed on `(listener, source event chain)`.
5. Flags: generic named boolean flags on entities (`untargetable`,
   `cannot_move`, …) set by buffs; the ruleset decides what they mean.
6. Action state machine: an entity runs at most one *locking* action; an action
   is a sequence of timed stages with lock flags, cancel rules, and a buffer
   slot for one queued command.
7. Commands: `MoveTo`, `Stop`, `UseAction { action_id, target }`. The kernel
   asks the ruleset `can(entity, command) -> Legality`. `legal_mask(entity)`
   returns the mask used by agents.
8. Target selectors (`self`, `target`, `nearest_enemy`, `allies_in_radius`, …)
   and affiliation filters (enemy / ally / self / neutral / any).
9. Generic named resources with max, regen and decay.
10. Effect catalogue v1 (the building blocks data may use): `modify_resource`,
    `apply_buff`, `remove_buff`, `spawn_projectile`, `spawn_entity`,
    `spawn_region`, `displace`, `attach`, `detach`, `emit_event`, `if`,
    `for_each_target`, `set_cooldown`, `add_score`. Rulesets add more.

Done when
- Unit tests cover each stacking policy, modifier layer order, conditional
  modifiers, listener priority, internal cooldowns and the recursion guard.
- A toy ruleset in `arena-kernel/tests/toy_ruleset.rs` (two stats, one damage
  rule) runs a 1v1 to completion deterministically. This proves the kernel has
  no LoL knowledge.

### M4 — LoL ruleset

Files: `arena-ruleset-lol/src/{stats,attack,ability,damage,heal,cc,movement,resource,loadout,targetability}.rs`

Tasks
1. `Ruleset` trait (defined in the kernel): `register(&mut Registry)`,
   `can(...)`, `on_spawn(...)`, `validate_loadout(...)`, plus effect handlers.
2. Stats and formulas: level growth curve, attack speed (base + ratio × bonus,
   cap), ability haste → cooldown, move speed with slows and soft caps,
   tenacity and slow resist.
3. Basic attacks: wind-up from per-unit wind-up % and modifier, wind-down,
   melee vs projectile, attack resets, cancel rules.
4. Abilities: ranks, costs, cooldowns (start-on-cast and start-on-end), charges,
   recasts, cast time, channels, cast-while-moving.
5. Damage pipeline with fixed named stages: raw → source amplifiers → crit →
   resistances (flat reduction, % reduction, % pen, flat pen) → target
   reduction → shields → HP → post-damage (vamp, triggers). Adds effects
   `damage`, `heal`, `shield`.
6. CC table: each CC type as a flag set plus `tenacity_applies` and
   `cleansable`. Adds effect `apply_cc`.
7. On-hit / on-attack hooks with on-hit ratio; proc tags.
8. Targetability: untargetable, invulnerable, stasis, spell shield.
9. Lifesaving hook at the HP stage (lethal-damage interception).
10. Kill credit and assists.
11. Loadout rules: 6 slots, uniqueness groups, consumable limits, gold cost.

Done when: golden unit tests match known LoL numbers, each with its source in a
comment — at minimum: stat at level N for 3 champions; attack timings at 1.0
and 2.0 attack speed; damage vs positive and negative armor with mixed
penetration; move speed across all three soft-cap bands with a slow; tenacity
stacking; shield ordering; grievous wounds on healing.

### M5 — Content schema, loader, first LoL content

Files: `arena-content/`, `content/lol/`, `tools/ingest/`, `docs/schema.md`

Tasks
1. Canonical schema as Rust types with `serde` + `schemars`; emit JSON Schema
   to `docs/schema/` for editor validation. Files are YAML.
2. Loader: read pack → merge `generated/` numbers into `authored/` behaviour by
   id → resolve references → compile formulas → validate against the ruleset's
   registries.
3. `arena validate content/lol` CLI command.
4. `tools/ingest/`: Node script that reads the existing
   `src/data/generated/*.json` and writes `content/lol/generated/` (base stats,
   per-rank ability values, item stats and costs). Extend the weekly
   `refresh-data` workflow to run it.
5. Author the first content, chosen to exercise different systems:
   - Units: **Garen** (melee, no resource, silence, damage reduction, execute),
     **Ashe** (ranged, crit rule change, slows, skillshot stun),
     **Annie** (mage, mana, stacking passive → stun, shield, summoned pet),
     **Vayne** (attack reset, % max-HP true damage, knockback, stealth later).
   - Items: ~12 covering plain stats, on-hit, spellblade, lifesteal, shield
     lifeline, stasis active, grievous wounds, armor pen, ability haste.
   - Runes: 3–4 keystones (one burst proc, one stacking, one on-hit).
   - Summoner spells: Flash, Ignite, Exhaust, Barrier.
6. Minimal scripted policy (hard-coded: walk into range, attack, cast when
   ready) so fights can run before M7.

Done when
- `arena validate content/lol` passes; deliberately broken fixtures fail with
  file + field in the message.
- Golden fights (`tests/golden/1v1_*.yaml`) for each pair of the four units run
  to completion with pinned state hashes.
- Changing a number in `generated/` changes outcomes with no code changes.

### M6 — Scenarios, staging, game modes

Files: `arena-modes/`, `maps/`, `scenarios/`, `arena-cli/src/run.rs`

Tasks
1. Scenario schema: map, tick config, teams (relations, friendly fire),
   members (unit, level, spawn, gold, loadout `fixed | policy`, policy id),
   mode + parameters, timeout, seed.
2. Match lifecycle: `Load → Staging → Combat → Result`. Staging collects one
   loadout per unit, validates through the ruleset, rejects over-budget or
   illegal loadouts.
3. `GameMode` trait: `setup(world)`, `on_event(event)`, `status() ->
   Ongoing | Finished(placements)`, `observe(team)`, `reward(team)`, and
   serialisable state. Modes may spawn entities and regions, add flags, add
   actions, and reset positions mid-match.
4. Modes:
   - **Elimination**: lives per unit (default 1); last team standing.
   - **Points**: data-defined point rules (`on_kill`, `hold_region` per second,
     `on_objective`); first to X.
   - **Nexus**: structure entities with HP and optional invulnerability
     conditions; a team is out when its nexus dies.
   - Combinator: any-of, plus timeout with tiebreak (score, then total HP %).
5. Respawn: timer rule, spawn region, loadout preserved.
6. CLI: `arena run scenarios/x.yaml --seed 1 --log events.jsonl` and
   `arena batch scenarios/x.yaml --runs 1000 --jobs 8` (parallel with `rayon`),
   printing win rates with confidence intervals and per-unit stats (damage
   dealt / taken / mitigated, healing, CC time, kills, deaths).
7. Replay file: scenario + seed + loadouts + command stream. Replaying it
   reproduces the match exactly.

Done when
- One golden scenario per mode, plus a 3-team elimination and a combined mode,
  all with pinned hashes.
- `arena batch` on a mirror match reports a win rate within its confidence
  interval of 50%.
- A recorded replay re-runs to the same final hash.

### M7 — Agent interface and scripted AI

Files: `arena-agents/`, `content/lol/authored/policies/`

Tasks
1. `Policy` trait: `stage(obs) -> Loadout`, `act(obs, mask) -> Command`.
2. Observation spec (content-agnostic): self features, a padded list of visible
   entities (relative position, team relation, normalised stats declared by the
   ruleset, flags, current action), per-action-slot features (ready, cooldown,
   range, targeting kind, id), mode features, and the legality mask.
3. Action spec: `(kind, action slot, target entity index, target point)` with
   the point encoded as a discretised offset; mask per head.
4. Decision cadence: every `decision_ticks` ticks (default 4), plus an optional
   per-agent observation delay to model reaction time.
5. Data-driven scripted policy: per-unit priority lists in YAML
   (`when: target_in_range(Q) and ready(Q)` → `cast: Q`), plus generic
   behaviours: target selection, kiting, retreat thresholds, skillshot aim with
   lead and configurable error, sidestep on incoming projectile.
6. Optional per-team commander hook (focus target assignment).

Done when
- 1v1, 3v3 and 5v5 scenarios run with scripted policies; `arena batch` produces
  a matchup table for all pairs of the four units.
- Throughput benchmark recorded in `docs/benchmarks.md` (simulated seconds per
  wall-clock second for 1v1 and 5v5) as the baseline for M8.

**This is the first usable product: matchup analysis from the command line.**

### M8 — Python bindings and training

Files: `arena-py/`, `python/arena_gym/`, `python/train/`

Tasks
1. `arena-py` with PyO3 + maturin: `VecEnv(scenario, num_envs, seed)` whose
   `reset()` and `step(actions)` take and return NumPy arrays for all
   environments at once; environments step in parallel inside Rust.
2. `arena_gym`: PettingZoo parallel-API wrapper around a single environment,
   plus the vectorised interface for training.
3. Rewards: placement reward from the mode, plus configurable shaping terms
   (damage dealt / taken, score delta, objective damage).
4. Snapshot / restore exposed to Python (for curricula and search).
5. Reference training script: PPO with action masking and self-play against a
   pool of past checkpoints; fixed builds first.
6. Evaluation script: trained policy vs scripted policy over N seeds; Elo table
   across checkpoints.
7. Build policy: staging exposed as a one-step decision before combat, so the
   same agent can learn to buy (`loadout: policy`).

Done when
- `pip install -e python/` then `python -m arena_gym.smoke` runs 10k random
  steps with no mask violations.
- A PPO agent trained on 1v1 Garen mirror beats the scripted policy in more
  than 60% of 1,000 evaluation matches.
- Environment steps per second recorded in `docs/benchmarks.md`.

### M9 — WASM build and web viewer

Files: `arena-wasm/`, `web/`

Tasks
1. `arena-wasm`: load pack + scenario, step, read render state, load replay.
2. Cross-platform determinism test in CI: run the golden scenarios under
   `wasm32` (via `wasmtime` or Node) and compare state hashes with native.
3. Move the React app into `web/` and update the Vercel root directory setting.
   Retire the old calculator utilities (`src/utils/damage.js` and related) once
   the viewer replaces them.
4. Viewer: 2D canvas with units, projectiles, regions, health bars, buff icons,
   timeline scrubber, play / pause / speed, event log panel.
5. Scenario builder and staging UI: pick map, teams, units, level, gold; buy
   items; run N simulations in a Web Worker; show win rates and stats.

Done when: a replay recorded by `arena run` natively plays in the browser and
ends on the same state hash; the staging UI can launch a batch and show results.

### M10 — Luau scripting host

Files: `arena-script/`, `content/lol/authored/scripts/`

Tasks
1. Embed Luau via `mlua` with sandboxing on; remove `os`, `io`, `math.random`
   and float maths that bypasses `libm`; expose the engine API (the same effect
   catalogue data uses, plus event subscription and per-script state).
2. Script state is serialised in snapshots.
3. Hooks: unit / ability / item scripts, and scripted game modes implementing
   the `GameMode` interface.
4. Proofs: one unit that data cannot express (e.g. a weapon-swapping kit), and
   the Points mode re-implemented in Luau with identical results to the Rust
   version.

Done when: both proofs pass golden tests natively and under WASM, and a script
error reports script file and line without crashing the engine.

---

## 6. Backlog (after M10, not ordered)

- Navmesh pathfinding, walls that block dashes, knockback-into-wall.
- Vision: brush, stealth, fog of war, per-team observations.
- Content breadth: more units, items and runes; an authoring guide.
- Self-play league and matchmaking for training at scale.
- Search-based agents using snapshots (e.g. MCTS).
- Football-style mode: carried ball, downs, yard lines, end zones. The kernel
  pieces it needs (regions, attachment, mid-match resets, extensible actions,
  mode state) are built in M2, M3 and M6.
- A second ruleset and content pack to prove the generic boundary.

## 7. Risks to watch

| Risk | Mitigation |
|---|---|
| LoL concepts leaking into the kernel | Toy-ruleset test in M3 must keep passing; kernel code review rule: no game nouns |
| Determinism regressions | State-hash golden tests in CI, native and WASM |
| Data schema churn breaking content | Schema versioning; `arena validate` in CI on every content change |
| Effect catalogue too weak, pushing everything into scripts | Track the share of units needing scripts; add a building block when a pattern recurs three times |
| Scripted AI too weak for meaningful matchup numbers | Tunable skill parameters; compare against trained agents from M8 |
| Training throughput too low | Benchmarks from M7; vectorised stepping in Rust; profile before optimising |
| Accuracy vs the real game | Every ruleset number has a golden test with a cited source |

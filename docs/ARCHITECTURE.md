# Arena — architecture

Arena is a deterministic combat sandbox. The engine is generic; League of
Legends is its first ruleset and content pack. This document describes the
layers and the contract between each pair of them. [PLAN.md](PLAN.md) covers
what is built and what comes next.

## Layers

```
arena-cli        validate / run / batch                               (binary)
arena-match      scenario → staging → match loop → result, replay, batch
arena-agents     Policy trait, scripted bot, observations for learned agents
arena-modes      GameMode trait, elimination / points / nexus, respawning
arena-content    YAML pack → kernel definitions, with full validation
arena-ruleset-lol  the LoL rules                              (one of many possible)
arena-kernel     time, entities, 2D space, stats, buffs, actions, effects, events
arena-formula    the formula language used in data files
```

A crate may only depend on crates below it. Two rules follow from that and are
checked by tests:

- **The kernel knows no game.** It has no notion of health, armor, mana, stuns,
  items or champions. `arena-kernel/tests/toy_ruleset.rs` runs fights under a
  made-up ruleset to prove it.
- **Modes, agents and the match runner know no ruleset.** They work with any
  ruleset and any content pack. Only `arena-cli` names `arena-ruleset-lol`.

## Contracts

Each boundary is one small, explicit interface.

### Ruleset ⇄ kernel — `arena_kernel::Ruleset`

A ruleset is built by a factory `fn(&mut Defs) -> Arc<dyn Ruleset>`. The
factory first **registers the game's vocabulary** in `Defs::reg`:

| Registered | Example (LoL) | Used by data as |
|---|---|---|
| stats | `max_hp`, `ad`, `armor` | formula variables, modifiers |
| resources | `hp` (capped by `max_hp`) | costs, formula variables |
| flags | `stunned`, `silenced` | buff `flags` |
| tags | `basic_attack`, `physical` | action, buff and effect `tags` |
| events | `damage_dealt`, `on_hit` | buff listeners |
| custom effects | `damage(amount, crit)`, `stun(duration)` | effect names |
| built-in buffs | `lol_stun`, `lol_shield` | applied by the ruleset itself |

The kernel then calls the ruleset whenever a decision depends on game meaning:

| Hook | Question it answers |
|---|---|
| `base_stats`, `default_ranks` | What does this unit look like at this level? |
| `validate_loadout` | Is this staging choice legal within the budget? |
| `init_unit` | What state does a freshly (re)spawned unit start in? |
| `move_speed`, `can_move`, `can_use`, `targetable` | What may this unit do right now? |
| `action_timing` | How long are this action's wind-up, recovery and cooldown? |
| `apply_custom` | What does one of *your* effects do to this target? |
| `on_event` | Do you want to react to this event before data listeners do? |
| `vitality` | How healthy is this unit, `0..=1`? (tiebreaks, observations) |

In return the ruleset drives the kernel through `World`'s public methods
(`apply_buff`, `set_resource`, `kill`, `interrupt`, `emit`, …).

### Content ⇄ kernel — `arena_kernel::Defs`

`arena-content` compiles a pack directory into immutable definitions:
`UnitDef`, `ActionDef`, `BuffDef`, `PerkDef` and lists of `Effect`. Effects are
the building blocks data composes:

`apply_buff`, `remove_buff`, `resource`, `projectile`, `area`, `dash`, `if`,
`emit`, `score`, `reset_cooldown`, plus every custom effect the ruleset registered.

Every name and formula is resolved while loading. A pack that loads cannot
fail later on a missing stat, a typo in a field or a bad formula; errors name
the file and the place (`garen.yaml: action garen_q > on_cast[0] apply_buff:
unknown buff …`). `pack.yaml` states the ruleset and version the pack targets;
a mismatch is refused.

A pack keeps **numbers** and **behaviour** apart:

```
generated/   numbers per id (stats, growth, costs, constants) — written by tools/ingest
authored/    behaviour per unit / perk, referring to numbers as @constants
```

A balance patch only rewrites `generated/`.

### Agents ⇄ kernel — `Command` in, read-only `World` out

`arena_agents::Policy` has two methods: `stage` (choose a loadout within a
budget) and `act` (return one `Command`: `None`, `Stop`, `MoveTo`, `Use`).
Policies receive `&World` and cannot change it. `World::legal` tells any agent
whether a command would be accepted; it is the action mask.

`arena_agents::obs` gives learned policies a fixed-size numeric observation and
decodes their output into a `Command`. The layout depends only on the ruleset's
vocabulary, never on which units are in the match.

### Modes ⇄ kernel — `arena_modes::GameMode`

A mode is told about the teams (`setup`), sees every event (`on_event`), runs
once per tick (`on_tick`) and reports `Status::Ongoing` or
`Status::Finished(Outcome)` with placements. It may spawn its own entities
(zones), keep scores and veto respawns (`may_respawn`). `standings` ranks teams
when the clock runs out. Respawning itself is a separate rule (`Respawner`).

### Everything ⇄ everything — `Event`

All layers observe the world through one event shape:
`{ kind, time, source, target, value, action }`. Data listeners, the ruleset,
modes, the stats report and the event log all read the same stream.

## One tick

`Match::step` does this, in order:

1. Each agent whose decision is due returns a command; legal ones are applied
   and recorded for the replay.
2. `World::tick`:
   1. timers due this tick fire in a fixed order (regeneration, buff ticks,
      action stages, buff expiry);
   2. units and projectiles move (projectiles sweep their whole step, so they
      cannot pass through a target);
   3. regions report who entered and left.
   Deaths are carried out only after everything at the same instant has
   resolved, so simultaneous blows both land.
3. The tick's events go to the stats, the mode and the respawner.
4. The mode is asked whether the match is over; otherwise the timeout is checked.

## Determinism

The same scenario, seed and commands always give the same match, verified by
`World::state_hash`:

- time is integer microseconds, snapped to server ticks;
- the world is plain data in id-ordered maps, so iteration order is fixed and a
  snapshot is a clone;
- one seeded random generator lives in the world state; agents get their own;
- maths beyond `+ − × ÷ √` goes through `libm` (`arena_kernel::math`), and
  `clippy.toml` forbids the platform versions.

A `Replay` (seed, loadouts, accepted commands) reproduces a match exactly.

## Adding things

| To add… | Touch |
|---|---|
| a unit, item, rune or spell | YAML under `content/<pack>/authored/` |
| a balance patch | `node tools/ingest/ingest.mjs` |
| a new kind of effect for one game | the ruleset: register a custom effect, handle it in `apply_custom` |
| a generic building block | `Effect` in the kernel, plus its YAML form in `arena-content` |
| a game mode | a `GameMode` implementation and a `ModeConfig` variant |
| a different AI | a `Policy` implementation, or drive `Match::step_with` from outside |
| another game | a new ruleset crate and a content pack; nothing else changes |

# LoL Damage Calculator

React + Vite app for calculating League of Legends champion damage across combos, items, and levels.

## Data

Champion/item data is pulled from Riot's Data Dragon and Community Dragon at build time.

```bash
# Fetch latest patch data for all champions (~5 min, ~50 MB output)
npm run fetch-data

# Fetch just a few champions (useful for testing)
node scripts/fetch-data.js --sample=Lux,Aatrox,Ahri
```

Output lands in `src/data/generated/`:
- `meta.json` — patch version + fetch timestamp
- `champions.json` — list of all champs with base stats
- `champion-details.json` — per-champ spell tooltips + raw bin data
- `items.json` — purchasable Summoner's Rift items

A weekly GitHub Action (`.github/workflows/refresh-data.yml`) opens a PR with refreshed data every Wednesday.

## Dev

```bash
npm install
npm run dev
```

## Arena: the combat sandbox engine

The repo is being pivoted to a real-time fight simulator: a generic Rust engine
with League of Legends as its first content pack. The React app above is
unchanged for now.

```bash
cargo test --workspace                                   # all engine tests
cargo run --release -p arena-cli -- validate content/lol
cargo run --release -p arena-cli -- run scenarios/1v1_garen_vs_ashe.yaml --seed 1
cargo run --release -p arena-cli -- batch scenarios/3v3_points.yaml --runs 500
```

- `crates/` — the engine, one crate per layer
- `content/lol/` — the LoL pack: `generated/` numbers, `authored/` behaviour
- `scenarios/` — who fights whom, where, and under which win condition
- `tools/ingest/ingest.mjs` — turns the fetched Riot data into `content/lol/generated/`

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the layers and their
contracts, and [docs/PLAN.md](docs/PLAN.md) for status and next steps.

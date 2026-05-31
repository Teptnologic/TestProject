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

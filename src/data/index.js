// Turn raw bin.json spell entries into a clean per-ability shape.
// Each champion's 4 main abilities (Q/W/E/R) + passive get normalized into:
//   { key, name, dataValues: {Name -> values[]}, calculations: {OutputName -> formula}, maxrank, cost, cooldown, range }

import champions from './generated/champions.json';
import details from './generated/champion-details.json';
import items from './generated/items.json';

const ABILITY_KEYS = ['P', 'Q', 'W', 'E', 'R'];

function normalizeDataValues(rawDV) {
  if (!Array.isArray(rawDV)) return {};
  const out = {};
  for (const dv of rawDV) {
    if (dv && dv.name && Array.isArray(dv.values)) {
      out[dv.name] = dv.values;
    }
  }
  return out;
}

function normalizeCalculations(rawCalc) {
  if (!rawCalc || typeof rawCalc !== 'object') return {};
  const out = {};
  for (const [name, calc] of Object.entries(rawCalc)) {
    if (!calc || !Array.isArray(calc.mFormulaParts)) continue;
    out[name] = {
      parts: calc.mFormulaParts.map(normalizeFormulaPart).filter(Boolean),
      multiplier: calc.mMultiplier?.mNumber,
    };
  }
  return out;
}

function normalizeFormulaPart(p) {
  if (!p || !p.__type) return null;
  switch (p.__type) {
    case 'NamedDataValueCalculationPart':
      return { kind: 'dataValue', name: p.mDataValue };
    case 'StatByNamedDataValueCalculationPart':
      // Multiplies a stat (default: AP) by a named data value (ratio)
      return { kind: 'statByDataValue', name: p.mDataValue, stat: p.mStat || 'AP' };
    case 'StatByCoefficientCalculationPart':
      // Stat (default: AP) * fixed coefficient
      return { kind: 'statByCoefficient', coefficient: p.mCoefficient, stat: p.mStat || 'AP' };
    case 'ByCharLevelFormulaCalculationPart':
      return { kind: 'byCharLevel', values: p.values };
    case 'NumberCalculationPart':
      return { kind: 'number', value: p.mNumber };
    default:
      return { kind: 'unknown', type: p.__type };
  }
}

// Pick the most useful "main" spell entry for a given ability key.
// Lux Q has 3 entries (Dummy/Missile/main); we want the one with DataValues + Calculations.
function pickMainSpellEntry(bin, spellIdentifier) {
  if (!bin) return null;
  const candidates = [];
  for (const [path, entry] of Object.entries(bin)) {
    if (!entry?.mSpell) continue;
    const last = path.split('/').pop();
    // Match by mScriptName matching the identifier (e.g., "LuxLightBinding")
    if (entry.mScriptName === spellIdentifier || last === spellIdentifier) {
      candidates.push({ path, entry });
    }
  }
  // Prefer the one with DataValues populated
  candidates.sort((a, b) => {
    const aHas = a.entry.mSpell?.DataValues ? 1 : 0;
    const bHas = b.entry.mSpell?.DataValues ? 1 : 0;
    return bHas - aHas;
  });
  return candidates[0]?.entry || null;
}

function normalizeSpell(rawEntry, ddragonSpell, abilityKey) {
  const mSpell = rawEntry?.mSpell || {};
  return {
    key: abilityKey,
    id: rawEntry?.mScriptName || ddragonSpell?.id,
    name: ddragonSpell?.name || rawEntry?.mScriptName,
    icon: ddragonSpell?.icon,
    description: ddragonSpell?.description,
    tooltip: ddragonSpell?.tooltip,
    maxrank: ddragonSpell?.maxrank ?? (abilityKey === 'R' ? 3 : 5),
    cooldown: ddragonSpell?.cooldown || mSpell.cooldownTime,
    cost: ddragonSpell?.cost,
    range: ddragonSpell?.range,
    effect: ddragonSpell?.effect,
    dataValues: normalizeDataValues(mSpell.DataValues),
    calculations: normalizeCalculations(mSpell.mSpellCalculations),
    coefficient: mSpell.mCoefficient,
  };
}

function normalizeChampion(meta) {
  const detail = details[meta.id];
  if (!detail) return { ...meta, abilities: [] };

  const ddragonSpells = detail.ddragon?.spells || [];
  const bin = detail.cdragon?.bin || {};

  // CharacterRecords/Root tells us the canonical spell paths
  const rootKey = Object.keys(bin).find((k) => k.includes('CharacterRecords/Root'));
  const root = rootKey ? bin[rootKey] : {};
  const spellPaths = root.spellNames || [];

  const abilities = [];

  // Passive
  const passiveEntry = Object.entries(bin).find(([k]) => /Passive/.test(k) && bin[k]?.mSpell?.DataValues);
  if (passiveEntry) {
    abilities.push({
      ...normalizeSpell(passiveEntry[1], null, 'P'),
      name: detail.ddragon?.passive?.name,
      icon: detail.ddragon?.passive?.icon,
      description: detail.ddragon?.passive?.description,
      maxrank: 1,
    });
  }

  // Q/W/E/R from CharacterRecords spell paths
  for (let i = 0; i < 4 && i < spellPaths.length; i++) {
    const path = spellPaths[i]; // e.g. "LuxLightBindingAbility/LuxLightBinding"
    const identifier = path.split('/').pop();
    const entry = pickMainSpellEntry(bin, identifier);
    const ddSpell = ddragonSpells[i];
    abilities.push(normalizeSpell(entry, ddSpell, ABILITY_KEYS[i + 1]));
  }

  return {
    id: meta.id,
    key: meta.key,
    name: meta.name,
    title: meta.title,
    tags: meta.tags,
    stats: meta.stats,
    abilities,
    passive: detail.ddragon?.passive,
  };
}

// Build the normalized champion index
const championIndex = {};
for (const c of champions) {
  championIndex[c.id] = normalizeChampion(c);
}

export const championList = champions.map((c) => ({
  id: c.id,
  name: c.name,
  title: c.title,
  tags: c.tags,
}));

export function getChampion(id) {
  return championIndex[id];
}

export const itemList = items;

export function getItem(id) {
  return items.find((i) => i.id === id);
}

// Convenience: filter items to common damage-relevant ones
export function searchItems(query) {
  const q = query.toLowerCase();
  return items.filter((i) => i.name && i.name.toLowerCase().includes(q));
}

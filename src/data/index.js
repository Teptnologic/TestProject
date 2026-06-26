// Turn raw bin.json spell entries into a clean per-ability shape.
// Each champion's 4 main abilities (Q/W/E/R) + passive get normalized into:
//   { key, name, dataValues: {Name -> values[]}, calculations: {OutputName -> formula}, maxrank, cost, cooldown, range }

import champions from './generated/champions.json';
import details from './generated/champion-details.json';
import items from './generated/items.json';

const itemBinModules = import.meta.glob('./generated/item-bin.json', { eager: true });
const itemBinData = Object.values(itemBinModules)[0]?.default || null;

const ABILITY_KEYS = ['P', 'Q', 'W', 'E', 'R'];

const STAT_ENUM = {
  0: 'AP',
  1: 'Armor',
  2: 'AD',
  3: 'AS',
  4: 'AS_Pct',
  11: 'HP',
  12: 'HP',
  14: 'AP',
  15: 'MR',
  28: 'BonusAD',
  29: 'BonusHP',
  36: 'MaxHP',
};

function resolveStatName(mStat, mStatFormula) {
  if (typeof mStat === 'string') return mStat;
  const base = STAT_ENUM[mStat] || 'AP';
  if (mStatFormula === 1 && base === 'AD') return 'BaseAD';
  if (mStatFormula === 2 && base === 'AD') return 'BonusAD';
  if (mStatFormula === 2 && base === 'HP') return 'BonusHP';
  return base;
}

function normalizeDataValues(rawDV) {
  if (!Array.isArray(rawDV)) return {};
  const out = {};
  for (const dv of rawDV) {
    if (dv && dv.name && Array.isArray(dv.values)) {
      out[dv.name] = dv.values;
      const lower = dv.name.toLowerCase();
      if (lower !== dv.name) out[lower] = dv.values;
    }
  }
  return out;
}

function normalizeCalculations(rawCalc) {
  if (!rawCalc || typeof rawCalc !== 'object') return {};
  const out = {};
  for (const [name, calc] of Object.entries(rawCalc)) {
    if (!calc || !Array.isArray(calc.mFormulaParts)) continue;
    const mult = calc.mMultiplier;
    out[name] = {
      parts: calc.mFormulaParts.map(normalizeFormulaPart).filter(Boolean),
      multiplier: mult?.mNumber,
      multiplierPart: mult && !mult.mNumber ? normalizeFormulaPart(mult) : undefined,
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
      return { kind: 'statByDataValue', name: p.mDataValue, stat: resolveStatName(p.mStat, p.mStatFormula) };
    case 'StatByCoefficientCalculationPart':
      return { kind: 'statByCoefficient', coefficient: p.mCoefficient, stat: resolveStatName(p.mStat, p.mStatFormula) };
    case 'ByCharLevelFormulaCalculationPart':
      return { kind: 'byCharLevel', values: p.values };
    case 'ByCharLevelBreakpointsCalculationPart':
      return { kind: 'byCharLevelBreakpoints', baseValue: p.mLevel1Value, initialPerLevel: p.mInitialBonusPerLevel, breakpoints: p.mBreakpoints };
    case 'ByCharLevelInterpolationCalculationPart':
      return { kind: 'byCharLevelInterp', start: p.mStartValue, end: p.mEndValue };
    case 'StatBySubPartCalculationPart':
      return { kind: 'statBySubPart', stat: resolveStatName(p.mStat, p.mStatFormula), subPart: normalizeFormulaPart(p.mSubpart) };
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


// Merge list stats (wiki-patched) with detail stats. Detail wins unless its
// value is 0/falsy — DDragon's detail endpoint returns 0 for per-level growth
// on every champion, so we keep the wiki-patched list value in that case.
function mergeStats(listStats, detailStats) {
  const out = { ...listStats };
  if (!detailStats) return out;
  for (const [k, v] of Object.entries(detailStats)) {
    if (v != null && v !== 0) out[k] = v;
  }
  return out;
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

  // Passive — always include from DDragon; optionally enrich with bin data
  // Look for explicit "Passive" path, or hash-keyed entries with mSpell that
  // aren't referenced by spellNames (Q/W/E/R)
  const spellScriptNames = new Set(spellPaths.map((p) => p.split('/').pop()));
  const hasSpellData = (v) => v?.mSpell?.DataValues || v?.mSpell?.mSpellCalculations;
  let passiveEntry = Object.entries(bin).find(([k, v]) =>
    (/Passive/i.test(k) || /Passive/i.test(v?.mScriptName || '')) && hasSpellData(v)
  );
  if (!passiveEntry) {
    passiveEntry = Object.entries(bin).find(([k, v]) => {
      if (k.includes('CharacterRecords')) return false;
      if (k.includes('BasicAttack') || k.includes('CritAttack')) return false;
      if (!hasSpellData(v)) return false;
      const scriptName = v.mScriptName || k.split('/').pop();
      return !spellScriptNames.has(scriptName) && !spellPaths.some((p) => k.includes(p));
    });
  }
  if (passiveEntry) {
    abilities.push({
      ...normalizeSpell(passiveEntry[1], null, 'P'),
      name: detail.ddragon?.passive?.name,
      icon: detail.ddragon?.passive?.icon,
      description: detail.ddragon?.passive?.description,
      maxrank: 1,
    });
  } else if (detail.ddragon?.passive) {
    abilities.push({
      key: 'P',
      name: detail.ddragon.passive.name,
      icon: detail.ddragon.passive.icon,
      description: detail.ddragon.passive.description,
      maxrank: 1,
      dataValues: {},
      calculations: {},
    });
  }

  // Q/W/E/R from CharacterRecords spell paths
  const mainScriptNames = new Set();
  for (let i = 0; i < 4 && i < spellPaths.length; i++) {
    const path = spellPaths[i]; // e.g. "LuxLightBindingAbility/LuxLightBinding"
    const identifier = path.split('/').pop();
    mainScriptNames.add(identifier);
    const entry = pickMainSpellEntry(bin, identifier);
    const ddSpell = ddragonSpells[i];
    abilities.push(normalizeSpell(entry, ddSpell, ABILITY_KEYS[i + 1]));
  }

  // Recast spells: hash-keyed bin entries with mSpell that aren't passive or main spells
  const recastAbilities = {};
  const passiveScriptName = passiveEntry?.[1]?.mScriptName;
  for (const [k, v] of Object.entries(bin)) {
    if (!v?.mSpell || !v.mScriptName) continue;
    if (mainScriptNames.has(v.mScriptName)) continue;
    if (v.mScriptName === passiveScriptName) continue;
    if (k.includes('CharacterRecords')) continue;
    const spell = normalizeSpell(v, null, v.mScriptName);
    if (spell.dataValues && Object.keys(spell.dataValues).length) {
      recastAbilities[v.mScriptName] = spell;
    }
  }

  return {
    id: meta.id,
    key: meta.key,
    name: meta.name,
    title: meta.title,
    tags: meta.tags,
    stats: mergeStats(meta.stats, detail.ddragon?.stats),
    abilities,
    recastAbilities,
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

function normalizeItemCalculations(rawCalc) {
  if (!rawCalc || typeof rawCalc !== 'object') return {};
  const out = {};
  for (const [name, calc] of Object.entries(rawCalc)) {
    if (!calc) continue;
    if (calc.__type === 'GameCalculationModified') {
      out[name] = {
        modified: calc.mModifiedGameCalculation,
        multiplier: calc.mMultiplier,
        parts: [],
      };
      if (calc.mMultiplier) {
        out[name].multiplierParts = [normalizeItemFormulaPart(calc.mMultiplier)].filter(Boolean);
      }
      continue;
    }
    if (!Array.isArray(calc.mFormulaParts)) continue;
    out[name] = {
      parts: calc.mFormulaParts.map(normalizeItemFormulaPart).filter(Boolean),
    };
  }
  return out;
}

function normalizeItemFormulaPart(p) {
  if (!p || !p.__type) return null;
  switch (p.__type) {
    case 'NamedDataValueCalculationPart':
      return { kind: 'dataValue', name: p.mDataValue };
    case 'StatByNamedDataValueCalculationPart':
      return { kind: 'statByDataValue', name: p.mDataValue, stat: resolveStatName(p.mStat, p.mStatFormula) };
    case 'StatByCoefficientCalculationPart':
      return { kind: 'statByCoefficient', coefficient: p.mCoefficient, stat: resolveStatName(p.mStat, p.mStatFormula) };
    case 'ByCharLevelBreakpointsCalculationPart':
      return { kind: 'byCharLevelBreakpoints', baseValue: p.mLevel1Value, breakpoints: p.mBreakpoints };
    case 'ByCharLevelInterpolationCalculationPart':
      return { kind: 'byCharLevelInterp', start: p.mStartValue, end: p.mEndValue };
    case 'NumberCalculationPart':
      return { kind: 'number', value: p.mNumber };
    default:
      return null;
  }
}

function normalizeItemBin(binEntry) {
  if (!binEntry) return null;
  const result = {};
  if (binEntry.dataValues) result.dataValues = binEntry.dataValues;
  if (binEntry.calculations) result.calculations = normalizeItemCalculations(binEntry.calculations);
  if (binEntry.directStats) result.directStats = binEntry.directStats;
  if (binEntry.effectAmount) result.effectAmount = binEntry.effectAmount;
  return Object.keys(result).length ? result : null;
}

// Build enriched item index with bin data (exclude Arena/game-mode variants with 6-digit IDs)
const itemIndex = {};
for (const item of items) {
  if (item.id >= 100000) continue;
  const binEntry = itemBinData?.[String(item.id)];
  const bin = normalizeItemBin(binEntry);
  itemIndex[item.id] = bin ? { ...item, bin } : item;
}

export const itemList = Object.values(itemIndex);

export function getItem(id) {
  return itemIndex[id] || null;
}

// Convenience: filter items to common damage-relevant ones
export function searchItems(query) {
  const q = query.toLowerCase();
  return items.filter((i) => i.name && i.name.toLowerCase().includes(q));
}

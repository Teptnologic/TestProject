// Evaluate a spell's damage calculation against an attacker's stats.

import { totalStats } from './stats.js';

// Pick the most relevant "damage" calculation from a spell's calculations map.
function pickDamageCalc(calculations) {
  if (!calculations) return null;
  const keys = Object.keys(calculations);
  // Prefer "TotalDamage", "TotalDamageTT", "Damage" — fall back to any calc whose
  // formula references BaseDamage/Damage data values
  const preferred = ['TotalDamage', 'TotalDamageTT', 'Damage', 'DamageTT'];
  for (const p of preferred) if (calculations[p]) return { name: p, calc: calculations[p] };
  for (const [name, calc] of Object.entries(calculations)) {
    if (calc.parts?.some((p) => p.kind === 'dataValue' && /Damage/i.test(p.name))) {
      return { name, calc };
    }
  }
  return null;
}

function statValue(attacker, statName) {
  switch (statName) {
    case 'AP': return attacker.ap || 0;
    case 'AD': return attacker.attackdamage || 0;
    case 'BonusAD': return attacker.bonusAD || 0;
    case 'BonusHP': return attacker.bonusHP || 0;
    case 'MaxHP': return attacker.hp || 0;
    default: return 0;
  }
}

// Evaluate a single calculation at the given ability rank + attacker stats
function evaluateCalc(calc, rank, attacker, charLevel) {
  let total = 0;
  for (const part of calc.parts) {
    switch (part.kind) {
      case 'dataValue': {
        const arr = attacker.spellDataValues?.[part.name];
        if (arr) total += arr[rank] ?? 0;
        break;
      }
      case 'statByDataValue': {
        const arr = attacker.spellDataValues?.[part.name];
        const ratio = arr ? arr[rank] ?? 0 : 0;
        total += statValue(attacker, part.stat) * ratio;
        break;
      }
      case 'statByCoefficient': {
        total += statValue(attacker, part.stat) * (part.coefficient || 0);
        break;
      }
      case 'byCharLevel': {
        const idx = Math.max(0, Math.min(charLevel - 1, part.values.length - 1));
        total += part.values[idx];
        break;
      }
      case 'number': total += part.value || 0; break;
      default: break;
    }
  }
  if (calc.multiplier !== undefined) total *= calc.multiplier;
  return total;
}

// Compute raw damage for one ability cast
// rank is 1-based (1..maxrank). We map to the DataValues array which is 0-indexed
// but includes a rank-0 entry, so DataValues[rank] gives us the rank-1..5 values.
export function computeAbilityDamage(ability, rank, attacker, charLevel) {
  if (!ability) return null;
  const { calc, name: calcName } = pickDamageCalc(ability.calculations) || {};
  if (!calc) return null;
  const attackerWithSpell = { ...attacker, spellDataValues: ability.dataValues };
  const raw = evaluateCalc(calc, rank, attackerWithSpell, charLevel);
  return {
    abilityKey: ability.key,
    abilityName: ability.name,
    calcName,
    raw,
    type: detectDamageType(ability),
  };
}

// Heuristic: most abilities with AP scaling are magic, most with AD/BonusAD are physical.
function detectDamageType(ability) {
  const desc = (ability.description || '') + ' ' + (ability.tooltip || '');
  if (/physicalDamage/i.test(desc)) return 'physical';
  if (/trueDamage/i.test(desc)) return 'true';
  if (/magicDamage/i.test(desc)) return 'magic';
  // Fall back to inspecting the calculation
  const calc = pickDamageCalc(ability.calculations)?.calc;
  if (calc) {
    for (const p of calc.parts) {
      if (p.stat === 'AD' || p.stat === 'BonusAD') return 'physical';
      if (p.stat === 'AP') return 'magic';
    }
  }
  return 'magic';
}

// Apply armor/MR + penetration to convert raw damage to post-mitigation damage
export function applyResistance(rawDamage, type, target, attacker) {
  if (type === 'true') return rawDamage;
  const resist = type === 'physical' ? target.armor || 0 : target.spellblock || 0;
  const flatPen = type === 'physical' ? attacker.lethality || 0 : attacker.flatMagicPen || 0;
  const pctPen = type === 'physical' ? attacker.armorPenPct || 0 : attacker.magicPenPct || 0;
  let effective = Math.max(0, resist * (1 - pctPen) - flatPen);
  const mult = effective >= 0 ? 100 / (100 + effective) : 2 - 100 / (100 - effective);
  return rawDamage * mult;
}

// Sum a combo's worth of damage
export function computeCombo(combo, champion, ranks, attackerStats, target, charLevel) {
  const steps = [];
  let totalPhys = 0, totalMagic = 0, totalTrue = 0;
  for (const step of combo) {
    const ability = champion.abilities.find((a) => a.key === step);
    if (!ability) continue;
    const rank = ranks[step] || 1;
    const result = computeAbilityDamage(ability, rank, attackerStats, charLevel);
    if (!result || !result.raw) continue;
    const post = applyResistance(result.raw, result.type, target, attackerStats);
    if (result.type === 'physical') totalPhys += post;
    else if (result.type === 'true') totalTrue += post;
    else totalMagic += post;
    steps.push({ ...result, post });
  }
  return {
    steps,
    physical: totalPhys,
    magic: totalMagic,
    true: totalTrue,
    total: totalPhys + totalMagic + totalTrue,
  };
}

export { totalStats };

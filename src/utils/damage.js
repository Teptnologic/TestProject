// Evaluate a spell's damage calculation against an attacker's stats.

import { totalStats, baseStatsAtLevel } from './stats.js';

// Compute auto-attack damage (physical = total AD)
function computeAADamage(attacker, items) {
  const results = [
    { abilityKey: 'AA', abilityName: 'Auto Attack', raw: attacker.attackdamage, type: 'physical' },
  ];
  // On-hit item effects
  for (const item of items || []) {
    const ov = item?._overrides;
    if (!ov?.passive) continue;
    const p = ov.passive;
    switch (p.type) {
      case 'nashors':
        results.push({
          abilityKey: 'AA',
          abilityName: `Nashor's On-Hit`,
          raw: (p.flatOnHit || 0) + (p.apRatio || 0) * (attacker.ap || 0),
          type: 'magic',
        });
        break;
      case 'witsEnd': {
        const lvl = attacker.level || 1;
        const min = p.flatOnHitMin || 15;
        const max = p.flatOnHitMax || 80;
        const dmg = min + (max - min) * ((lvl - 1) / 17);
        results.push({
          abilityKey: 'AA',
          abilityName: `Wit's End On-Hit`,
          raw: dmg,
          type: 'magic',
        });
        break;
      }
      case 'botrk': {
        const currentHP = attacker._targetCurrentHP || attacker._targetMaxHP || 2000;
        results.push({
          abilityKey: 'AA',
          abilityName: 'BotRK On-Hit',
          raw: currentHP * (p.currentHpRatioMelee || 0.09),
          type: 'physical',
        });
        break;
      }
    }
  }
  return results;
}

// Compute spellblade proc (Sheen/Lich Bane/Trinity/Essence Reaver)
function computeSpellbladeDamage(attacker, items) {
  let best = null;
  for (const item of items || []) {
    const p = item?._overrides?.passive;
    if (!p || p.type !== 'spellblade') continue;
    const baseAD = attacker.baseAD || attacker.attackdamage;
    let raw = baseAD * (p.baseAdRatio || p.baseADRatio || 1);
    if (p.apRatio) raw += (attacker.ap || 0) * p.apRatio;
    const type = p.damageType || 'physical';
    if (!best || raw > best.raw) best = { raw, type, name: item.name };
  }
  return best;
}

// Compute item proc damage that triggers on ability hits (Luden's, Stormsurge, etc.)
function computeItemProcs(attacker, items, target, abilityKey) {
  const results = [];
  for (const item of items || []) {
    const p = item?._overrides?.passive;
    if (!p) continue;
    switch (p.type) {
      case 'ludens': {
        const raw = (p.flatDamage || 0) + (p.apRatio || 0) * (attacker.ap || 0);
        results.push({ abilityKey, abilityName: `Luden's Echo`, raw, type: 'magic' });
        break;
      }
      case 'stormsurge': {
        const lvl = attacker.level || 1;
        const flat = (p.flatDamageMin || 100) + ((p.flatDamageMax || 200) - (p.flatDamageMin || 100)) * ((lvl - 1) / 17);
        const raw = flat + (p.apRatio || 0) * (attacker.ap || 0);
        results.push({ abilityKey, abilityName: 'Stormsurge', raw, type: 'magic' });
        break;
      }
      case 'blackfireTorch': {
        const raw = (p.flatDamageTotal || 0) + (p.apRatio || 0) * (attacker.ap || 0);
        results.push({ abilityKey, abilityName: 'Blackfire Torch', raw, type: 'magic' });
        break;
      }
      case 'liandrys': {
        const targetHP = target?.hp || 2000;
        const raw = targetHP * (p.maxHpPerSecond || 0.02) * (p.duration || 3);
        results.push({ abilityKey, abilityName: `Liandry's Burn`, raw, type: 'magic' });
        break;
      }
      case 'malignance': {
        if (abilityKey !== 'R') break;
        const raw = ((p.flatDamagePerSecond || 0) + (p.apRatioPerSecond || 0) * (attacker.ap || 0)) * (p.duration || 3);
        results.push({ abilityKey, abilityName: 'Malignance', raw, type: 'magic' });
        break;
      }
    }
  }
  return results;
}

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
export function computeCombo(combo, champion, ranks, attackerStats, target, charLevel, items) {
  const steps = [];
  let totalPhys = 0, totalMagic = 0, totalTrue = 0;
  let lastWasSpell = false;
  let itemProcsUsed = false;

  function addDmg(result) {
    const post = applyResistance(result.raw, result.type, target, attackerStats);
    if (result.type === 'physical') totalPhys += post;
    else if (result.type === 'true') totalTrue += post;
    else totalMagic += post;
    steps.push({ ...result, post });
  }

  for (const step of combo) {
    if (step === 'AA') {
      const aaResults = computeAADamage(attackerStats, items);
      for (const r of aaResults) addDmg(r);
      if (lastWasSpell) {
        const sb = computeSpellbladeDamage(attackerStats, items);
        if (sb) addDmg({ abilityKey: 'AA', abilityName: sb.name + ' Proc', raw: sb.raw, type: sb.type });
      }
      lastWasSpell = false;
      continue;
    }

    const ability = champion.abilities.find((a) => a.key === step);
    if (!ability) continue;
    const rank = step === 'P' ? 1 : (ranks[step] || 1);
    const result = computeAbilityDamage(ability, rank, attackerStats, charLevel);
    if (result && result.raw) addDmg(result);

    if (!itemProcsUsed) {
      const procs = computeItemProcs(attackerStats, items, target, step);
      for (const r of procs) addDmg(r);
      if (procs.length > 0) itemProcsUsed = true;
    } else if (step === 'R') {
      const procs = computeItemProcs(attackerStats, items, target, 'R');
      const ultOnly = procs.filter((r) => r.abilityName === 'Malignance');
      for (const r of ultOnly) addDmg(r);
    }

    lastWasSpell = true;
  }

  return {
    steps,
    physical: totalPhys,
    magic: totalMagic,
    true: totalTrue,
    total: totalPhys + totalMagic + totalTrue,
  };
}

export { totalStats, baseStatsAtLevel };

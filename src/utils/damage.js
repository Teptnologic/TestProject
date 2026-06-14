// Evaluate a spell's damage calculation against an attacker's stats.

import { totalStats, baseStatsAtLevel } from './stats.js';

// Compute auto-attack damage (physical = total AD), with crit expected value and on-hits
function computeAADamage(attacker, items, aaIndex, targetCurrentHP, targetMaxHP) {
  // Crit expected value
  const critChance = Math.min(1, attacker.crit || 0);
  let critDamageMultiplier = 1.75;
  if (critChance > 0) {
    for (const item of items || []) {
      const ov = item?._overrides;
      if (ov?.passive?.type === 'infinityEdge' && critChance >= (ov.passive.critThreshold || 0.40)) {
        critDamageMultiplier += ov.passive.critDamageBonus || 0.35;
        break;
      }
    }
  }
  const expectedCritMult = critChance > 0 ? 1 + critChance * (critDamageMultiplier - 1) : 1;
  const baseDmg = attacker.attackdamage * expectedCritMult;

  const aaLabel = critChance > 0
    ? `Auto Attack (${Math.round(critChance * 100)}% crit)`
    : 'Auto Attack';

  const results = [
    { abilityKey: 'AA', abilityName: aaLabel, raw: baseDmg, type: 'physical' },
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
        const hp = targetCurrentHP || 2000;
        results.push({
          abilityKey: 'AA',
          abilityName: `BotRK On-Hit (${Math.round(hp)} HP)`,
          raw: hp * (p.currentHpRatioMelee || 0.09),
          type: 'physical',
        });
        break;
      }
      case 'kraken': {
        const bin = item?.bin;
        const procEvery = bin?.dataValues?.AttackCount || 3;
        if ((aaIndex + 1) % procEvery === 0) {
          let raw = 0;
          if (bin?.calculations?.DamageAmount) {
            raw = evaluateItemCalc(bin.calculations.DamageAmount, bin.dataValues || {}, attacker, attacker.level || 1);
          }
          // Missing health scaling: damage * (1 + (MaxAmpNumber - 1) * missingHealthPct)
          const maxAmp = bin?.dataValues?.MaxAmpNumber || 1;
          if (maxAmp > 1 && targetMaxHP > 0) {
            const missingPct = Math.max(0, 1 - (targetCurrentHP || targetMaxHP) / targetMaxHP);
            raw *= 1 + (maxAmp - 1) * missingPct;
          }
          results.push({
            abilityKey: 'AA',
            abilityName: `Kraken Slayer (${aaIndex + 1}${ordSuffix(aaIndex + 1)} hit)`,
            raw,
            type: p.damageType || 'physical',
          });
        }
        break;
      }
    }
  }
  return results;
}

function ordSuffix(n) {
  if (n % 100 >= 11 && n % 100 <= 13) return 'th';
  switch (n % 10) {
    case 1: return 'st';
    case 2: return 'nd';
    case 3: return 'rd';
    default: return 'th';
  }
}

// Compute spellblade proc (Sheen/Lich Bane/Trinity/Essence Reaver/Dusk and Dawn)
function computeSpellbladeDamage(attacker, items) {
  let best = null;
  for (const item of items || []) {
    // Try bin-based SpellbladeDamage calculation first
    const bin = item?.bin;
    if (bin?.calculations?.SpellbladeDamage) {
      const raw = evaluateItemCalc(bin.calculations.SpellbladeDamage, bin.dataValues, attacker, attacker.level || 1);
      // Detect damage type: items with AP scaling (Lich Bane, Dusk and Dawn) deal magic
      const hasAPScaling = bin.calculations.SpellbladeDamage.parts?.some(
        (p) => (p.kind === 'statByCoefficient' || p.kind === 'statByDataValue') && p.stat === 'AP'
      );
      const type = hasAPScaling ? 'magic' : 'physical';
      if (raw > 0 && (!best || raw > best.raw)) best = { raw, type, name: item.name };
      continue;
    }
    // Fallback to manual overrides
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

// Compute item active damage (Rocketbelt, Gunblade, etc.)
function computeItemActiveDamage(itemId, attacker, items) {
  for (const item of items || []) {
    if (item?.id !== itemId) continue;
    const active = item?._overrides?.active;
    if (!active) {
      const bin = item?.bin;
      if (bin?.calculations) {
        const found = pickItemDamageCalc(bin.calculations);
        if (found) {
          const raw = evaluateItemCalc(found.calc, bin.dataValues || {}, attacker, attacker.level || 1);
          if (raw > 0) return { abilityKey: 'ITEM', abilityName: item.name, raw, type: 'magic' };
        }
      }
      return null;
    }
    let raw = 0;
    if (active.flatDamage != null) {
      raw += active.flatDamage;
    } else if (active.flatDamageMin != null && active.flatDamageMax != null) {
      const t = ((attacker.level || 1) - 1) / 17;
      raw += active.flatDamageMin + (active.flatDamageMax - active.flatDamageMin) * t;
    }
    if (active.apRatio) raw += (attacker.ap || 0) * active.apRatio;
    if (active.adRatio) raw += (attacker.attackdamage || 0) * active.adRatio;
    if (active.bonusAdRatio) raw += (attacker.bonusAD || 0) * active.bonusAdRatio;
    return {
      abilityKey: 'ITEM',
      abilityName: active.name || item.name,
      raw,
      type: active.damageType || 'magic',
    };
  }
  return null;
}

// Get list of equipped items that have actives (for UI)
export function getEquippedActives(items) {
  const actives = [];
  for (const item of items || []) {
    if (!item) continue;
    if (item._overrides?.active) {
      actives.push({ id: item.id, name: item._overrides.active.name || item.name, itemName: item.name });
    }
  }
  return actives;
}

// Evaluate an item calculation using bin data (same as champion evaluateCalc but
// item dataValues are single numbers, not per-rank arrays)
function evaluateItemCalc(calc, dataValues, attacker, charLevel) {
  let total = 0;
  for (const part of calc.parts || []) {
    switch (part.kind) {
      case 'dataValue':
        total += dataValues?.[part.name] ?? 0;
        break;
      case 'statByDataValue': {
        const ratio = dataValues?.[part.name] ?? 0;
        total += statValue(attacker, part.stat) * ratio;
        break;
      }
      case 'statByCoefficient':
        total += statValue(attacker, part.stat) * (part.coefficient || 0);
        break;
      case 'byCharLevelBreakpoints': {
        let val = part.baseValue || 0;
        if (part.initialPerLevel) val += part.initialPerLevel * (charLevel - 1);
        if (part.breakpoints) {
          for (const bp of part.breakpoints) {
            if (charLevel >= (bp.mLevel || 1)) {
              const perLevel = bp['{57fdc438}'] || bp.mPerLevel || bp.mBonusPerLevelAtAndAfter || 0;
              val += perLevel * (charLevel - (bp.mLevel || 1));
              if (bp.mAdditionalBonusAtThisLevel) val += bp.mAdditionalBonusAtThisLevel;
            }
          }
        }
        total += val;
        break;
      }
      case 'byCharLevelInterp': {
        const t = Math.max(0, Math.min(1, (charLevel - 1) / 17));
        total += (part.start || 0) + ((part.end || 0) - (part.start || 0)) * t;
        break;
      }
      case 'number':
        total += part.value || 0;
        break;
    }
  }
  return total;
}

// Pick the best damage calculation from item bin data
function pickItemDamageCalc(calculations) {
  if (!calculations) return null;
  const preferred = [
    'Damage', 'TotalDamage', 'SquallDamage', 'MeleeItemCalcValue',
    'DamageAmount', 'TotalDamageAmount', 'PassiveDamage', 'ProcDamage',
  ];
  for (const name of preferred) {
    if (calculations[name]?.parts?.length) return { name, calc: calculations[name] };
  }
  for (const [name, calc] of Object.entries(calculations)) {
    if (/damage|burn/i.test(name) && calc.parts?.length && name !== 'SpellbladeDamage') {
      return { name, calc };
    }
  }
  return null;
}

// Known item IDs for special handling
const ITEM_IDS = {
  LIANDRYS: 6653,
  BLACKFIRE: 2503,
  MALIGNANCE: 3118,
};

// Compute item proc damage that triggers on ability hits
function computeItemProcs(attacker, items, target, abilityKey) {
  const results = [];
  for (const item of items || []) {
    const bin = item?.bin;
    if (!bin) continue;
    const dv = bin.dataValues || {};

    // Liandry's: % max HP burn, no formula calc — use dataValues directly
    if (item.id === ITEM_IDS.LIANDRYS && dv.BurnPercentHealthDamage) {
      const targetHP = target?.hp || 2000;
      const raw = targetHP * dv.BurnPercentHealthDamage * (dv.BurnDuration || 3);
      results.push({ abilityKey, abilityName: `Liandry's Burn`, raw, type: 'magic' });
      continue;
    }

    // Malignance: ult-only ground burn
    if (item.id === ITEM_IDS.MALIGNANCE) {
      if (abilityKey !== 'R') continue;
      const found = pickItemDamageCalc(bin.calculations);
      if (found) {
        const perTick = evaluateItemCalc(found.calc, dv, attacker, attacker.level || 1);
        const raw = perTick * (dv.GroundDuration || 3);
        results.push({ abilityKey, abilityName: 'Malignance', raw, type: 'magic' });
      }
      continue;
    }

    // Blackfire Torch: burn per second × duration
    if (item.id === ITEM_IDS.BLACKFIRE && bin.calculations) {
      const found = pickItemDamageCalc(bin.calculations);
      if (found) {
        const perSecond = evaluateItemCalc(found.calc, dv, attacker, attacker.level || 1);
        const raw = perSecond * (dv.BurnDuration || 3);
        results.push({ abilityKey, abilityName: 'Blackfire Torch', raw, type: 'magic' });
      }
      continue;
    }

    // General proc items (Luden's, Stormsurge, etc.)
    if (bin.calculations) {
      const found = pickItemDamageCalc(bin.calculations);
      if (!found) continue;

      let raw = evaluateItemCalc(found.calc, dv, attacker, attacker.level || 1);

      // Check for SingleTargetMax or similar modified calc that multiplies base
      for (const [, calc] of Object.entries(bin.calculations)) {
        if (calc.modified === found.name && calc.multiplierParts?.length) {
          const mult = evaluateItemCalc({ parts: calc.multiplierParts }, dv, attacker, attacker.level || 1);
          if (mult > 1) {
            raw *= mult;
            break;
          }
        }
      }

      if (raw > 0) {
        results.push({ abilityKey, abilityName: item.name, raw, type: 'magic' });
      }
    }
  }
  return results;
}

// Pick the most relevant "damage" calculation from a spell's calculations map.
function pickDamageCalc(calculations) {
  if (!calculations) return null;
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
    case 'BaseAD': return attacker.baseAD || attacker.attackdamage || 0;
    case 'BonusAD': return attacker.bonusAD || 0;
    case 'BonusHP': return attacker.bonusHP || 0;
    case 'MaxHP': return attacker.hp || 0;
    default: return 0;
  }
}

// Evaluate a single calculation at the given ability rank + attacker stats
export function evaluateCalc(calc, rank, attacker, charLevel) {
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
      case 'byCharLevelBreakpoints': {
        let val = part.baseValue || 0;
        if (part.initialPerLevel) val += part.initialPerLevel * (charLevel - 1);
        if (part.breakpoints) {
          for (const bp of part.breakpoints) {
            if (charLevel >= (bp.mLevel || 1)) {
              const perLevel = bp['{57fdc438}'] || bp.mPerLevel || bp.mBonusPerLevelAtAndAfter || 0;
              val += perLevel * (charLevel - (bp.mLevel || 1));
              if (bp.mAdditionalBonusAtThisLevel) val += bp.mAdditionalBonusAtThisLevel;
            }
          }
        }
        total += val;
        break;
      }
      case 'byCharLevelInterp': {
        const t = Math.max(0, Math.min(1, (charLevel - 1) / 17));
        total += (part.start || 0) + ((part.end || 0) - (part.start || 0)) * t;
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
  let aaCount = 0;
  const targetMaxHP = (target.hp || 0) + (target.shield || 0);
  let targetCurrentHP = targetMaxHP;

  function addDmg(result) {
    const post = applyResistance(result.raw, result.type, target, attackerStats);
    if (result.type === 'physical') totalPhys += post;
    else if (result.type === 'true') totalTrue += post;
    else totalMagic += post;
    steps.push({ ...result, post });
    targetCurrentHP = Math.max(0, targetCurrentHP - post);
  }

  for (const step of combo) {
    if (step === 'AA') {
      const aaResults = computeAADamage(attackerStats, items, aaCount, targetCurrentHP, targetMaxHP);
      aaCount++;
      for (const r of aaResults) addDmg(r);
      if (lastWasSpell) {
        const sb = computeSpellbladeDamage(attackerStats, items);
        if (sb) addDmg({ abilityKey: 'AA', abilityName: sb.name + ' Proc', raw: sb.raw, type: sb.type });
      }
      lastWasSpell = false;
      continue;
    }

    // Item active steps (e.g., "ITEM_3152" for Rocketbelt)
    if (step.startsWith('ITEM_')) {
      const itemId = parseInt(step.slice(5));
      const result = computeItemActiveDamage(itemId, attackerStats, items);
      if (result) addDmg(result);
      lastWasSpell = true;
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

// Evaluate a spell's damage calculation against an attacker's stats.

import { totalStats, baseStatsAtLevel } from './stats.js';
import { parentAbilityKey, getMultiCasts } from '../data/multi-cast.js';

// Champion-specific AA modifiers
const CHAMPION_AA = {
  Jhin: {
    type: 'nthShot',
    every: 4,
    adMultiplier: 1.5,
    missingHpPct: {
      kind: 'byCharLevelBreakpoints',
      baseValue: 0.15,
      breakpoints: [
        { mLevel: 6, mAdditionalBonusAtThisLevel: 0.05 },
        { mLevel: 11, mAdditionalBonusAtThisLevel: 0.05 },
      ],
    },
  },
  Locke: {
    // Passive (Silver Stake): AA on-hit magic damage scaling with level + AP
    passiveOnHit: true,
    // Q marks: AA and E2 consume marks for bonus damage
    markConsumers: ['AA', 'E2'],
    markAbility: 'Q',
    markDamage: [0, 10, 20, 30, 40, 50, 60, 70],
    markRatio: [0, 0.225, 0.25, 0.275, 0.30, 0.325, 0.35, 0.375],
    twoMarkBonus: 0.20,
    threeMarkBonus: 0.40,
    maxStacks: 3,
    // R execute: base threshold + per-mark bonus
    executeThreshold: [0, 0.09, 0.10, 0.11, 0.12, 0.13, 0.14, 0.15],
    executePerStack: 0.005,
  },
  Vayne: {
    appliesStacks: ['E'],
    onHits: [
      {
        type: 'nthHitPassive',
        ability: 'W',
        every: 3,
        damageType: 'true',
        maxHpRatio: [0, 0.06, 0.07, 0.08, 0.09, 0.10, 0.11],
        minDamage: [0, 50, 65, 80, 95, 110, 125],
        label: 'Silver Bolts',
      },
      {
        type: 'empoweredAA',
        triggerStep: 'Q',
        damageType: 'physical',
        totalAdRatio: [0, 0.75, 0.85, 0.95, 1.05, 1.15, 1.25],
        label: 'Tumble',
      },
    ],
  },
};

function resolveBreakpointValue(bp, charLevel) {
  let val = bp.baseValue || 0;
  if (bp.initialPerLevel) val += bp.initialPerLevel * (charLevel - 1);
  if (bp.breakpoints) {
    for (const b of bp.breakpoints) {
      if (charLevel >= (b.mLevel || 1)) {
        const perLevel = b.mPerLevel || b.mBonusPerLevelAtAndAfter || 0;
        val += perLevel * (charLevel - (b.mLevel || 1));
        if (b.mAdditionalBonusAtThisLevel) val += b.mAdditionalBonusAtThisLevel;
      }
    }
  }
  return val;
}

// Compute auto-attack damage (physical = total AD), with crit expected value and on-hits
function computeAADamage(attacker, items, aaIndex, targetCurrentHP, targetMaxHP, champCtx) {
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
  let baseDmg = attacker.attackdamage * expectedCritMult;

  let aaLabel = critChance > 0
    ? `Auto Attack (${Math.round(critChance * 100)}% crit)`
    : 'Auto Attack';

  // Jhin 4th shot: bonus AD multiplier + missing HP execute
  const champAA = champCtx?.championId ? CHAMPION_AA[champCtx.championId] : null;
  if (champAA?.type === 'nthShot' && (aaIndex + 1) % champAA.every === 0) {
    baseDmg = attacker.attackdamage * champAA.adMultiplier;
    // 4th shot always crits
    baseDmg *= critDamageMultiplier;
    const missingHP = Math.max(0, targetMaxHP - targetCurrentHP);
    const missingPct = resolveBreakpointValue(champAA.missingHpPct, attacker.level || 1);
    baseDmg += missingHP * missingPct;
    aaLabel = `4th Shot (${Math.round(missingPct * 100)}% missing HP)`;
  }

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

  // Champion-specific on-hits (Vayne W, Vayne Q empowered AA)
  if (champAA?.onHits && champCtx) {
    for (const oh of champAA.onHits) {
      if (oh.type === 'nthHitPassive') {
        const rank = champCtx.ranks?.[oh.ability] || 0;
        if (rank > 0 && champCtx.hitCount != null && (champCtx.hitCount + 1) % oh.every === 0) {
          const maxHpRatio = oh.maxHpRatio[rank] || 0;
          const targetHP = targetMaxHP || 2000;
          const raw = Math.max(oh.minDamage[rank] || 0, targetHP * maxHpRatio);
          results.push({
            abilityKey: 'AA',
            abilityName: `${oh.label} (${Math.round(maxHpRatio * 100)}% max HP)`,
            raw,
            type: oh.damageType,
          });
        }
      }
      if (oh.type === 'empoweredAA' && champCtx.empowered === oh.triggerStep) {
        const rank = champCtx.ranks?.[oh.triggerStep] || 0;
        if (rank > 0) {
          const ratio = oh.totalAdRatio[rank] || 0;
          const raw = attacker.attackdamage * ratio;
          results.push({
            abilityKey: 'AA',
            abilityName: `${oh.label} (${Math.round(ratio * 100)}% AD)`,
            raw,
            type: oh.damageType,
          });
        }
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
      case 'statBySubPart': {
        const coeff = evaluateItemCalc({ parts: [part.subPart] }, dataValues, attacker, charLevel);
        total += statValue(attacker, part.stat) * coeff;
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
    // Skip items with an active ability — those are added manually via ITEM_ combo steps
    if (bin.calculations && !item._overrides?.active) {
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
  const entries = Object.entries(calculations);
  if (entries.length) {
    const [name, calc] = entries[0];
    if (calc.parts?.length) return { name, calc };
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
// DataValue arrays are 1-indexed: index 0 is unused, index 1 = rank 1
function dvIndex(rank) {
  return Math.max(1, rank || 1);
}

export function evaluateCalc(calc, rank, attacker, charLevel) {
  let total = 0;
  for (const part of calc.parts) {
    switch (part.kind) {
      case 'dataValue': {
        const arr = attacker.spellDataValues?.[part.name];
        if (arr) total += arr[dvIndex(rank)] ?? 0;
        break;
      }
      case 'statByDataValue': {
        const arr = attacker.spellDataValues?.[part.name];
        const ratio = arr ? arr[dvIndex(rank)] ?? 0 : 0;
        total += statValue(attacker, part.stat) * ratio;
        break;
      }
      case 'statByCoefficient': {
        total += statValue(attacker, part.stat) * (part.coefficient || 0);
        break;
      }
      case 'byCharLevel': {
        const idx = Math.max(0, Math.min(charLevel, part.values.length - 1));
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
      case 'statBySubPart': {
        const coeff = evaluateCalc({ parts: [part.subPart] }, rank, attacker, charLevel);
        total += statValue(attacker, part.stat) * coeff;
        break;
      }
      case 'number': total += part.value || 0; break;
      default: break;
    }
  }
  if (calc.multiplier !== undefined) {
    total *= calc.multiplier;
  } else if (calc.multiplierPart) {
    const multVal = evaluateCalc({ parts: [calc.multiplierPart] }, rank, attacker, charLevel);
    total *= multVal;
  }
  return total;
}

// Compute raw damage for one ability cast. When targetCalcName is provided,
// use that specific calculation instead of auto-picking (for multi-cast abilities).
export function computeAbilityDamage(ability, rank, attacker, charLevel, targetCalcName) {
  if (!ability) return null;
  let calc, calcName;
  if (targetCalcName && ability.calculations?.[targetCalcName]) {
    calc = ability.calculations[targetCalcName];
    calcName = targetCalcName;
  } else {
    const picked = pickDamageCalc(ability.calculations);
    if (!picked) return null;
    calc = picked.calc;
    calcName = picked.name;
  }
  // Resolve GameCalculationModified: use the base calc's parts, then apply multiplier
  if (calc.modified && ability.calculations?.[calc.modified]) {
    const baseCalc = ability.calculations[calc.modified];
    const multParts = calc.multiplierParts;
    calc = {
      parts: [...baseCalc.parts],
      multiplierPart: multParts?.[0],
    };
  }
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
  let hitCount = 0; // consecutive hits on target (for Vayne W)
  let empowered = null; // ability key that empowers next AA (for Vayne Q)
  let lockeMarks = 0; // Locke Q mark stacks on target
  const targetMaxHP = (target.hp || 0) + (target.shield || 0);
  let targetCurrentHP = targetMaxHP;

  // Stacking resistance shred / damage amp from items
  let cleaverStacks = 0, cleaverMax = 0, cleaverPer = 0;
  let bloodletterStacks = 0, bloodletterMax = 0, bloodletterPer = 0;
  let abyssalAmp = 0;
  for (const item of items || []) {
    const p = item?._overrides?.passive;
    if (!p) continue;
    if (p.type === 'blackCleaver') { cleaverMax = p.maxStacks; cleaverPer = p.armorReductionPerStack; }
    if (p.type === 'bloodlettersCurse') { bloodletterMax = p.maxStacks; bloodletterPer = p.mrReductionPerStack; }
    if (p.type === 'abyssalMask') { abyssalAmp = p.magicDamageAmp; }
  }

  function effectiveTarget() {
    const armorShred = cleaverStacks * cleaverPer;
    const mrShred = bloodletterStacks * bloodletterPer;
    return {
      ...target,
      armor: (target.armor || 0) * (1 - armorShred),
      spellblock: (target.spellblock || 0) * (1 - mrShred),
    };
  }

  function addDmg(result) {
    let raw = result.raw;
    if (result.type === 'magic' && abyssalAmp > 0) raw *= (1 + abyssalAmp);
    const et = effectiveTarget();
    const post = applyResistance(raw, result.type, et, activeStats);
    if (result.type === 'physical') totalPhys += post;
    else if (result.type === 'true') totalTrue += post;
    else totalMagic += post;
    steps.push({ ...result, raw, post });
    targetCurrentHP = Math.max(0, targetCurrentHP - post);
    // Apply stacking shred after damage
    if (result.type === 'physical' && cleaverMax > 0) {
      cleaverStacks = Math.min(cleaverMax, cleaverStacks + 1);
    }
    if (result.type === 'magic' && bloodletterMax > 0) {
      bloodletterStacks = Math.min(bloodletterMax, bloodletterStacks + 1);
    }
  }

  const champId = champion?.id;
  let activeStats = attackerStats;

  for (const step of combo) {
    if (step === 'AA' || step === 'AA4') {
      const forceIndex = step === 'AA4' ? 3 : aaCount;
      const champCtx = {
        championId: champId,
        ranks,
        hitCount,
        empowered,
      };
      const aaResults = computeAADamage(activeStats, items, forceIndex, targetCurrentHP, targetMaxHP, champCtx);
      aaCount++;
      hitCount++;
      empowered = null; // consumed
      for (const r of aaResults) addDmg(r);
      // Locke mark consumption on AA
      if (champId === 'Locke') {
        const lockeAA = CHAMPION_AA.Locke;
        if (lockeMarks > 0) {
          const qRank = ranks.Q || 1;
          const baseMark = lockeAA.markDamage[qRank] || 0;
          const ratio = lockeAA.markRatio[qRank] || 0;
          let markRaw = baseMark + ratio * (activeStats.ap || 0);
          const bonus = lockeMarks >= 3 ? lockeAA.threeMarkBonus : lockeMarks >= 2 ? lockeAA.twoMarkBonus : 0;
          markRaw *= (1 + bonus);
          markRaw *= lockeMarks;
          addDmg({
            abilityKey: 'AA',
            abilityName: `Ritual Nails (${lockeMarks} mark${lockeMarks > 1 ? 's' : ''})`,
            raw: markRaw,
            type: 'magic',
          });
          lockeMarks = 0;
        }
      }
      if (lastWasSpell) {
        const sb = computeSpellbladeDamage(activeStats, items);
        if (sb) addDmg({ abilityKey: 'AA', abilityName: sb.name + ' Proc', raw: sb.raw, type: sb.type });
      }
      lastWasSpell = false;
      continue;
    }

    // Item active steps (e.g., "ITEM_3152" for Rocketbelt)
    if (step.startsWith('ITEM_')) {
      const itemId = parseInt(step.slice(5));
      const result = computeItemActiveDamage(itemId, activeStats, items);
      if (result) addDmg(result);
      lastWasSpell = true;
      continue;
    }

    // Resolve multi-cast keys (E1, R2, Q1…) to the parent ability + specific calc.
    const baseKey = parentAbilityKey(step);
    const ability = champion.abilities.find((a) => a.key === baseKey);
    if (!ability) continue;
    const rank = baseKey === 'P' ? 1 : (ranks[baseKey] || 1);

    let targetCalcName = null;
    let cast = null;
    if (step !== baseKey) {
      const casts = getMultiCasts(champion.id, baseKey);
      cast = casts?.find((c) => c.castKey === step) || null;
      if (cast) targetCalcName = cast.calcName;
    }
    // Vayne R: buff AD when R is cast
    if (champId === 'Vayne' && baseKey === 'R' && ranks.R > 0) {
      const rBonusAD = [0, 20, 35, 50][ranks.R] || 0;
      activeStats = {
        ...activeStats,
        attackdamage: activeStats.attackdamage + rBonusAD,
        bonusAD: (activeStats.bonusAD || 0) + rBonusAD,
      };
    }

    // Locke P: scales with missing HP, interpolate between base and empowered calcs
    if (champId === 'Locke' && baseKey === 'P' && ability.calculations) {
      const calcEntries = Object.entries(ability.calculations);
      if (calcEntries.length >= 2) {
        const attackerWithSpell = { ...activeStats, spellDataValues: ability.dataValues };
        const rawBase = evaluateCalc(calcEntries[0][1], rank, attackerWithSpell, charLevel);
        const rawEmpowered = evaluateCalc(calcEntries[1][1], rank, attackerWithSpell, charLevel);
        const missingPct = targetMaxHP > 0 ? Math.max(0, 1 - targetCurrentHP / targetMaxHP) : 0;
        const t = Math.min(1, missingPct / 0.7);
        const raw = rawBase + (rawEmpowered - rawBase) * t;
        addDmg({
          abilityKey: 'P',
          abilityName: ability.name,
          raw,
          type: 'magic',
        });
        lastWasSpell = true;
        continue;
      }
    }

    const result = computeAbilityDamage(ability, rank, activeStats, charLevel, targetCalcName);
    if (result && result.raw) {
      if (cast) {
        result.abilityKey = step;
        result.abilityName = `${ability.name} (${cast.label})`;
        if (cast.damageType) result.type = cast.damageType;
        if (cast.multiplier) result.raw *= cast.multiplier;
        // Execute scaling: damage scales linearly from 1× to maxMultiplier× based on missing HP
        if (cast.execute) {
          const missingPct = targetMaxHP > 0 ? Math.max(0, 1 - targetCurrentHP / targetMaxHP) : 0;
          const t = Math.min(1, missingPct / cast.execute.threshold);
          const executeMult = 1 + (cast.execute.maxMultiplier - 1) * t;
          result.raw *= executeMult;
          result.abilityName += ` ${Math.round(executeMult * 100)}%`;
        }
      }
      addDmg(result);
    }

    // Champion-specific ability interactions
    const champAA = CHAMPION_AA[champId];
    if (champAA) {
      // Abilities that apply on-hit stacks (Vayne E applies Silver Bolts)
      if (champAA.appliesStacks?.includes(baseKey)) {
        hitCount++;
        // Check if this hit procs an nthHitPassive
        for (const oh of champAA.onHits || []) {
          if (oh.type === 'nthHitPassive') {
            const wRank = ranks?.[oh.ability] || 0;
            if (wRank > 0 && hitCount % oh.every === 0) {
              const maxHpRatio = oh.maxHpRatio[wRank] || 0;
              const targetHP = targetMaxHP || 2000;
              const raw = Math.max(oh.minDamage[wRank] || 0, targetHP * maxHpRatio);
              addDmg({
                abilityKey: baseKey,
                abilityName: `${oh.label} (${Math.round(maxHpRatio * 100)}% max HP)`,
                raw,
                type: oh.damageType,
              });
            }
          }
        }
      }
      // Track empowered-AA abilities (Vayne Q empowers next AA)
      if (champAA.onHits) {
        for (const oh of champAA.onHits) {
          if (oh.type === 'empoweredAA' && baseKey === oh.triggerStep) {
            empowered = oh.triggerStep;
          }
        }
      }
      // Locke: Q applies marks, E2 consumes marks, R executes with mark scaling
      if (champId === 'Locke') {
        if (baseKey === 'Q') {
          lockeMarks = Math.min(champAA.maxStacks, lockeMarks + 1);
        }
        if (step === 'E2' && lockeMarks > 0) {
          const qRank = ranks.Q || 1;
          const baseMark = champAA.markDamage[qRank] || 0;
          const ratio = champAA.markRatio[qRank] || 0;
          let markRaw = baseMark + ratio * (activeStats.ap || 0);
          const bonus = lockeMarks >= 3 ? champAA.threeMarkBonus : lockeMarks >= 2 ? champAA.twoMarkBonus : 0;
          markRaw *= (1 + bonus);
          markRaw *= lockeMarks;
          addDmg({
            abilityKey: 'E',
            abilityName: `Ritual Nails (${lockeMarks} mark${lockeMarks > 1 ? 's' : ''})`,
            raw: markRaw,
            type: 'magic',
          });
          lockeMarks = 0;
        }
        if (baseKey === 'R' && result && result.raw) {
          const rRank = ranks.R || 1;
          const threshold = champAA.executeThreshold[rRank] + champAA.executePerStack * lockeMarks;
          const hpPct = targetMaxHP > 0 ? targetCurrentHP / targetMaxHP : 1;
          if (hpPct <= threshold) {
            result.abilityName += ` [EXECUTE ${Math.round(threshold * 100)}%]`;
          }
        }
      }
    }

    if (!itemProcsUsed) {
      const procs = computeItemProcs(activeStats, items, target, baseKey);
      for (const r of procs) addDmg(r);
      if (procs.length > 0) itemProcsUsed = true;
    } else if (baseKey === 'R') {
      const procs = computeItemProcs(activeStats, items, target, 'R');
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

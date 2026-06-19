import { evaluateCalc } from './damage.js';

const STAT_DISPLAY = {
  AP: 'AP',
  AD: 'AD',
  BaseAD: 'base AD',
  BonusAD: 'bonus AD',
  BonusHP: 'bonus HP',
  MaxHP: 'max HP',
  HP: 'HP',
};

const TAG_CLASS = {
  magicDamage: 'lol-magic',
  physicalDamage: 'lol-physical',
  trueDamage: 'lol-true',
  status: 'lol-status',
  healing: 'lol-healing',
  shield: 'lol-shield',
  speed: 'lol-speed',
  attention: 'lol-attention',
  passive: 'lol-keyword',
  active: 'lol-keyword',
  OnHit: 'lol-keyword',
  keyword: 'lol-keyword',
  keywordStealth: 'lol-keyword',
  rules: 'lol-rules',
  scaleAP: 'lol-scale',
  scaleAD: 'lol-scale',
  scaleMana: 'lol-scale',
  scaleHealth: 'lol-scale',
  lifeSteal: 'lol-scale',
  rarityGeneric: 'lol-rules',
  mainText: 'lol-main',
  stats: 'lol-stats',
};

function formatValue(value, key) {
  if (value == null || Number.isNaN(value)) return '?';
  if (typeof value !== 'number') return String(value);
  if (/ratio|percent|pct/i.test(key) && value > 0 && value <= 1) {
    return `${Math.round(value * 100)}%`;
  }
  if (Number.isInteger(value)) return String(value);
  return value.toFixed(1).replace(/\.0$/, '');
}

function dataValueIndex(rank) {
  return Math.max(1, rank || 1);
}

function lookupDataValue(ability, key, rank) {
  const idx = dataValueIndex(rank);
  for (const [name, values] of Object.entries(ability.dataValues || {})) {
    if (name.toLowerCase() !== key.toLowerCase()) continue;
    const v = values[idx] ?? values[values.length - 1];
    return formatValue(v, key);
  }
  return null;
}

function lookupCalculation(ability, key, rank, attacker, charLevel) {
  const idx = dataValueIndex(rank);
  for (const [name, calc] of Object.entries(ability.calculations || {})) {
    if (name.toLowerCase() !== key.toLowerCase()) continue;
    const attackerWithSpell = { ...attacker, spellDataValues: ability.dataValues };
    const v = evaluateCalc(calc, idx, attackerWithSpell, charLevel);
    return formatValue(Math.round(v), key);
  }
  return null;
}

function lookupEffect(ability, key, rank) {
  const m = key.match(/^e(\d+)$/i);
  if (!m || !ability.effect) return null;
  const idx = dataValueIndex(rank) - 1;
  const arr = ability.effect[parseInt(m[1], 10)];
  if (!Array.isArray(arr)) return null;
  return formatValue(arr[idx] ?? arr[arr.length - 1], key);
}

function lookupStandard(ability, key, rank) {
  const idx = dataValueIndex(rank) - 1;
  const k = key.toLowerCase();
  if (k === 'cooldown' && ability.cooldown) {
    return formatValue(ability.cooldown[idx] ?? ability.cooldown.at(-1), key);
  }
  if (k === 'cost' && ability.cost) {
    return formatValue(ability.cost[idx] ?? ability.cost.at(-1), key);
  }
  if (k === 'range' && ability.range) {
    const r = ability.range[idx] ?? ability.range.at(-1);
    return r === 0 ? 'Global' : formatValue(r, key);
  }
  return null;
}

export function formatAbilityTooltip(ability, rank, attacker, charLevel) {
  const text = ability.tooltip || ability.description || '';
  if (!text) return '';

  return text.replace(/\{\{\s*([^}]+?)\s*\}\}/g, (_, rawKey) => {
    const key = rawKey.trim();
    if (key === 'spellmodifierdescriptionappend') return '';

    const resolved =
      lookupDataValue(ability, key, rank) ??
      lookupCalculation(ability, key, rank, attacker, charLevel) ??
      lookupEffect(ability, key, rank) ??
      lookupStandard(ability, key, rank);

    return resolved ?? `{{${key}}}`;
  });
}

export function lolHtmlToSafeHtml(html) {
  if (!html) return '';

  let out = html
    .replace(/\{\{\s*spellmodifierdescriptionappend\s*\}\}/gi, '')
    .replace(/<font[^>]*>/gi, '')
    .replace(/<\/font>/gi, '');

  for (const [tag, className] of Object.entries(TAG_CLASS)) {
    const open = new RegExp(`<${tag}(\\s[^>]*)?>`, 'gi');
    const close = new RegExp(`</${tag}>`, 'gi');
    out = out.replace(open, `<span class="${className}">`).replace(close, '</span>');
  }

  out = out.replace(/<\/?li>/gi, '<br>');
  out = out.replace(/<(?!\/?(br|span)\b)[^>]+>/gi, '');

  return out;
}

export function formatItemTooltip(item) {
  return lolHtmlToSafeHtml(item.description || item.name || '');
}

function evaluateSubPart(part, dataValues, rank, charLevel) {
  if (!part) return 0;
  const idx = dataValueIndex(rank);
  const lvl = charLevel || 1;
  switch (part.kind) {
    case 'dataValue': {
      const arr = dataValues?.[part.name];
      return arr ? (Array.isArray(arr) ? (arr[idx] ?? arr[arr.length - 1]) : arr) : 0;
    }
    case 'byCharLevel': {
      const vidx = Math.max(0, Math.min(lvl - 1, (part.values?.length || 1) - 1));
      return part.values?.[vidx] ?? 0;
    }
    case 'byCharLevelBreakpoints': {
      let val = part.baseValue || 0;
      if (part.initialPerLevel) val += part.initialPerLevel * (lvl - 1);
      if (part.breakpoints) {
        for (const bp of part.breakpoints) {
          if (lvl >= (bp.mLevel || 1)) {
            const perLevel = bp['{57fdc438}'] || bp.mPerLevel || bp.mBonusPerLevelAtAndAfter || 0;
            val += perLevel * (lvl - (bp.mLevel || 1));
            if (bp.mAdditionalBonusAtThisLevel) val += bp.mAdditionalBonusAtThisLevel;
          }
        }
      }
      return val;
    }
    case 'byCharLevelInterp': {
      const t = Math.max(0, Math.min(1, (lvl - 1) / 17));
      return (part.start || 0) + ((part.end || 0) - (part.start || 0)) * t;
    }
    case 'number':
      return part.value || 0;
    default:
      return 0;
  }
}

function describeCalcParts(calc, dataValues, rank, attacker, charLevel) {
  if (!calc?.parts?.length) return null;
  const idx = dataValueIndex(rank);
  const segments = [];

  for (const part of calc.parts) {
    switch (part.kind) {
      case 'dataValue': {
        const arr = dataValues?.[part.name];
        const val = arr ? (Array.isArray(arr) ? (arr[idx] ?? arr[arr.length - 1]) : arr) : 0;
        if (val === 0) continue;
        const isPercent = /percent|ratio|hpdamage/i.test(part.name) || (val > 0 && val < 1);
        const hpBased = /hp|health/i.test(part.name);
        segments.push({
          flat: isPercent ? null : val,
          pct: isPercent ? val : null,
          label: hpBased ? 'max HP' : null,
        });
        break;
      }
      case 'statByDataValue': {
        const arr = dataValues?.[part.name];
        const ratio = arr ? (Array.isArray(arr) ? (arr[idx] ?? arr[arr.length - 1]) : arr) : 0;
        if (ratio === 0) continue;
        const statLabel = STAT_DISPLAY[part.stat] || part.stat;
        const isPercent = ratio > 0 && ratio <= 5;
        segments.push({
          flat: null,
          pct: isPercent ? ratio : null,
          label: statLabel,
        });
        break;
      }
      case 'statByCoefficient': {
        if (!part.coefficient) continue;
        const statLabel = STAT_DISPLAY[part.stat] || part.stat;
        segments.push({
          flat: null,
          pct: part.coefficient,
          label: statLabel,
        });
        break;
      }
      case 'byCharLevel': {
        const lvl = charLevel || 1;
        const vidx = Math.max(0, Math.min(lvl - 1, (part.values?.length || 1) - 1));
        const val = part.values?.[vidx] ?? 0;
        if (val !== 0) segments.push({ flat: val, pct: null, label: null });
        break;
      }
      case 'byCharLevelInterp': {
        const t = Math.max(0, Math.min(1, ((charLevel || 1) - 1) / 17));
        const val = (part.start || 0) + ((part.end || 0) - (part.start || 0)) * t;
        segments.push({ flat: val, pct: null, label: null });
        break;
      }
      case 'byCharLevelBreakpoints': {
        let val = part.baseValue || 0;
        const lvl = charLevel || 1;
        if (part.initialPerLevel) val += part.initialPerLevel * (lvl - 1);
        if (part.breakpoints) {
          for (const bp of part.breakpoints) {
            if (lvl >= (bp.mLevel || 1)) {
              const perLevel = bp['{57fdc438}'] || bp.mPerLevel || bp.mBonusPerLevelAtAndAfter || 0;
              val += perLevel * (lvl - (bp.mLevel || 1));
              if (bp.mAdditionalBonusAtThisLevel) val += bp.mAdditionalBonusAtThisLevel;
            }
          }
        }
        segments.push({ flat: val, pct: null, label: null });
        break;
      }
      case 'statBySubPart': {
        const subVal = evaluateSubPart(part.subPart, dataValues, rank, charLevel);
        if (subVal === 0) continue;
        const statLabel = STAT_DISPLAY[part.stat] || part.stat;
        segments.push({ flat: null, pct: subVal, label: statLabel });
        break;
      }
      case 'number': {
        if (part.value) segments.push({ flat: part.value, pct: null, label: null });
        break;
      }
    }
  }

  if (!segments.length) return null;

  // Apply multiplier (e.g., Akali E1=30%, E2=70% of total damage)
  let mult = null;
  if (calc.multiplier != null) {
    mult = calc.multiplier;
  } else if (calc.multiplierPart) {
    mult = evaluateSubPart(calc.multiplierPart, dataValues, rank, charLevel);
  }
  if (mult != null && mult !== 1) {
    for (const s of segments) {
      if (s.flat != null) s.flat *= mult;
      if (s.pct != null) s.pct *= mult;
    }
  }

  const pieces = [];
  for (const s of segments) {
    if (s.flat != null) {
      pieces.push(Math.round(s.flat));
    } else if (s.pct != null) {
      const pctVal = s.pct * 100;
      const pctStr = pctVal < 1 && pctVal > 0
        ? pctVal.toFixed(2).replace(/0+$/, '').replace(/\.$/, '') + '%'
        : Math.round(pctVal) + '%';
      pieces.push(s.label ? `${pctStr} ${s.label}` : pctStr);
    }
  }
  return pieces.join(' + ');
}

function humanizeCalcName(name) {
  return name
    .replace(/([a-z])([A-Z])/g, '$1 $2')
    .replace(/^./, (c) => c.toUpperCase())
    .replace(/Calc$/, '')
    .trim();
}

export function formatPassiveDamageTooltip(ability, attacker, charLevel) {
  return formatDamageTooltip(ability, 1, attacker, charLevel);
}

export function formatDamageTooltip(ability, rank, attacker, charLevel) {
  const calcs = ability.calculations;
  const dv = ability.dataValues;
  if (!calcs || !Object.keys(calcs).length) return '';

  const lines = [];
  for (const [name, calc] of Object.entries(calcs)) {
    if (/cooldown|duration|speed|move|heal|shield|range|radius|size|cost|mana/i.test(name)) continue;
    if (name.startsWith('{')) continue;
    const desc = describeCalcParts(calc, dv, rank, attacker, charLevel);
    if (!desc) continue;
    const label = humanizeCalcName(name);
    lines.push(`<span class="lol-keyword">${label}</span>: <span class="lol-magic">${desc}</span>`);
  }

  return lines.length ? lines.join('<br>') : '';
}

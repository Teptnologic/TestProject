import { evaluateCalc } from './damage.js';

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

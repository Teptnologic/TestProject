// Compute total champion stats given a build (level + items).
// Uses Riot's quadratic growth formula: stat = base + perLevel * (lvl-1) * (0.685 + 0.0175 * lvl)

const GROWTH_KEYS = [
  ['hp', 'hpperlevel'],
  ['mp', 'mpperlevel'],
  ['armor', 'armorperlevel'],
  ['spellblock', 'spellblockperlevel'],
  ['attackdamage', 'attackdamageperlevel'],
  ['hpregen', 'hpregenperlevel'],
  ['mpregen', 'mpregenperlevel'],
  ['crit', 'critperlevel'],
];

function growth(level) {
  if (level <= 1) return 0;
  return (level - 1) * (0.685 + 0.0175 * level);
}

export function baseStatsAtLevel(champStats, level) {
  const g = growth(level);
  const out = { level, movespeed: champStats.movespeed, attackrange: champStats.attackrange };
  for (const [stat, perLvl] of GROWTH_KEYS) {
    out[stat] = (champStats[stat] || 0) + (champStats[perLvl] || 0) * g;
  }
  // Attack speed grows additively from a ratio, not the quadratic
  const asGrowth = (champStats.attackspeedperlevel || 0) * g / 100;
  out.attackspeed = (champStats.attackspeed || 0) * (1 + asGrowth);
  return out;
}

// Sum item flat stat modifiers into a champion stat object
const ITEM_STAT_MAP = {
  FlatHPPoolMod: 'hp',
  FlatMPPoolMod: 'mp',
  FlatArmorMod: 'armor',
  FlatSpellBlockMod: 'spellblock',
  FlatPhysicalDamageMod: 'attackdamage',
  FlatMagicDamageMod: 'ap',
  PercentAttackSpeedMod: 'attackspeed_pct',
  FlatCritChanceMod: 'crit',
  FlatMovementSpeedMod: 'movespeed',
  PercentMovementSpeedMod: 'movespeed_pct',
  FlatHPRegenMod: 'hpregen',
};

export function totalStats(champStats, level, items) {
  const base = baseStatsAtLevel(champStats, level);
  const bonus = { hp: 0, mp: 0, armor: 0, spellblock: 0, attackdamage: 0, ap: 0, crit: 0 };
  let asPct = 0;
  for (const item of items || []) {
    if (!item?.stats) continue;
    for (const [riotKey, val] of Object.entries(item.stats)) {
      const mapped = ITEM_STAT_MAP[riotKey];
      if (!mapped) continue;
      if (mapped === 'attackspeed_pct') asPct += val;
      else if (mapped === 'movespeed_pct') { /* skip for now */ }
      else bonus[mapped] = (bonus[mapped] || 0) + val;
    }
  }
  return {
    ...base,
    hp: base.hp + bonus.hp,
    mp: base.mp + bonus.mp,
    armor: base.armor + bonus.armor,
    spellblock: base.spellblock + bonus.spellblock,
    attackdamage: base.attackdamage + bonus.attackdamage,
    bonusAD: bonus.attackdamage,
    ap: bonus.ap,
    crit: base.crit + bonus.crit,
    attackspeed: base.attackspeed * (1 + asPct),
    bonusHP: bonus.hp,
  };
}

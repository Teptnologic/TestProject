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

export function totalStats(champStats, level, items, championId, ranks, adaptiveForce) {
  const base = baseStatsAtLevel(champStats, level);
  const bonus = { hp: 0, mp: 0, armor: 0, spellblock: 0, attackdamage: 0, ap: 0, crit: 0 };
  let asPct = 0;
  let lethality = 0;
  let flatMagicPen = 0;
  let magicPenPct = 0;
  let armorPenPct = 0;
  let abilityHaste = 0;
  let hasRabadons = false;

  for (const item of items || []) {
    if (!item) continue;
    // Data Dragon flat stats
    if (item.stats) {
      for (const [riotKey, val] of Object.entries(item.stats)) {
        const mapped = ITEM_STAT_MAP[riotKey];
        if (!mapped) continue;
        if (mapped === 'attackspeed_pct') asPct += val;
        else if (mapped === 'movespeed_pct') { /* skip for now */ }
        else bonus[mapped] = (bonus[mapped] || 0) + val;
      }
    }
    // Bin data: direct stats from CDragon (preferred, auto-updated)
    const ds = item.bin?.directStats;
    if (ds) {
      lethality += ds.lethality || 0;
      flatMagicPen += ds.flatMagicPen || 0;
      magicPenPct += ds.pctMagicPen || 0;
      armorPenPct += ds.pctArmorPen || ds.pctBonusArmorPen || 0;
      abilityHaste += ds.abilityHaste || 0;
    }
    // Fallback: manual overrides for pen, haste, passives
    const ov = item._overrides;
    if (ov) {
      if (!ds) {
        lethality += ov.lethality || 0;
        flatMagicPen += ov.flatMagicPen || 0;
        magicPenPct += ov.pctMagicPen || ov.magicPenPct || 0;
        armorPenPct += ov.pctArmorPen || ov.armorPenPct || 0;
        abilityHaste += ov.abilityHaste || 0;
      }
      if (ov.passive?.type === 'rabadons') hasRabadons = true;
    }
  }

  // Adaptive force from rune shards: each shard = +9 AD or +14 AP
  const afShards = adaptiveForce || 0;
  if (afShards > 0) {
    const isAP = bonus.ap > bonus.attackdamage;
    if (isAP) bonus.ap += afShards * 14;
    else bonus.attackdamage += afShards * 9;
  }

  let ap = bonus.ap;
  if (hasRabadons) ap = Math.floor(ap * 1.35);

  const finalAS = base.attackspeed * (1 + asPct);
  const totalCrit = base.crit + bonus.crit;
  let bonusAD = bonus.attackdamage;

  // Jhin passive: bonus AD from attack speed, crit, and level
  if (championId === 'Jhin') {
    let levelPct = 0.04;
    const lvl = level || 1;
    if (lvl > 1) levelPct += 0.01 * (Math.min(lvl, 9) - 1);
    if (lvl >= 10) levelPct += 0.02 * (Math.min(lvl, 11) - 9);
    if (lvl >= 12) levelPct += 0.04 * (lvl - 11);
    const fromCrit = 0.35 * totalCrit;
    const fromAS = 0.30 * asPct;
    const jhinBonusPct = levelPct + fromCrit + fromAS;
    const jhinBonusAD = (base.attackdamage + bonusAD) * jhinBonusPct;
    bonusAD += jhinBonusAD;
  }

  return {
    ...base,
    hp: base.hp + bonus.hp,
    mp: base.mp + bonus.mp,
    armor: base.armor + bonus.armor,
    spellblock: base.spellblock + bonus.spellblock,
    attackdamage: base.attackdamage + bonusAD,
    baseAD: base.attackdamage,
    bonusAD,
    ap,
    crit: totalCrit,
    attackspeed: finalAS,
    bonusHP: bonus.hp,
    lethality,
    flatMagicPen,
    magicPenPct,
    armorPenPct,
    abilityHaste,
  };
}

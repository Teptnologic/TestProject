// Static definitions for abilities with multiple casts that deal different damage.
// Each entry maps a champion + ability key to an array of casts. Each cast gets
// its own combo button and resolves to a specific calculation from the spell's
// mSpellCalculations. Abilities not listed here keep the default single-button
// behavior where pickDamageCalc chooses the best calc automatically.

export const MULTI_CAST = {
  Akali: {
    E: [
      { castKey: 'E1', label: 'E1', name: 'Shuriken Flip', calcName: 'E1Damage' },
      { castKey: 'E2', label: 'E2', name: 'Shuriken Dash', calcName: 'E2DamageCalc' },
    ],
    R: [
      { castKey: 'R1', label: 'R1', name: 'R1: Perfect Execution', calcName: 'Cast1Damage' },
      { castKey: 'R2', label: 'R2', name: 'R2: Perfect Execution', calcName: 'Cast2DamageMax' },
    ],
  },
  Evelynn: {
    Q: [
      { castKey: 'Q1', label: 'Q1', name: 'Hate Spike', calcName: 'MissileDamage' },
      { castKey: 'Q2', label: 'Q2', name: 'Hate Spike (spikes)', calcName: 'TotalBonusDamage' },
    ],
  },
};

// Given a champion id and ability key, return the casts array or null.
export function getMultiCasts(championId, abilityKey) {
  return MULTI_CAST[championId]?.[abilityKey] || null;
}

// Given a cast key like 'E1' or 'R2', return the parent ability key ('E', 'R').
export function parentAbilityKey(castKey) {
  const m = castKey.match(/^([PQWER])(\d)$/);
  return m ? m[1] : castKey;
}

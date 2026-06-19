export const COMBO_TEMPLATES = {
  Akali: [
    { name: 'Short Trade', keys: ['Q', 'P', 'AA', 'W'] },
    { name: 'All-In', keys: ['R1', 'P', 'E1', 'Q', 'P', 'AA', 'E2', 'R2', 'Q', 'P'] },
    { name: 'E Engage', keys: ['E1', 'Q', 'P', 'AA', 'E2', 'R1', 'Q', 'P', 'R2'] },
    { name: 'Burst', keys: ['R1', 'P', 'Q', 'P', 'AA', 'R2'] },
    { name: 'Poke', keys: ['Q', 'P', 'AA'] },
  ],
  Lux: [
    { name: 'Full Combo', keys: ['Q', 'AA', 'P', 'E', 'R', 'AA', 'P'] },
    { name: 'Poke', keys: ['E', 'AA', 'P'] },
    { name: 'Snare Burst', keys: ['Q', 'AA', 'P', 'E', 'AA', 'P'] },
    { name: 'One-Shot', keys: ['Q', 'E', 'R', 'AA', 'P'] },
  ],
  Ahri: [
    { name: 'Full Combo', keys: ['E', 'W', 'Q1', 'Q2', 'R', 'R', 'R', 'AA'] },
    { name: 'Charm Burst', keys: ['E', 'Q1', 'Q2', 'W'] },
    { name: 'Poke', keys: ['Q1', 'Q2'] },
    { name: 'All-In', keys: ['R', 'E', 'W', 'Q1', 'Q2', 'R', 'AA', 'R'] },
    { name: 'Quick Trade', keys: ['E', 'Q1', 'Q2', 'W', 'AA'] },
  ],
  Yone: [
    { name: 'Short Trade', keys: ['Q', 'AA', 'Q', 'AA'] },
    { name: 'All-In', keys: ['E', 'Q', 'AA', 'Q', 'AA', 'R', 'Q', 'AA'] },
    { name: 'Knockup Combo', keys: ['Q', 'AA', 'Q', 'AA', 'Q', 'AA'] },
    { name: 'Ult Engage', keys: ['E', 'R', 'Q', 'AA', 'W', 'Q', 'AA'] },
    { name: 'Quick Trade', keys: ['E', 'Q', 'AA', 'W'] },
  ],
  Katarina: [
    { name: 'Full Combo', keys: ['E', 'W', 'P', 'Q', 'E', 'P', 'R'] },
    { name: 'Dagger Burst', keys: ['Q', 'E', 'P', 'W', 'P', 'AA', 'E'] },
    { name: 'Quick Trade', keys: ['Q', 'E', 'P', 'AA'] },
    { name: 'All-In', keys: ['E', 'W', 'P', 'Q', 'E', 'P', 'R'] },
    { name: 'Poke', keys: ['Q'] },
  ],
  Vayne: [
    { name: '3-Hit Trade', keys: ['AA', 'AA', 'Q'] },
    { name: 'Condemn Wall', keys: ['AA', 'AA', 'E'] },
    { name: 'Extended Trade', keys: ['AA', 'Q', 'AA', 'AA', 'AA'] },
    { name: 'All-In', keys: ['R', 'Q', 'AA', 'AA', 'AA', 'E', 'AA', 'Q', 'AA'] },
    { name: 'Short Trade', keys: ['AA', 'Q', 'AA'] },
  ],
  Jhin: [
    { name: 'Poke', keys: ['Q', 'AA'] },
    { name: 'Root Combo', keys: ['W', 'Q', 'AA'] },
    { name: 'Trap Burst', keys: ['E', 'W', 'Q', 'AA'] },
    { name: 'Full Combo', keys: ['W', 'Q', 'AA', 'AA', 'AA', 'AA'] },
    { name: '4th Shot Trade', keys: ['AA', 'AA', 'AA', 'Q', 'AA'] },
  ],
};

const STORAGE_KEY = 'lol-dmg-calc-custom-combos';

export function getCustomCombos(championId) {
  try {
    const all = JSON.parse(localStorage.getItem(STORAGE_KEY) || '{}');
    return all[championId] || [];
  } catch {
    return [];
  }
}

export function saveCustomCombo(championId, name, keys) {
  try {
    const all = JSON.parse(localStorage.getItem(STORAGE_KEY) || '{}');
    if (!all[championId]) all[championId] = [];
    all[championId].push({ name, keys });
    localStorage.setItem(STORAGE_KEY, JSON.stringify(all));
  } catch { /* noop */ }
}

export function deleteCustomCombo(championId, index) {
  try {
    const all = JSON.parse(localStorage.getItem(STORAGE_KEY) || '{}');
    if (all[championId]) {
      all[championId].splice(index, 1);
      localStorage.setItem(STORAGE_KEY, JSON.stringify(all));
    }
  } catch { /* noop */ }
}

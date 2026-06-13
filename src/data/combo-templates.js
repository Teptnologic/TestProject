export const COMBO_TEMPLATES = {
  Akali: [
    { name: 'Short Trade', keys: ['Q', 'AA', 'W'] },
    { name: 'All-In', keys: ['R', 'E', 'Q', 'AA', 'E', 'R', 'Q'] },
    { name: 'E Engage', keys: ['E', 'Q', 'AA', 'E', 'R', 'Q', 'R'] },
    { name: 'Burst', keys: ['R', 'Q', 'AA', 'R'] },
    { name: 'Poke', keys: ['Q', 'P', 'AA'] },
  ],
  Lux: [
    { name: 'Full Combo', keys: ['Q', 'E', 'R', 'AA'] },
    { name: 'Poke', keys: ['E', 'AA'] },
    { name: 'Snare Burst', keys: ['Q', 'AA', 'E', 'AA'] },
    { name: 'One-Shot', keys: ['Q', 'E', 'R'] },
  ],
  Ahri: [
    { name: 'Full Combo', keys: ['E', 'W', 'Q', 'R', 'AA'] },
    { name: 'Charm Burst', keys: ['E', 'Q', 'W'] },
    { name: 'Poke', keys: ['Q'] },
    { name: 'All-In', keys: ['R', 'E', 'W', 'Q', 'R', 'AA', 'R'] },
    { name: 'Quick Trade', keys: ['E', 'Q', 'W', 'AA'] },
  ],
  Yone: [
    { name: 'Short Trade', keys: ['Q', 'AA', 'Q', 'AA'] },
    { name: 'All-In', keys: ['E', 'Q', 'AA', 'Q', 'AA', 'R', 'Q', 'AA'] },
    { name: 'Knockup Combo', keys: ['Q', 'AA', 'Q', 'AA', 'Q', 'AA'] },
    { name: 'Ult Engage', keys: ['E', 'R', 'Q', 'AA', 'W', 'Q', 'AA'] },
    { name: 'Quick Trade', keys: ['E', 'Q', 'AA', 'W'] },
  ],
  Katarina: [
    { name: 'Full Combo', keys: ['E', 'W', 'Q', 'R'] },
    { name: 'Dagger Burst', keys: ['Q', 'E', 'W', 'AA', 'E'] },
    { name: 'Quick Trade', keys: ['Q', 'E', 'AA'] },
    { name: 'All-In', keys: ['E', 'W', 'Q', 'E', 'R'] },
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

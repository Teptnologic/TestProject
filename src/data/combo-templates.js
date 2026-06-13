export const COMBO_TEMPLATES = {
  Akali: [
    { name: 'Short Trade', keys: ['Q', 'AA', 'W'] },
    { name: 'All-In', keys: ['R', 'E', 'Q', 'AA', 'E', 'R', 'Q'] },
    { name: 'E Engage', keys: ['E', 'Q', 'AA', 'E', 'R', 'Q', 'R'] },
    { name: 'Burst', keys: ['R', 'Q', 'AA', 'R'] },
    { name: 'Poke', keys: ['Q', 'P', 'AA'] },
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

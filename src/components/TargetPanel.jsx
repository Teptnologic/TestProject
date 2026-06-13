import { useState, useEffect, useMemo } from 'react';
import { getChampion, championList, getItem } from '../data';
import { ITEM_OVERRIDES } from '../data/item-overrides';
import { totalStats } from '../utils/damage';
import meta from '../data/generated/meta.json';
import ItemSlots from './ItemSlots';

const DDRAGON_IMG = `https://ddragon.leagueoflegends.com/cdn/${meta.version}/img`;

const TARGET_PRESETS = [
  { name: 'Squishy', hp: 2000, armor: 50, spellblock: 35, bonusHP: 0, shield: 0, level: 11 },
  { name: 'ADC', hp: 1800, armor: 60, spellblock: 35, bonusHP: 0, shield: 0, level: 11 },
  { name: 'Bruiser', hp: 2800, armor: 120, spellblock: 60, bonusHP: 800, shield: 0, level: 11 },
  { name: 'Tank', hp: 3500, armor: 250, spellblock: 150, bonusHP: 1500, shield: 0, level: 11 },
];

function resolveTargetItems(itemIds) {
  return itemIds.map((id) => {
    if (!id) return null;
    const item = getItem(id);
    if (!item) return null;
    const overrides = ITEM_OVERRIDES[id];
    return overrides ? { ...item, _overrides: overrides } : item;
  });
}

export default function TargetPanel({ target, setTarget }) {
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [targetChampId, setTargetChampId] = useState(null);
  const [targetLevel, setTargetLevel] = useState(11);
  const [targetItemIds, setTargetItemIds] = useState([null, null, null, null, null, null]);
  const [champQuery, setChampQuery] = useState('');
  const [dropdownOpen, setDropdownOpen] = useState(false);

  const targetChamp = targetChampId ? getChampion(targetChampId) : null;
  const targetItems = useMemo(() => resolveTargetItems(targetItemIds), [targetItemIds]);

  useEffect(() => {
    if (!targetChampId) return;
    const champ = getChampion(targetChampId);
    if (!champ) return;
    const items = resolveTargetItems(targetItemIds);
    const stats = totalStats(champ.stats, targetLevel, items);
    setTarget({
      hp: Math.round(stats.hp),
      bonusHP: Math.round(stats.bonusHP),
      armor: Math.round(stats.armor),
      spellblock: Math.round(stats.spellblock),
      shield: 0,
      level: targetLevel,
    });
  }, [targetChampId, targetLevel, targetItemIds, setTarget]);

  function applyPreset(preset) {
    setTarget({ ...preset });
    setTargetChampId(null);
  }

  function update(field, value) {
    setTarget((t) => ({ ...t, [field]: Number(value) || 0 }));
  }

  function selectChamp(c) {
    setTargetChampId(c.id);
    setChampQuery('');
    setDropdownOpen(false);
  }

  function clearChamp() {
    setTargetChampId(null);
    setTargetItemIds([null, null, null, null, null, null]);
  }

  const filteredChamps = useMemo(() => {
    if (!champQuery.trim()) return [];
    const q = champQuery.toLowerCase();
    return championList.filter((c) => c.name.toLowerCase().includes(q)).slice(0, 8);
  }, [champQuery]);

  const targetBuildForSlots = {
    items: targetItems,
    champion: targetChamp,
  };

  function setTargetBuild(updater) {
    if (typeof updater === 'function') {
      const fakeBuild = { items: targetItemIds };
      const result = updater(fakeBuild);
      if (result.items) setTargetItemIds(result.items);
    }
  }

  return (
    <div className="card">
      <div className="card-header">
        <h2>Target</h2>
        <span style={{ fontSize: '12px', color: '#5a6270' }}>
          {targetChamp ? targetChamp.name : 'Manual dummy'}
        </span>
      </div>
      <div className="card-body">
        <div className="target-presets">
          {TARGET_PRESETS.map((p) => (
            <button key={p.name} className="target-preset-btn" onClick={() => applyPreset(p)}>
              {p.name}
            </button>
          ))}
        </div>

        <div className="dummy-stats">
          <div className="dummy-stat-group">
            <label>Health</label>
            <input type="number" value={target.hp} onChange={(e) => update('hp', e.target.value)} />
          </div>
          <div className="dummy-stat-group">
            <label>Armor</label>
            <input type="number" value={target.armor} onChange={(e) => update('armor', e.target.value)} />
          </div>
          <div className="dummy-stat-group">
            <label>Magic Resist</label>
            <input type="number" value={target.spellblock} onChange={(e) => update('spellblock', e.target.value)} />
          </div>
        </div>
        <div className="dummy-stats" style={{ marginTop: 10 }}>
          <div className="dummy-stat-group">
            <label>Shield</label>
            <input type="number" value={target.shield} onChange={(e) => update('shield', e.target.value)} />
          </div>
          <div className="dummy-stat-group">
            <label>Level</label>
            <input type="number" min="1" max="18" value={target.level} onChange={(e) => update('level', e.target.value)} />
          </div>
          <div className="dummy-stat-group">
            <label>Bonus HP</label>
            <input type="number" value={target.bonusHP} onChange={(e) => update('bonusHP', e.target.value)} />
          </div>
        </div>

        <button
          className="advanced-toggle"
          onClick={() => setShowAdvanced(!showAdvanced)}
        >
          {showAdvanced ? '▾' : '▸'} Advanced: Use Champion
        </button>

        {showAdvanced && (
          <div className="target-advanced">
            <div className="target-champ-picker">
              {targetChamp ? (
                <div className="target-champ-selected">
                  <img
                    src={`${DDRAGON_IMG}/champion/${targetChampId}.png`}
                    alt={targetChamp.name}
                    className="target-champ-icon"
                  />
                  <div className="target-champ-info">
                    <span className="target-champ-name">{targetChamp.name}</span>
                    <div className="target-level-control">
                      <label>Lvl</label>
                      <input
                        type="range"
                        min="1"
                        max="18"
                        value={targetLevel}
                        onChange={(e) => setTargetLevel(parseInt(e.target.value))}
                        className="level-slider"
                      />
                      <span className="level-value">{targetLevel}</span>
                    </div>
                  </div>
                  <button className="target-champ-clear" onClick={clearChamp}>×</button>
                </div>
              ) : (
                <div className="target-champ-search-wrapper">
                  <input
                    className="champ-search"
                    placeholder="Search target champion..."
                    value={champQuery}
                    onChange={(e) => { setChampQuery(e.target.value); setDropdownOpen(true); }}
                    onFocus={() => setDropdownOpen(true)}
                    onBlur={() => setTimeout(() => setDropdownOpen(false), 150)}
                  />
                  {dropdownOpen && filteredChamps.length > 0 && (
                    <div className="champ-dropdown">
                      {filteredChamps.map((c) => (
                        <div key={c.id} className="champ-dropdown-item" onMouseDown={() => selectChamp(c)}>
                          {c.name}
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              )}
            </div>

            {targetChamp && (
              <>
                <div className="section-label" style={{ marginTop: 10, marginBottom: 6 }}>Target Items</div>
                <ItemSlots build={targetBuildForSlots} setBuild={setTargetBuild} />
              </>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

import { useState, useMemo } from 'react';
import { itemList, getItem } from '../data';
import meta from '../data/generated/meta.json';

const DDRAGON_IMG = `https://ddragon.leagueoflegends.com/cdn/${meta.version}/img`;

const STAT_LABELS = {
  FlatMagicDamageMod: 'AP',
  FlatPhysicalDamageMod: 'AD',
  FlatHPPoolMod: 'HP',
  FlatArmorMod: 'Armor',
  FlatSpellBlockMod: 'MR',
  PercentAttackSpeedMod: '% AS',
  FlatCritChanceMod: '% Crit',
};

function summarizeStats(stats) {
  if (!stats) return '';
  return Object.entries(stats)
    .map(([k, v]) => {
      const label = STAT_LABELS[k];
      if (!label) return null;
      if (k.startsWith('Percent')) return `+${Math.round(v * 100)}${label}`;
      return `+${v} ${label}`;
    })
    .filter(Boolean)
    .join(', ');
}

export default function ItemSlots({ build, setBuild }) {
  const [openSlot, setOpenSlot] = useState(null);
  const [query, setQuery] = useState('');

  const slots = build.items;

  const filtered = useMemo(() => {
    const q = query.toLowerCase().trim();
    let list = itemList;
    if (q) list = list.filter((i) => i.name?.toLowerCase().includes(q));
    return list.slice(0, 60);
  }, [query]);

  function pickItem(item) {
    setBuild((b) => {
      const items = [...b.items];
      items[openSlot] = item.id;
      return { ...b, items };
    });
    setOpenSlot(null);
    setQuery('');
  }

  function clearSlot(idx, e) {
    e.stopPropagation();
    setBuild((b) => {
      const items = [...b.items];
      items[idx] = null;
      return { ...b, items };
    });
  }

  return (
    <>
      <div className="items-grid">
        {slots.map((id, idx) => {
          const item = id ? getItem(id) : null;
          return (
            <div
              key={idx}
              className={`item-slot ${item ? 'filled' : ''}`}
              onClick={() => setOpenSlot(idx)}
              title={item?.name}
            >
              {item ? (
                <>
                  <img
                    src={`${DDRAGON_IMG}/item/${item.id}.png`}
                    alt={item.name}
                    style={{ width: '100%', height: '100%', objectFit: 'cover', borderRadius: 4 }}
                    onError={(e) => { e.currentTarget.style.display = 'none'; }}
                  />
                  <button className="remove" onClick={(e) => clearSlot(idx, e)}>×</button>
                </>
              ) : (
                <span className="plus">+</span>
              )}
            </div>
          );
        })}
      </div>

      {openSlot !== null && (
        <div className="item-search-modal" onClick={() => setOpenSlot(null)}>
          <div className="item-search-box" onClick={(e) => e.stopPropagation()}>
            <input
              className="item-search-input"
              placeholder="Search items..."
              autoFocus
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
            <div className="item-results">
              {filtered.map((item) => (
                <div key={item.id} className="item-result" onClick={() => pickItem(item)}>
                  <span>{item.name}</span>
                  <span className="stats">{summarizeStats(item.stats)}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}
    </>
  );
}

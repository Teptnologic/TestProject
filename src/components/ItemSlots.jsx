import { useState, useMemo } from 'react';
import { itemList, getItem } from '../data';
import meta from '../data/generated/meta.json';
import GameTooltip from './GameTooltip';
import { formatItemTooltip } from '../utils/tooltip';
import { CHAMPION_ITEMS } from '../data/item-recommendations';

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

function getRecommendedItems(championId) {
  if (!championId) return [];
  const itemIds = CHAMPION_ITEMS[championId];
  if (!itemIds) return [];
  return itemIds.map((id) => getItem(id)).filter(Boolean);
}

export default function ItemSlots({ build, setBuild }) {
  const [openSlot, setOpenSlot] = useState(null);
  const [query, setQuery] = useState('');

  const slots = build.items.map((item) => {
    if (!item) return null;
    if (typeof item === 'number') return item;
    return item.id;
  });

  const championId = build.champion?.id;

  const recommended = useMemo(() => getRecommendedItems(championId), [championId]);

  const filtered = useMemo(() => {
    const q = query.toLowerCase().trim();
    if (!q) return [];
    return itemList.filter((i) => i.name?.toLowerCase().includes(q)).slice(0, 60);
  }, [query]);

  const showRecommended = !query.trim() && recommended.length > 0;

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

  function renderItemRow(item) {
    return (
      <div key={item.id} className="item-result" onClick={() => pickItem(item)}>
        <span style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <img
            src={`${DDRAGON_IMG}/item/${item.id}.png`}
            alt=""
            style={{ width: 24, height: 24, borderRadius: 3 }}
            onError={(e) => { e.currentTarget.style.display = 'none'; }}
          />
          {item.name}
        </span>
        <span className="stats">{item.gold}g — {summarizeStats(item.stats)}</span>
      </div>
    );
  }

  return (
    <>
      <div className="items-grid-header">
        {slots.some(Boolean) && (
          <button
            className="clear-items-btn"
            onClick={() => setBuild((b) => ({ ...b, items: Array(b.items.length).fill(null) }))}
          >
            Clear All
          </button>
        )}
      </div>
      <div className="items-grid">
        {slots.map((id, idx) => {
          const item = id ? getItem(id) : null;
          const slotInner = item ? (
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
          );

          return (
            <div
              key={idx}
              className={`item-slot ${item ? 'filled' : ''}`}
              onClick={() => setOpenSlot(idx)}
            >
              {item ? (
                <GameTooltip
                  title={item.name}
                  subtitle={item.gold ? `${item.gold} gold` : null}
                  html={formatItemTooltip(item)}
                >
                  {slotInner}
                </GameTooltip>
              ) : (
                slotInner
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
              {showRecommended && (
                <>
                  <div className="item-section-label">Recommended</div>
                  {recommended.map(renderItemRow)}
                </>
              )}
              {!showRecommended && filtered.length === 0 && query.trim() && (
                <div className="item-no-results">No items found</div>
              )}
              {filtered.length > 0 && filtered.map(renderItemRow)}
            </div>
          </div>
        </div>
      )}
    </>
  );
}

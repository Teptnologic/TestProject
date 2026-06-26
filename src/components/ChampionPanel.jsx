import { useState, useMemo } from 'react';
import { championList } from '../data';
import { COMBO_TEMPLATES } from '../data/combo-templates';
import meta from '../data/generated/meta.json';
import AbilityRow from './AbilityRow';
import ItemSlots from './ItemSlots';
import StatsDisplay from './StatsDisplay';

const DDRAGON_IMG = `https://ddragon.leagueoflegends.com/cdn/${meta.version}/img`;
const SUPPORTED = new Set(Object.keys(COMBO_TEMPLATES));

export default function ChampionPanel({ build, setBuild, stats, setCombo }) {
  const [query, setQuery] = useState('');
  const [open, setOpen] = useState(false);

  const filtered = useMemo(() => {
    if (!query.trim()) return [];
    const q = query.toLowerCase();
    return championList
      .filter((c) => SUPPORTED.has(c.id) && c.name.toLowerCase().includes(q))
      .slice(0, 8);
  }, [query]);

  function selectChamp(c) {
    setBuild((b) => ({ ...b, championId: c.id }));
    setQuery('');
    setOpen(false);
  }

  const portrait = build.champion ? (
    <img src={`${DDRAGON_IMG}/champion/${build.championId}.png`} alt={build.champion.name} />
  ) : (
    <span className="placeholder">Click<br />to select</span>
  );

  return (
    <div className="card">
      <div className="card-header">
        <h2>Your Champion</h2>
        {build.champion && (
          <span style={{ fontSize: '12px', color: '#8b92a0' }}>{build.champion.title}</span>
        )}
      </div>
      <div className="card-body">
        <div className="champ-picker">
          <div className="champ-portrait">{portrait}</div>
          <div className="champ-search-wrapper">
            <input
              className="champ-search"
              placeholder={build.champion ? build.champion.name : 'Search champion...'}
              value={query}
              onChange={(e) => { setQuery(e.target.value); setOpen(true); }}
              onFocus={() => setOpen(true)}
              onBlur={() => setTimeout(() => setOpen(false), 150)}
            />
            {open && filtered.length > 0 && (
              <div className="champ-dropdown">
                {filtered.map((c) => (
                  <div key={c.id} className="champ-dropdown-item" onMouseDown={() => selectChamp(c)}>
                    {c.name} <span style={{ color: '#5a6270', fontSize: '11px' }}>— {c.title}</span>
                  </div>
                ))}
              </div>
            )}
            <div className="level-control">
              <label>Level</label>
              <input
                type="range"
                className="level-slider"
                min="1"
                max="18"
                value={build.level}
                onChange={(e) => {
                  const newLevel = parseInt(e.target.value);
                  setBuild((b) => {
                    const ranks = { ...b.ranks };
                    const maxR = newLevel < 6 ? 0 : newLevel < 11 ? 1 : newLevel < 16 ? 2 : 3;
                    const maxBasic = Math.min(5, Math.ceil(newLevel / 2));
                    for (const k of ['Q', 'W', 'E']) {
                      if ((ranks[k] || 0) > maxBasic) ranks[k] = maxBasic;
                    }
                    if ((ranks.R || 0) > maxR) ranks.R = maxR;
                    // Clamp total points to new level
                    let total = ['Q', 'W', 'E', 'R'].reduce((s, k) => s + (ranks[k] || 0), 0);
                    while (total > newLevel) {
                      for (const k of ['R', 'E', 'W', 'Q']) {
                        if (ranks[k] > 0 && total > newLevel) { ranks[k]--; total--; }
                      }
                    }
                    return { ...b, level: newLevel, ranks };
                  });
                }}
                style={{ '--fill': `${((build.level - 1) / 17) * 100}%` }}
              />
              <span className="level-value">{build.level}</span>
            </div>
          </div>
        </div>

        {build.champion && (
          <>
            <div className="section-label">Abilities</div>
            <AbilityRow build={build} setBuild={setBuild} stats={stats} setCombo={setCombo} />

            <div className="section-label">Items</div>
            <ItemSlots build={build} setBuild={setBuild} />

            <div className="section-label">Runes</div>
            <div className="rune-adaptive">
              <span className="rune-adaptive-label">Adaptive Force</span>
              <div className="rune-shard-btns">
                {[0, 1, 2].map((n) => (
                  <button
                    key={n}
                    className={`rune-shard-btn ${(build.adaptiveForce || 0) === n ? 'active' : ''}`}
                    onClick={() => setBuild((b) => ({ ...b, adaptiveForce: n }))}
                  >
                    {n === 0 ? 'Off' : `×${n}`}
                  </button>
                ))}
              </div>
              {(build.adaptiveForce || 0) > 0 && (
                <span className="rune-adaptive-hint">
                  +{stats?.ap > (stats?.bonusAD || 0) ? `${(build.adaptiveForce || 0) * 9} AP` : `${((build.adaptiveForce || 0) * 5.4).toFixed(1)} AD`}
                </span>
              )}
            </div>

            <div className="section-label">Stats</div>
            <StatsDisplay stats={stats} />
          </>
        )}
      </div>
    </div>
  );
}

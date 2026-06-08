import { useState, useMemo } from 'react';
import { championList } from '../data';
import meta from '../data/generated/meta.json';
import AbilityRow from './AbilityRow';
import ItemSlots from './ItemSlots';
import StatsDisplay from './StatsDisplay';

const DDRAGON_IMG = `https://ddragon.leagueoflegends.com/cdn/${meta.version}/img`;

export default function ChampionPanel({ build, setBuild, stats }) {
  const [query, setQuery] = useState('');
  const [open, setOpen] = useState(false);

  const filtered = useMemo(() => {
    if (!query.trim()) return [];
    const q = query.toLowerCase();
    return championList.filter((c) => c.name.toLowerCase().includes(q)).slice(0, 8);
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
                onChange={(e) => setBuild((b) => ({ ...b, level: parseInt(e.target.value) }))}
              />
              <span className="level-value">{build.level}</span>
            </div>
          </div>
        </div>

        {build.champion && (
          <>
            <div className="section-label">Abilities</div>
            <AbilityRow build={build} setBuild={setBuild} stats={stats} />

            <div className="section-label">Items</div>
            <ItemSlots build={build} setBuild={setBuild} />

            <div className="section-label">Stats</div>
            <StatsDisplay stats={stats} />
          </>
        )}
      </div>
    </div>
  );
}

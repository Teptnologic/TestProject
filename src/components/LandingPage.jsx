import { useState, useMemo } from 'react';
import { championList } from '../data';
import { COMBO_TEMPLATES } from '../data/combo-templates';
import meta from '../data/generated/meta.json';

const DDRAGON_IMG = `https://ddragon.leagueoflegends.com/cdn/${meta.version}/img`;
const SUPPORTED = new Set(Object.keys(COMBO_TEMPLATES));

const TAG_ORDER = ['Assassin', 'Mage', 'Fighter', 'Marksman', 'Tank', 'Support'];

export default function LandingPage({ onSelect }) {
  const [query, setQuery] = useState('');
  const [tagFilter, setTagFilter] = useState(null);

  const filtered = useMemo(() => {
    let list = [...championList];
    if (tagFilter) list = list.filter((c) => c.tags?.includes(tagFilter));
    if (query.trim()) {
      const q = query.toLowerCase();
      list = list.filter((c) => c.name.toLowerCase().includes(q));
    }
    list.sort((a, b) => {
      const aS = SUPPORTED.has(a.id) ? 0 : 1;
      const bS = SUPPORTED.has(b.id) ? 0 : 1;
      if (aS !== bS) return aS - bS;
      return a.name.localeCompare(b.name);
    });
    return list;
  }, [query, tagFilter]);

  return (
    <div className="landing">
      <div className="landing-hero">
        <h2>Choose a Champion</h2>
        <p>Build combos, compare damage, and theorycraft item builds.</p>
      </div>

      <div className="landing-filters">
        <input
          className="landing-search"
          placeholder="Search champions..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <div className="landing-tags">
          {TAG_ORDER.map((tag) => (
            <button
              key={tag}
              className={`landing-tag ${tagFilter === tag ? 'active' : ''}`}
              onClick={() => setTagFilter(tagFilter === tag ? null : tag)}
            >
              {tag}
            </button>
          ))}
        </div>
      </div>

      <div className="landing-grid">
        {filtered.map((c) => {
          const supported = SUPPORTED.has(c.id);
          return (
            <button
              key={c.id}
              className={`champ-card ${supported ? '' : 'disabled'}`}
              onClick={() => supported && onSelect(c.id)}
              disabled={!supported}
              title={supported ? c.name : `${c.name} — coming soon`}
            >
              <img
                src={`${DDRAGON_IMG}/champion/${c.id}.png`}
                alt={c.name}
                loading="lazy"
              />
              <span className="champ-card-name">{c.name}</span>
              {!supported && <span className="champ-card-badge">Soon</span>}
            </button>
          );
        })}
      </div>

      <div className="landing-footer">
        <span>{SUPPORTED.size} of {championList.length} champions supported</span>
      </div>
    </div>
  );
}

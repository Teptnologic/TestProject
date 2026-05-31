import meta from '../data/generated/meta.json';

const DDRAGON_IMG = `https://ddragon.leagueoflegends.com/cdn/${meta.version}/img`;

export default function AbilityRow({ build, setBuild }) {
  const champ = build.champion;
  if (!champ) return null;

  function adjustRank(key, delta) {
    setBuild((b) => {
      const ability = champ.abilities.find((a) => a.key === key);
      const max = ability?.maxrank ?? 5;
      const cur = b.ranks[key] || 0;
      const next = Math.max(0, Math.min(max, cur + delta));
      return { ...b, ranks: { ...b.ranks, [key]: next } };
    });
  }

  function addToCombo(key) {
    setBuild((b) => ({ ...b, combo: [...b.combo, key] }));
  }

  return (
    <div className="ability-row">
      {champ.abilities.map((ability) => {
        const rank = build.ranks[ability.key] || (ability.key === 'P' ? 1 : 0);
        const iconUrl = ability.icon
          ? (ability.key === 'P'
              ? `${DDRAGON_IMG}/passive/${ability.icon}`
              : `${DDRAGON_IMG}/spell/${ability.icon}`)
          : null;

        return (
          <div className="ability-box" key={ability.key}>
            <div
              className="ability-icon"
              title={`${ability.key} - ${ability.name} (click to add to combo)`}
              onClick={() => addToCombo(ability.key)}
            >
              {iconUrl ? <img src={iconUrl} alt={ability.name} /> : <span className="placeholder">{ability.key}</span>}
              <span className="key-letter">{ability.key}</span>
            </div>
            {ability.key !== 'P' ? (
              <div className="ability-rank">
                <button className="rank-btn" onClick={() => adjustRank(ability.key, -1)} disabled={rank === 0}>-</button>
                <span className="rank-value">{rank}</span>
                <button className="rank-btn" onClick={() => adjustRank(ability.key, 1)} disabled={rank >= ability.maxrank}>+</button>
              </div>
            ) : (
              <div className="ability-rank"><span className="rank-value">—</span></div>
            )}
            <div className="ability-label">{ability.name?.slice(0, 14)}</div>
          </div>
        );
      })}
    </div>
  );
}

import meta from '../data/generated/meta.json';
import GameTooltip from './GameTooltip';
import { formatAbilityTooltip, formatDamageTooltip, lolHtmlToSafeHtml } from '../utils/tooltip';

const DDRAGON_IMG = `https://ddragon.leagueoflegends.com/cdn/${meta.version}/img`;

export default function AbilityRow({ build, setBuild, stats, setCombo }) {
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
    setCombo((prev) => [...prev, key]);
  }

  return (
    <div className="ability-row">
      {/* Auto Attack button */}
      <div className="ability-box">
        <GameTooltip
          title="Auto Attack"
          html="Basic attack dealing physical damage based on your Attack Damage. Click to add to combo."
        >
          <div
            className="ability-icon"
            onClick={() => addToCombo('AA')}
            style={{ borderColor: '#f39c12' }}
          >
            <span className="placeholder" style={{ color: '#f39c12', fontSize: '14px' }}>AA</span>
          </div>
        </GameTooltip>
        <div className="ability-rank"><span className="rank-value">—</span></div>
        <div className="ability-label">Auto Atk</div>
      </div>
      {champ.abilities.map((ability) => {
        const rank = build.ranks[ability.key] || (ability.key === 'P' ? 1 : 0);
        const iconUrl = ability.icon
          ? (ability.key === 'P'
              ? `${DDRAGON_IMG}/passive/${ability.icon}`
              : `${DDRAGON_IMG}/spell/${ability.icon}`)
          : null;

        const rankForTooltip = ability.key === 'P' ? 1 : rank;
        let tooltipHtml = lolHtmlToSafeHtml(
          formatAbilityTooltip(ability, rankForTooltip, stats || {}, build.level),
        );
        if (ability.calculations && Object.keys(ability.calculations).length) {
          const dmgHtml = formatDamageTooltip(ability, rankForTooltip, stats || {}, build.level);
          if (dmgHtml) {
            tooltipHtml += '<br><br><span class="lol-rules">— Damage Formula —</span><br>' + dmgHtml;
          }
        }
        const costLine =
          ability.cost && rankForTooltip > 0
            ? `Cost: ${ability.cost[rankForTooltip - 1] ?? ability.cost.at(-1)}`
            : null;

        return (
          <div className="ability-box" key={ability.key}>
            <GameTooltip
              title={ability.name}
              subtitle={costLine ? `${ability.key} · ${costLine}` : ability.key}
              html={tooltipHtml}
            >
              <div
                className="ability-icon"
                onClick={() => addToCombo(ability.key)}
              >
                {iconUrl ? <img src={iconUrl} alt={ability.name} /> : <span className="placeholder">{ability.key}</span>}
                <span className="key-letter">{ability.key}</span>
              </div>
            </GameTooltip>
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

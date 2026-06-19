import { useMemo } from 'react';
import meta from '../data/generated/meta.json';
import GameTooltip from './GameTooltip';
import { formatAbilityTooltip, formatDamageTooltip, lolHtmlToSafeHtml } from '../utils/tooltip';
import { getEquippedActives } from '../utils/damage';
import { getMultiCasts } from '../data/multi-cast';

const DDRAGON_IMG = `https://ddragon.leagueoflegends.com/cdn/${meta.version}/img`;

export default function AbilityRow({ build, setBuild, stats, setCombo }) {
  const champ = build.champion;
  const actives = useMemo(() => getEquippedActives(build.items), [build.items]);
  if (!champ) return null;

  function totalPointsSpent(ranks) {
    return ['Q', 'W', 'E', 'R'].reduce((sum, k) => sum + (ranks[k] || 0), 0);
  }

  function adjustRank(key, delta) {
    setBuild((b) => {
      const ability = champ.abilities.find((a) => a.key === key);
      const max = ability?.maxrank ?? 5;
      const cur = b.ranks[key] || 0;
      let next = Math.max(0, Math.min(max, cur + delta));
      if (delta > 0 && totalPointsSpent(b.ranks) >= b.level) return b;
      return { ...b, ranks: { ...b.ranks, [key]: next } };
    });
  }

  function addToCombo(key) {
    setCombo((prev) => [...prev, key]);
  }

  const pointsAtCap = totalPointsSpent(build.ranks) >= build.level;

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

        const casts = champ.id ? getMultiCasts(champ.id, ability.key) : null;
        const canClick = ability.key === 'P' || rank > 0;

        return (
          <div className={`ability-box ${!canClick ? 'locked' : ''}`} key={ability.key}>
            <GameTooltip
              title={ability.name}
              subtitle={costLine ? `${ability.key} · ${costLine}` : ability.key}
              html={tooltipHtml}
            >
              {casts ? (
                <div className={`ability-icon multi-cast ${!canClick ? 'disabled' : ''}`}>
                  {iconUrl ? <img src={iconUrl} alt={ability.name} /> : null}
                  <div className="cast-buttons">
                    {casts.map((cast) => (
                      <button
                        key={cast.castKey}
                        className="cast-btn"
                        onClick={(e) => { e.stopPropagation(); if (canClick) addToCombo(cast.castKey); }}
                        title={cast.name}
                        disabled={!canClick}
                      >
                        {cast.label}
                      </button>
                    ))}
                  </div>
                </div>
              ) : (
                <div
                  className={`ability-icon ${!canClick ? 'disabled' : ''}`}
                  onClick={() => canClick && addToCombo(ability.key)}
                >
                  {iconUrl ? <img src={iconUrl} alt={ability.name} /> : <span className="placeholder">{ability.key}</span>}
                  <span className="key-letter">{ability.key}</span>
                </div>
              )}
            </GameTooltip>
            {ability.key !== 'P' ? (
              <div className="ability-rank">
                <button className="rank-btn" onClick={() => adjustRank(ability.key, -1)} disabled={rank === 0}>-</button>
                <span className="rank-value">{rank}</span>
                <button className="rank-btn" onClick={() => adjustRank(ability.key, 1)} disabled={rank >= ability.maxrank || pointsAtCap}>+</button>
              </div>
            ) : (
              <div className="ability-rank"><span className="rank-value">—</span></div>
            )}
            <div className="ability-label">{ability.name?.slice(0, 14)}</div>
          </div>
        );
      })}
      {actives.map((active) => (
        <div className="ability-box" key={`item-${active.id}`}>
          <GameTooltip
            title={active.name}
            html={`Item active from ${active.itemName}. Click to add to combo.`}
          >
            <div
              className="ability-icon"
              onClick={() => addToCombo(`ITEM_${active.id}`)}
              style={{ borderColor: '#e67e22' }}
            >
              <img
                src={`${DDRAGON_IMG}/item/${active.id}.png`}
                alt={active.name}
                onError={(e) => { e.currentTarget.replaceWith(document.createTextNode(active.name.slice(0, 2))); }}
              />
            </div>
          </GameTooltip>
          <div className="ability-rank"><span className="rank-value" style={{ fontSize: '9px', color: '#e67e22' }}>Active</span></div>
          <div className="ability-label">{active.name.slice(0, 14)}</div>
        </div>
      ))}
    </div>
  );
}

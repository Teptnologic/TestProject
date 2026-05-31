import meta from '../data/generated/meta.json';

const DDRAGON_IMG = `https://ddragon.leagueoflegends.com/cdn/${meta.version}/img`;

export default function ComboPanel({ build, setBuild }) {
  const champ = build.champion;
  const combo = build.combo;

  function removeStep(idx) {
    setBuild((b) => ({ ...b, combo: b.combo.filter((_, i) => i !== idx) }));
  }

  function clearCombo() {
    setBuild((b) => ({ ...b, combo: [] }));
  }

  function presetCombo(keys) {
    setBuild((b) => ({ ...b, combo: keys }));
  }

  return (
    <div className="card full">
      <div className="card-header">
        <h2>Combo Builder</h2>
        <span style={{ fontSize: '11px', color: '#5a6270' }}>
          {champ ? 'Click an ability above to append' : 'Select a champion first'}
        </span>
      </div>
      <div className="card-body">
        <div className="combo-sequence">
          {combo.length === 0 && <span className="combo-empty">No abilities yet — click Q/W/E/R above to build a combo</span>}
          {combo.map((key, idx) => {
            const ability = champ?.abilities.find((a) => a.key === key);
            const iconUrl = ability?.icon
              ? (key === 'P'
                  ? `${DDRAGON_IMG}/passive/${ability.icon}`
                  : `${DDRAGON_IMG}/spell/${ability.icon}`)
              : null;
            return (
              <div key={idx} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <div className="combo-step">
                  <div className="combo-icon" title={ability?.name}>
                    {iconUrl ? <img src={iconUrl} alt={key} /> : <span>{key}</span>}
                  </div>
                  <span className="combo-step-number">{idx + 1}</span>
                  <button className="remove-step" onClick={() => removeStep(idx)}>×</button>
                </div>
                {idx < combo.length - 1 && <span className="combo-arrow">→</span>}
              </div>
            );
          })}
        </div>
        <div className="combo-controls">
          <button className="combo-btn" onClick={clearCombo}>Clear</button>
          {champ && (
            <>
              <button className="combo-btn" onClick={() => presetCombo(['E', 'Q', 'R'])}>E → Q → R</button>
              <button className="combo-btn" onClick={() => presetCombo(['Q', 'W', 'E', 'R'])}>Full combo</button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}

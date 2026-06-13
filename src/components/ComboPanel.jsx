import { useState, useEffect } from 'react';
import meta from '../data/generated/meta.json';
import { COMBO_TEMPLATES, getCustomCombos, saveCustomCombo, deleteCustomCombo } from '../data/combo-templates';

const DDRAGON_IMG = `https://ddragon.leagueoflegends.com/cdn/${meta.version}/img`;

export default function ComboPanel({ build, setBuild }) {
  const champ = build.champion;
  const combo = build.combo;
  const champId = champ?.id;

  const [customCombos, setCustomCombos] = useState([]);
  const [saveName, setSaveName] = useState('');
  const [showSave, setShowSave] = useState(false);

  useEffect(() => {
    if (champId) setCustomCombos(getCustomCombos(champId));
  }, [champId]);

  const builtInTemplates = champId ? (COMBO_TEMPLATES[champId] || []) : [];

  function removeStep(idx) {
    setBuild((b) => ({ ...b, combo: b.combo.filter((_, i) => i !== idx) }));
  }

  function clearCombo() {
    setBuild((b) => ({ ...b, combo: [] }));
  }

  function loadCombo(keys) {
    setBuild((b) => ({ ...b, combo: [...keys] }));
  }

  function handleSave() {
    if (!saveName.trim() || !combo.length || !champId) return;
    saveCustomCombo(champId, saveName.trim(), [...combo]);
    setCustomCombos(getCustomCombos(champId));
    setSaveName('');
    setShowSave(false);
  }

  function handleDelete(idx) {
    deleteCustomCombo(champId, idx);
    setCustomCombos(getCustomCombos(champId));
  }

  function formatComboPreview(keys) {
    return keys.join(' → ');
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
            const isAA = key === 'AA';
            const ability = !isAA ? champ?.abilities.find((a) => a.key === key) : null;
            const iconUrl = ability?.icon
              ? (key === 'P'
                  ? `${DDRAGON_IMG}/passive/${ability.icon}`
                  : `${DDRAGON_IMG}/spell/${ability.icon}`)
              : null;
            return (
              <div key={idx} style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <div className="combo-step">
                  <div className="combo-icon" title={isAA ? 'Auto Attack' : ability?.name} style={isAA ? { borderColor: '#f39c12', color: '#f39c12' } : undefined}>
                    {isAA ? <span>AA</span> : iconUrl ? <img src={iconUrl} alt={key} /> : <span>{key}</span>}
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
          {combo.length > 0 && (
            <button className="combo-btn save-btn" onClick={() => setShowSave(!showSave)}>
              {showSave ? 'Cancel' : 'Save Combo'}
            </button>
          )}
        </div>

        {showSave && (
          <div className="combo-save-row">
            <input
              className="combo-save-input"
              placeholder="Combo name..."
              value={saveName}
              onChange={(e) => setSaveName(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && handleSave()}
              autoFocus
            />
            <button className="combo-btn save-btn" onClick={handleSave} disabled={!saveName.trim()}>Save</button>
          </div>
        )}

        {champ && (builtInTemplates.length > 0 || customCombos.length > 0) && (
          <div className="combo-templates">
            <div className="combo-templates-label">Templates</div>
            <div className="combo-template-list">
              {builtInTemplates.map((t, i) => (
                <button
                  key={`builtin-${i}`}
                  className="combo-template-btn"
                  onClick={() => loadCombo(t.keys)}
                  title={formatComboPreview(t.keys)}
                >
                  <span className="template-name">{t.name}</span>
                  <span className="template-preview">{formatComboPreview(t.keys)}</span>
                </button>
              ))}
              {customCombos.map((t, i) => (
                <button
                  key={`custom-${i}`}
                  className="combo-template-btn custom"
                  onClick={() => loadCombo(t.keys)}
                  title={formatComboPreview(t.keys)}
                >
                  <span className="template-name">{t.name}</span>
                  <span className="template-preview">{formatComboPreview(t.keys)}</span>
                  <span className="template-delete" onClick={(e) => { e.stopPropagation(); handleDelete(i); }}>×</span>
                </button>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

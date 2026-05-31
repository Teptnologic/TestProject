export default function TargetPanel({ target, setTarget }) {
  function update(field, value) {
    setTarget((t) => ({ ...t, [field]: Number(value) || 0 }));
  }

  return (
    <div className="card">
      <div className="card-header">
        <h2>Target</h2>
        <span style={{ fontSize: '12px', color: '#5a6270' }}>Manual dummy</span>
      </div>
      <div className="card-body">
        <div className="dummy-stats">
          <div className="dummy-stat-group">
            <label>Health</label>
            <input type="number" value={target.hp} onChange={(e) => update('hp', e.target.value)} />
          </div>
          <div className="dummy-stat-group">
            <label>Armor</label>
            <input type="number" value={target.armor} onChange={(e) => update('armor', e.target.value)} />
          </div>
          <div className="dummy-stat-group">
            <label>Magic Resist</label>
            <input type="number" value={target.spellblock} onChange={(e) => update('spellblock', e.target.value)} />
          </div>
        </div>
        <div className="dummy-stats" style={{ marginTop: 10 }}>
          <div className="dummy-stat-group">
            <label>Shield</label>
            <input type="number" value={target.shield} onChange={(e) => update('shield', e.target.value)} />
          </div>
          <div className="dummy-stat-group">
            <label>Level</label>
            <input type="number" min="1" max="18" value={target.level} onChange={(e) => update('level', e.target.value)} />
          </div>
          <div className="dummy-stat-group">
            <label>Bonus HP</label>
            <input type="number" value={target.bonusHP} onChange={(e) => update('bonusHP', e.target.value)} />
          </div>
        </div>
      </div>
    </div>
  );
}

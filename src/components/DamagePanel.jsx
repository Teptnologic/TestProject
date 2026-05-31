export default function DamagePanel({ result, target }) {
  if (!result || !result.steps.length) {
    return (
      <div className="card full">
        <div className="card-header"><h2>Damage Results</h2></div>
        <div className="card-body">
          <p style={{ color: '#5a6270', fontSize: 13, textAlign: 'center', padding: 20 }}>
            Build a combo to see damage breakdown.
          </p>
        </div>
      </div>
    );
  }

  const { steps, physical, magic, true: trueDmg, total } = result;
  const pct = (v) => total > 0 ? ((v / total) * 100).toFixed(1) : '0.0';
  const remaining = Math.max(0, target.hp - total + target.shield);
  const remainingPct = target.hp > 0 ? Math.max(0, Math.min(100, (remaining / target.hp) * 100)) : 0;
  const isKill = total >= target.hp + target.shield;

  return (
    <div className="card full">
      <div className="card-header">
        <h2>Damage Results</h2>
        <span style={{ fontSize: '11px', color: '#5a6270' }}>After resistances</span>
      </div>
      <div className="card-body">
        <div className="damage-summary">
          <div className="damage-card main">
            <div className="damage-label">Total</div>
            <div className="damage-value">{Math.round(total).toLocaleString()}</div>
            <div className="damage-sub">vs {target.hp.toLocaleString()} HP</div>
          </div>
          <div className="damage-card">
            <div className="damage-label">Physical</div>
            <div className="damage-value physical">{Math.round(physical)}</div>
            <div className="damage-sub">{pct(physical)}%</div>
          </div>
          <div className="damage-card">
            <div className="damage-label">Magic</div>
            <div className="damage-value magic">{Math.round(magic)}</div>
            <div className="damage-sub">{pct(magic)}%</div>
          </div>
          <div className="damage-card">
            <div className="damage-label">True</div>
            <div className="damage-value truedmg">{Math.round(trueDmg)}</div>
            <div className="damage-sub">{pct(trueDmg)}%</div>
          </div>
        </div>

        <table className="breakdown-table">
          <thead>
            <tr>
              <th>#</th>
              <th>Ability</th>
              <th>Type</th>
              <th>Raw</th>
              <th>Post-Resist</th>
              <th>% of Total</th>
            </tr>
          </thead>
          <tbody>
            {steps.map((s, i) => (
              <tr key={i}>
                <td>{i + 1}</td>
                <td><strong>{s.abilityKey}</strong> — {s.abilityName}</td>
                <td><span className={`dmg-type-badge ${s.type}`}>{s.type}</span></td>
                <td>{Math.round(s.raw)}</td>
                <td>{Math.round(s.post)}</td>
                <td>{pct(s.post)}%</td>
              </tr>
            ))}
          </tbody>
        </table>

        <div className="hp-bar-section">
          <div className="hp-bar-label">
            <span>Target HP after combo</span>
            <span>{Math.round(remaining)} / {target.hp.toLocaleString()} HP</span>
          </div>
          <div className="hp-bar">
            <div className="hp-bar-remaining" style={{ width: `${remainingPct}%` }} />
            <div className="hp-bar-damage" style={{ left: `${remainingPct}%`, width: `${100 - remainingPct}%` }} />
            <span className="hp-bar-text">{Math.round(Math.min(total, target.hp + target.shield)).toLocaleString()} damage</span>
          </div>
          <span className={`kill-tag ${isKill ? 'kill' : 'survive'}`}>
            {isKill ? `TARGET KILLED (${Math.round(total - target.hp - target.shield)} overkill)` : `Target survives — ${Math.round(remaining)} HP left`}
          </span>
        </div>
      </div>
    </div>
  );
}

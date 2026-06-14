export default function DamagePanel({ results, builds, target, combo }) {
  const hasCombo = combo && combo.length > 0;
  const validResults = results?.filter(Boolean) || [];

  if (!hasCombo || validResults.length === 0) {
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

  const isComparing = results.length === 2 && results[0] && results[1];

  if (isComparing) {
    return <CompareView results={results} builds={builds} target={target} />;
  }

  const resultIdx = results.findIndex(Boolean);
  const result = results[resultIdx];
  return <SingleView result={result} target={target} />;
}

function SingleView({ result, target }) {
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
        <DamageSummary result={result} target={target} />

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

        <HPBar total={total} target={target} remaining={remaining} remainingPct={remainingPct} isKill={isKill} />
      </div>
    </div>
  );
}

function CompareView({ results, builds, target }) {
  const [resultA, resultB] = results;
  const nameA = builds[0]?.champion?.name || 'Build A';
  const nameB = builds[1]?.champion?.name || 'Build B';
  const delta = resultA.total - resultB.total;

  return (
    <div className="card full">
      <div className="card-header">
        <h2>Damage Comparison</h2>
        <span style={{ fontSize: '11px', color: '#5a6270' }}>Side by side</span>
      </div>
      <div className="card-body">
        <div className="compare-grid">
          <div className="compare-column">
            <div className="compare-label">Build A — {nameA}</div>
            <DamageSummary result={resultA} target={target} compact />
          </div>
          <div className="compare-delta">
            <div className="delta-label">Delta</div>
            <div className={`delta-value ${delta > 0 ? 'positive' : delta < 0 ? 'negative' : ''}`}>
              {delta > 0 ? '+' : ''}{Math.round(delta)}
            </div>
            <div className="delta-sub">
              {delta > 0 ? 'A deals more' : delta < 0 ? 'B deals more' : 'Equal'}
            </div>
          </div>
          <div className="compare-column">
            <div className="compare-label">Build B — {nameB}</div>
            <DamageSummary result={resultB} target={target} compact />
          </div>
        </div>

        <table className="breakdown-table compare-breakdown">
          <thead>
            <tr>
              <th>#</th>
              <th>Ability</th>
              <th>Type</th>
              <th>Build A</th>
              <th>Build B</th>
              <th>Diff</th>
            </tr>
          </thead>
          <tbody>
            {resultA.steps.map((sA, i) => {
              const sB = resultB.steps[i];
              const diff = sB ? sA.post - sB.post : sA.post;
              return (
                <tr key={i}>
                  <td>{i + 1}</td>
                  <td><strong>{sA.abilityKey}</strong> — {sA.abilityName}</td>
                  <td><span className={`dmg-type-badge ${sA.type}`}>{sA.type}</span></td>
                  <td>{Math.round(sA.post)}</td>
                  <td>{sB ? Math.round(sB.post) : '—'}</td>
                  <td className={diff > 0 ? 'diff-positive' : diff < 0 ? 'diff-negative' : ''}>
                    {diff > 0 ? '+' : ''}{Math.round(diff)}
                  </td>
                </tr>
              );
            })}
          </tbody>
          <tfoot>
            <tr className="total-row">
              <td colSpan={3}><strong>Total</strong></td>
              <td><strong>{Math.round(resultA.total)}</strong></td>
              <td><strong>{Math.round(resultB.total)}</strong></td>
              <td className={delta > 0 ? 'diff-positive' : delta < 0 ? 'diff-negative' : ''}>
                <strong>{delta > 0 ? '+' : ''}{Math.round(delta)}</strong>
              </td>
            </tr>
          </tfoot>
        </table>

        <div className="compare-hp-bars">
          <div className="compare-hp-bar">
            <span className="compare-hp-label">{nameA}</span>
            <HPBarMini total={resultA.total} target={target} />
          </div>
          <div className="compare-hp-bar">
            <span className="compare-hp-label">{nameB}</span>
            <HPBarMini total={resultB.total} target={target} />
          </div>
        </div>
      </div>
    </div>
  );
}

function DamageSummary({ result, target, compact }) {
  const { physical, magic, true: trueDmg, total } = result;
  const pct = (v) => total > 0 ? ((v / total) * 100).toFixed(1) : '0.0';

  return (
    <div className={`damage-summary ${compact ? 'compact' : ''}`}>
      <div className="damage-card main">
        <div className="damage-label">Total</div>
        <div className="damage-value">{Math.round(total).toLocaleString()}</div>
        {!compact && <div className="damage-sub">vs {target.hp.toLocaleString()} HP</div>}
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
  );
}

function HPBar({ total, target, remaining, remainingPct, isKill }) {
  return (
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
  );
}

function HPBarMini({ total, target }) {
  const effective = target.hp + target.shield;
  const remaining = Math.max(0, effective - total);
  const remainingPct = target.hp > 0 ? Math.max(0, Math.min(100, (remaining / target.hp) * 100)) : 0;
  const isKill = total >= effective;

  return (
    <div className="hp-bar-mini">
      <div className="hp-bar">
        <div className="hp-bar-remaining" style={{ width: `${remainingPct}%` }} />
        <div className="hp-bar-damage" style={{ left: `${remainingPct}%`, width: `${100 - remainingPct}%` }} />
        <span className="hp-bar-text">
          {isKill ? 'KILL' : `${Math.round(remaining)} HP left`}
        </span>
      </div>
    </div>
  );
}

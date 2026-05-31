function StatRow({ kind, label, value, bonus }) {
  return (
    <div className="stat-item">
      <span className={`stat-icon ${kind}`}></span>
      <span className="stat-label">{label}</span>
      <span className="stat-value">
        {Math.round(value)}{bonus > 0 && <span className="stat-bonus"> (+{Math.round(bonus)})</span>}
      </span>
    </div>
  );
}

export default function StatsDisplay({ stats }) {
  if (!stats) return null;
  return (
    <div className="stats-grid">
      <StatRow kind="hp" label="HP" value={stats.hp} bonus={stats.bonusHP || 0} />
      <StatRow kind="mp" label="MP" value={stats.mp} bonus={0} />
      <StatRow kind="ad" label="AD" value={stats.attackdamage} bonus={stats.bonusAD || 0} />
      <StatRow kind="ap" label="AP" value={stats.ap || 0} bonus={0} />
      <StatRow kind="armor" label="Armor" value={stats.armor} bonus={0} />
      <StatRow kind="mr" label="MR" value={stats.spellblock} bonus={0} />
      <StatRow kind="as" label="Atk Speed" value={Math.round(stats.attackspeed * 1000) / 1000} bonus={0} />
      <StatRow kind="crit" label="Crit %" value={stats.crit} bonus={0} />
    </div>
  );
}

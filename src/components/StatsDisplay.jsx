function StatRow({ kind, label, value, bonus, decimals }) {
  const fmt = decimals ? Number(value).toFixed(decimals) : Math.round(value);
  const bonusFmt = decimals ? Number(bonus).toFixed(decimals) : Math.round(bonus);
  return (
    <div className="stat-item">
      <span className={`stat-icon ${kind}`}></span>
      <span className="stat-label">{label}</span>
      <span className="stat-value">
        {fmt}{bonus > 0 && <span className="stat-bonus"> (+{bonusFmt})</span>}
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
      <StatRow kind="as" label="Atk Speed" value={stats.attackspeed} bonus={0} decimals={3} />
      <StatRow kind="crit" label="Crit %" value={stats.crit} bonus={0} />
      {stats.lethality > 0 && <StatRow kind="ad" label="Lethality" value={stats.lethality} bonus={0} />}
      {stats.flatMagicPen > 0 && <StatRow kind="ap" label="Flat M.Pen" value={stats.flatMagicPen} bonus={0} />}
      {stats.magicPenPct > 0 && <StatRow kind="ap" label="% M.Pen" value={Math.round(stats.magicPenPct * 100)} bonus={0} />}
      {stats.armorPenPct > 0 && <StatRow kind="ad" label="% Armor Pen" value={Math.round(stats.armorPenPct * 100)} bonus={0} />}
      {stats.abilityHaste > 0 && <StatRow kind="mp" label="Ability Haste" value={stats.abilityHaste} bonus={0} />}
    </div>
  );
}

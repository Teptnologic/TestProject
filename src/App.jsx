import { useState, useMemo } from 'react';
import { getChampion, getItem } from './data';
import meta from './data/generated/meta.json';
import { totalStats, computeCombo } from './utils/damage';

import ChampionPanel from './components/ChampionPanel';
import TargetPanel from './components/TargetPanel';
import ComboPanel from './components/ComboPanel';
import DamagePanel from './components/DamagePanel';

import './App.css';

const INITIAL_BUILD = {
  championId: null,
  level: 11,
  ranks: { P: 1, Q: 5, W: 1, E: 3, R: 2 },
  items: [null, null, null, null, null, null],
  combo: [],
};

const INITIAL_TARGET = {
  hp: 2000,
  bonusHP: 0,
  armor: 80,
  spellblock: 45,
  shield: 0,
  level: 11,
};

export default function App() {
  const [build, setBuild] = useState(INITIAL_BUILD);
  const [target, setTarget] = useState(INITIAL_TARGET);

  // Resolve champion + items into denormalized objects
  const champion = build.championId ? getChampion(build.championId) : null;
  const items = build.items.map((id) => (id ? getItem(id) : null));
  const fullBuild = { ...build, champion, items };

  // Compute attacker stats
  const stats = useMemo(() => {
    if (!champion) return null;
    return totalStats(champion.stats, build.level, items);
  }, [champion, build.level, items]);

  // Compute combo damage
  const damageResult = useMemo(() => {
    if (!champion || !stats || !build.combo.length) return null;
    return computeCombo(build.combo, champion, build.ranks, stats, target, build.level);
  }, [champion, stats, build.combo, build.ranks, target, build.level]);

  return (
    <>
      <header className="app-header">
        <h1>LoL Damage Calc</h1>
        <span className="patch">Patch {meta.version}</span>
      </header>

      <main className="main-grid">
        <ChampionPanel build={fullBuild} setBuild={setBuild} stats={stats} />
        <TargetPanel target={target} setTarget={setTarget} />
        <ComboPanel build={fullBuild} setBuild={setBuild} />
        <DamagePanel result={damageResult} target={target} />
      </main>
    </>
  );
}

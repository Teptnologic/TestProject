import { useState, useMemo } from 'react';
import { getChampion, getItem } from './data';
import { ITEM_OVERRIDES } from './data/item-overrides';
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
};

const INITIAL_TARGET = {
  hp: 2000,
  bonusHP: 0,
  armor: 80,
  spellblock: 45,
  shield: 0,
  level: 11,
};

function resolveItems(itemIds) {
  return itemIds.map((id) => {
    if (!id) return null;
    const item = getItem(id);
    if (!item) return null;
    const overrides = ITEM_OVERRIDES[id];
    return overrides ? { ...item, _overrides: overrides } : item;
  });
}

export default function App() {
  const [builds, setBuilds] = useState([{ ...INITIAL_BUILD }]);
  const [activeBuildIdx, setActiveBuildIdx] = useState(0);
  const [combo, setCombo] = useState([]);
  const [target, setTarget] = useState(INITIAL_TARGET);

  const resolvedBuilds = useMemo(() => builds.map((build) => {
    const champion = build.championId ? getChampion(build.championId) : null;
    const items = resolveItems(build.items);
    const stats = champion ? totalStats(champion.stats, build.level, items) : null;
    return { ...build, champion, items, stats };
  }), [builds]);

  const damageResults = useMemo(() => resolvedBuilds.map((rb) => {
    if (!rb.champion || !rb.stats || !combo.length) return null;
    return computeCombo(combo, rb.champion, rb.ranks, rb.stats, target, rb.level, rb.items);
  }), [resolvedBuilds, combo, target]);

  const activeBuild = resolvedBuilds[activeBuildIdx] || resolvedBuilds[0];

  function setActiveBuild(updater) {
    setBuilds((prev) => {
      const next = [...prev];
      const idx = activeBuildIdx < next.length ? activeBuildIdx : 0;
      next[idx] = typeof updater === 'function' ? updater(next[idx]) : updater;
      return next;
    });
  }

  function addBuild() {
    if (builds.length >= 2) return;
    setBuilds((prev) => [...prev, { ...INITIAL_BUILD }]);
    setActiveBuildIdx(1);
  }

  function removeBuild(idx) {
    setBuilds((prev) => prev.filter((_, i) => i !== idx));
    setActiveBuildIdx(0);
  }

  const fullBuild = { ...activeBuild, combo };

  return (
    <>
      <header className="app-header">
        <h1>Boris Diff</h1>
        <span className="patch">Patch {meta.version}</span>
      </header>

      <div className="build-tabs">
        {builds.map((_, i) => (
          <button
            key={i}
            className={`build-tab ${i === activeBuildIdx ? 'active' : ''}`}
            onClick={() => setActiveBuildIdx(i)}
          >
            <span className={`build-tab-dot ${resolvedBuilds[i]?.champion ? 'has-champ' : ''}`} />
            Build {String.fromCharCode(65 + i)}
            {resolvedBuilds[i]?.champion && (
              <span className="build-tab-champ">{resolvedBuilds[i].champion.name}</span>
            )}
            {builds.length > 1 && (
              <span className="build-tab-close" onClick={(e) => { e.stopPropagation(); removeBuild(i); }}>×</span>
            )}
          </button>
        ))}
        {builds.length < 2 && (
          <button className="build-tab add" onClick={addBuild}>+ Compare</button>
        )}
      </div>

      <main className="main-grid">
        <ChampionPanel build={fullBuild} setBuild={setActiveBuild} stats={activeBuild.stats} setCombo={setCombo} />
        <TargetPanel target={target} setTarget={setTarget} />
        <ComboPanel build={fullBuild} setCombo={setCombo} />
        <DamagePanel results={damageResults} builds={resolvedBuilds} target={target} combo={combo} />
      </main>
    </>
  );
}

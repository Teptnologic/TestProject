import { useState, useMemo, useEffect, useRef } from 'react';
import { getChampion, getItem } from './data';
import { ITEM_OVERRIDES } from './data/item-overrides';
import meta from './data/generated/meta.json';
import { totalStats, computeCombo } from './utils/damage';
import { encodeShareUrl, decodeShareUrl } from './utils/shareState';

import ChampionPanel from './components/ChampionPanel';
import TargetPanel from './components/TargetPanel';
import ComboPanel from './components/ComboPanel';
import DamagePanel from './components/DamagePanel';
import LandingPage from './components/LandingPage';
import AdBanner from './components/AdBanner';


import './App.css';

const INITIAL_BUILD = {
  championId: null,
  level: 11,
  ranks: { P: 1, Q: 0, W: 0, E: 0, R: 0 },
  items: [null, null, null, null, null, null],
  adaptiveForce: 0,
  adaptiveType: 'auto',
};

const INITIAL_TARGET = {
  hp: 2000,
  bonusHP: 0,
  armor: 80,
  spellblock: 45,
  shield: 0,
  level: 11,
};

// Read the full shareable state (builds + combo + target) from the current URL,
// merged onto the defaults.
function stateFromUrl() {
  const decoded =
    typeof window === 'undefined'
      ? { builds: [{}], combo: null, target: null }
      : decodeShareUrl(window.location.pathname, window.location.search);
  return {
    builds: decoded.builds.map((b) => ({ ...INITIAL_BUILD, ...b })),
    combo: decoded.combo || [],
    target: { ...INITIAL_TARGET, ...(decoded.target || {}) },
  };
}

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
  const [initial] = useState(stateFromUrl);
  const [builds, setBuilds] = useState(initial.builds);
  const [activeBuildIdx, setActiveBuildIdx] = useState(0);
  const [combo, setCombo] = useState(initial.combo);
  const [target, setTarget] = useState(initial.target);
  const [copied, setCopied] = useState(false);

  // The whole shareable state is reflected in the URL: a champion change is a new
  // history entry (so Back works between champions); finer edits (level, items,
  // combo, target) replace the current entry to avoid flooding history.
  const shareUrl = useMemo(() => encodeShareUrl(builds, combo, target), [builds, combo, target]);
  const buildAChampId = builds[0]?.championId || null;
  const prevChamp = useRef(buildAChampId);
  useEffect(() => {
    const current = window.location.pathname + window.location.search;
    if (current !== shareUrl) {
      if (prevChamp.current !== buildAChampId) {
        window.history.pushState({}, '', shareUrl);
      } else {
        window.history.replaceState({}, '', shareUrl);
      }
    }
    prevChamp.current = buildAChampId;
    const champ = buildAChampId ? getChampion(buildAChampId) : null;
    document.title = champ ? `${champ.name} Build & Damage Calculator – Boris Diff` : 'Boris Diff';
  }, [shareUrl, buildAChampId]);

  // Re-read the full state when the user navigates back/forward.
  useEffect(() => {
    function onPop() {
      const s = stateFromUrl();
      setBuilds(s.builds);
      setCombo(s.combo);
      setTarget(s.target);
      setActiveBuildIdx((i) => Math.min(i, s.builds.length - 1));
    }
    window.addEventListener('popstate', onPop);
    return () => window.removeEventListener('popstate', onPop);
  }, []);

  function copyShareLink() {
    const url = window.location.origin + shareUrl;
    navigator.clipboard?.writeText(url).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  }

  const resolvedBuilds = useMemo(() => builds.map((build) => {
    const champion = build.championId ? getChampion(build.championId) : null;
    const items = resolveItems(build.items);
    const stats = champion ? totalStats(champion.stats, build.level, items, champion.id, build.ranks, build.adaptiveForce, build.adaptiveType) : null;
    if (champion && import.meta.env.DEV && import.meta.env.VITE_DEBUG) {
      fetch('/__log', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          label: `Boris ${champion.name} (${champion.id})`,
          data: {
            baseStats: champion.stats,
            computedStats: stats,
            ranks: build.ranks,
            level: build.level,
            adaptiveForce: build.adaptiveForce,
            adaptiveType: build.adaptiveType,
            items: items.map((i) => i && {
              id: i.id,
              name: i.name,
              stats: i.stats,
              overrides: i._overrides,
              binDirectStats: i.bin?.directStats,
            }),
            abilities: champion.abilities?.map((a) => ({
              key: a.key,
              name: a.name,
              maxrank: a.maxrank,
              cooldown: a.cooldown,
              cost: a.cost,
              range: a.range,
              dataValues: a.dataValues,
              calculations: a.calculations,
            })),
            recastAbilities: champion.recastAbilities,
            passive: champion.passive,
          },
        }),
      }).catch(() => {});
    }
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

  function selectFromLanding(champId) {
    setBuilds((prev) => {
      const next = [...prev];
      next[0] = { ...next[0], championId: champId || null };
      return next;
    });
  }

  const showLanding = !buildAChampId;

  return (
    <>
      <header className="app-header">
        <h1 className="app-title" onClick={() => selectFromLanding(null)} style={{ cursor: 'pointer' }}>
          <img src="/boris.png" alt="" className="app-logo" />
          Boris Diff
        </h1>
        <span className="patch">Patch {meta.version}</span>
        {!showLanding && (
          <button className="share-btn" onClick={copyShareLink} title="Copy a link to this exact build & combo">
            {copied ? '✓ Link copied' : '🔗 Share build'}
          </button>
        )}
      </header>

      {showLanding ? (
        <LandingPage onSelect={selectFromLanding} />
      ) : (
        <div className="detail-layout">
          <aside className="ad-sidebar-left">
            <AdBanner slot="6142626696" label="Champion Page Left" />
          </aside>
          <div className="detail-content">
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
          </div>
        </div>
      )}
    </>
  );
}

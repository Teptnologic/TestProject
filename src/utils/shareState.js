// Encode/decode the full shareable app state to and from the URL so any build,
// combo, and target can be reproduced from a link (e.g. shared on Reddit).
//
// Build A's champion lives in the path (/c/<Champion>) so each champion keeps a
// clean, prerendered, preview-friendly URL. Everything else rides in the query
// string:
//   lvl   Build A level            r   Build A ranks  "Q-W-E-R"
//   i     Build A items "id-id-…"  b   Build B champion id
//   blvl  Build B level            br  Build B ranks
//   bi    Build B items            combo  steps "Q-E-AA-ITEM_3152"
//   t     target "hp-armor-mr-shield-bonusHP-level"

import { championList } from '../data';

function canonChamp(slug) {
  if (!slug) return null;
  const s = decodeURIComponent(slug).toLowerCase();
  const found = championList.find((c) => c.id.toLowerCase() === s);
  return found ? found.id : null;
}

function clampLevel(n) {
  const v = parseInt(n, 10);
  if (!Number.isFinite(v)) return null;
  return Math.max(1, Math.min(18, v));
}

function encRanks(r = {}) {
  return [r.Q || 0, r.W || 0, r.E || 0, r.R || 0].join('-');
}
function decRanks(str) {
  const [Q, W, E, R] = String(str).split('-').map((n) => parseInt(n, 10) || 0);
  return { P: 1, Q, W, E, R };
}

function encItems(items = []) {
  return items.map((x) => x || '').join('-');
}
function decItems(str) {
  const parts = String(str).split('-');
  const out = [null, null, null, null, null, null];
  for (let i = 0; i < 6; i++) {
    const v = parseInt(parts[i], 10);
    out[i] = Number.isFinite(v) ? v : null;
  }
  return out;
}

export function encodeShareUrl(builds, combo, target) {
  const a = builds[0] || {};
  const path = a.championId ? `/c/${a.championId}` : '/';
  const q = new URLSearchParams();

  if (a.championId) {
    if (a.level != null) q.set('lvl', a.level);
    if (a.ranks) q.set('r', encRanks(a.ranks));
    if ((a.items || []).some(Boolean)) q.set('i', encItems(a.items));
  }

  const b = builds[1];
  if (b && b.championId) {
    q.set('b', b.championId);
    if (b.level != null) q.set('blvl', b.level);
    if (b.ranks) q.set('br', encRanks(b.ranks));
    if ((b.items || []).some(Boolean)) q.set('bi', encItems(b.items));
  }

  if (combo && combo.length) q.set('combo', combo.join('-'));

  if (a.championId && target) {
    q.set('t', [target.hp, target.armor, target.spellblock, target.shield, target.bonusHP, target.level].join('-'));
  }

  const qs = q.toString();
  return qs ? `${path}?${qs}` : path;
}

export function decodeShareUrl(pathname, search) {
  const m = pathname.match(/^\/c\/([^/]+)/);
  const buildA = { championId: m ? canonChamp(m[1]) : null };
  const q = new URLSearchParams(search || '');

  const lvlA = clampLevel(q.get('lvl'));
  if (lvlA != null) buildA.level = lvlA;
  if (q.has('r')) buildA.ranks = decRanks(q.get('r'));
  if (q.has('i')) buildA.items = decItems(q.get('i'));

  const builds = [buildA];

  const champB = canonChamp(q.get('b'));
  if (champB) {
    const buildB = { championId: champB };
    const lvlB = clampLevel(q.get('blvl'));
    if (lvlB != null) buildB.level = lvlB;
    if (q.has('br')) buildB.ranks = decRanks(q.get('br'));
    if (q.has('bi')) buildB.items = decItems(q.get('bi'));
    builds.push(buildB);
  }

  const combo = q.has('combo') ? q.get('combo').split('-').filter(Boolean) : null;

  let target = null;
  if (q.has('t')) {
    const [hp, armor, spellblock, shield, bonusHP, level] = q
      .get('t')
      .split('-')
      .map((n) => parseFloat(n) || 0);
    target = { hp, armor, spellblock, shield, bonusHP, level };
  }

  return { builds, combo, target };
}

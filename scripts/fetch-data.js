#!/usr/bin/env node
import { writeFile, mkdir } from 'node:fs/promises';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const OUT_DIR = join(__dirname, '..', 'src', 'data', 'generated');

const DDRAGON = 'https://ddragon.leagueoflegends.com';
const CDRAGON = 'https://raw.communitydragon.org/latest';

async function fetchJSON(url) {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`${res.status} ${url}`);
  return res.json();
}

async function getLatestVersion() {
  const versions = await fetchJSON(`${DDRAGON}/api/versions.json`);
  return versions[0];
}

async function fetchChampionList(version) {
  const data = await fetchJSON(`${DDRAGON}/cdn/${version}/data/en_US/champion.json`);
  return Object.values(data.data).map((c) => ({
    id: c.id,
    key: parseInt(c.key, 10),
    name: c.name,
    title: c.title,
    tags: c.tags,
    stats: c.stats,
  }));
}

async function fetchChampionDetail(version, championId) {
  const data = await fetchJSON(
    `${DDRAGON}/cdn/${version}/data/en_US/champion/${championId}.json`
  );
  const champ = data.data[championId];
  return {
    id: champ.id,
    name: champ.name,
    passive: {
      name: champ.passive.name,
      description: champ.passive.description,
      icon: champ.passive.image.full,
    },
    spells: champ.spells.map((s) => ({
      id: s.id,
      name: s.name,
      description: s.description,
      tooltip: s.tooltip,
      maxrank: s.maxrank,
      cooldown: s.cooldown,
      cost: s.cost,
      costType: s.costType,
      range: s.range,
      effect: s.effect,
      vars: s.vars,
      icon: s.image.full,
    })),
  };
}

// Strip animation/VFX/audio noise from spell objects but keep all numeric and
// calculation data intact (mDataValues, mSpellCalculations, mEffectAmount,
// mCoefficient, mBuffName, mBuffs, mDataValueOverrides, etc.)
const DROP_FIELDS = new Set([
  'mClientData',
  'mLineMissileSpec',
  'mMissileSpec',
  'mMissileGraphics',
  'mAnimationName',
  'mSpellRevealsChampion',
  'mTargetingType',
  'mTargetingTypeOverride',
]);

function stripNoise(obj) {
  if (!obj || typeof obj !== 'object') return obj;
  if (Array.isArray(obj)) return obj.map(stripNoise);
  const out = {};
  for (const [k, v] of Object.entries(obj)) {
    if (DROP_FIELDS.has(k)) continue;
    if (typeof v === 'object' && v !== null) {
      out[k] = stripNoise(v);
    } else {
      out[k] = v;
    }
  }
  return out;
}

function looksLikeBotVariant(key) {
  return /(?:Bot|Tutorial|URF|Sandbox|TFT|Arena|Cherry|Strawberry|Practice|OdysseyAugment)/i.test(
    key
  );
}

function looksLikeSpellEntry(key) {
  // Real spell entries live under Characters/<Champ>/Spells/... and have mScriptName or mSpell
  return key.startsWith('Characters/') && /\/Spells\//.test(key);
}

async function fetchCDragonChampion(key, championId) {
  const lc = championId.toLowerCase();
  const result = { spells: [] };

  // Slim metadata endpoint (champion key, splash, etc.)
  try {
    const data = await fetchJSON(
      `${CDRAGON}/plugins/rcp-be-lol-game-data/global/default/v1/champions/${key}.json`
    );
    result.spells = (data.spells || []).map((s) => ({
      id: s.spellKey,
      name: s.name,
      cooldownCoefficients: s.cooldownCoefficients,
      cost: s.cost,
      range: s.range,
      dataValues: s.dataValues,
      coefficients: s.coefficients,
      effectAmounts: s.effectAmounts,
    }));
  } catch (err) {
    console.warn(`\n  cdragon meta failed for ${championId}: ${err.message}`);
  }

  // Rich game data: keep full spell objects so we don't lose mDataValues etc.
  try {
    const bin = await fetchJSON(`${CDRAGON}/game/data/characters/${lc}/${lc}.bin.json`);
    const spellEntries = {};
    for (const [k, v] of Object.entries(bin)) {
      if (looksLikeBotVariant(k)) continue;

      // CharacterRecords/Root holds base stats + spell name pointers
      if (k.includes('CharacterRecords/Root')) {
        spellEntries[k] = pickCharacterStats(v);
        continue;
      }

      // Spell entries: keep entire mSpell payload (with noise stripped)
      if (looksLikeSpellEntry(k) && v && typeof v === 'object') {
        const out = { mScriptName: v.mScriptName };
        if (v.mSpell) out.mSpell = stripNoise(v.mSpell);
        // Some champs put data at top level instead of under mSpell
        for (const field of [
          'mDataValues',
          'mSpellCalculations',
          'mCoefficient',
          'mEffectAmount',
          'mCooldownTime',
          'mManaCost',
          'mCastRange',
          'mDataValueOverrides',
        ]) {
          if (v[field] !== undefined) out[field] = v[field];
        }
        spellEntries[k] = out;
      }
    }
    result.bin = spellEntries;
  } catch (err) {
    console.warn(`\n  cdragon bin failed for ${championId}: ${err.message}`);
  }

  return result;
}

function pickCharacterStats(v) {
  if (!v || typeof v !== 'object') return null;
  const wanted = [
    'baseHP', 'hpPerLevel', 'baseMP', 'mpPerLevel',
    'baseDamage', 'damagePerLevel',
    'baseArmor', 'armorPerLevel',
    'baseSpellBlock', 'spellBlockPerLevel',
    'baseStaticHPRegen', 'hpRegenPerLevel',
    'baseStaticMPRegen', 'mpRegenPerLevel',
    'attackSpeed', 'attackSpeedRatio', 'attackSpeedPerLevel',
    'attackRange', 'critDamageMultiplier',
    'baseMoveSpeed',
    'spellNames', 'extraSpells',
  ];
  const out = {};
  for (const w of wanted) if (w in v) out[w] = v[w];
  return out;
}

async function fetchItems(version) {
  const data = await fetchJSON(`${DDRAGON}/cdn/${version}/data/en_US/item.json`);
  return Object.entries(data.data)
    .filter(([, v]) => v.gold?.purchasable && v.maps?.['11'])
    .map(([id, v]) => ({
      id: parseInt(id, 10),
      name: v.name,
      description: v.description,
      plaintext: v.plaintext,
      gold: v.gold.total,
      stats: v.stats,
      tags: v.tags,
    }));
}

async function main() {
  await mkdir(OUT_DIR, { recursive: true });

  // CLI flags: --sample=Lux,Aatrox  (only fetch listed champs, useful for testing)
  const sampleArg = process.argv.find((a) => a.startsWith('--sample='));
  const sampleSet = sampleArg ? new Set(sampleArg.slice('--sample='.length).split(',')) : null;

  console.log('Fetching version...');
  const version = await getLatestVersion();
  console.log(`  version: ${version}`);

  console.log('Fetching champion list...');
  let champions = await fetchChampionList(version);
  console.log(`  ${champions.length} champions`);
  if (sampleSet) {
    champions = champions.filter((c) => sampleSet.has(c.id));
    console.log(`  sampled down to ${champions.length}`);
  }

  console.log('Fetching champion details (this takes a few minutes)...');
  const details = {};
  for (let i = 0; i < champions.length; i++) {
    const c = champions[i];
    process.stdout.write(`\r  [${i + 1}/${champions.length}] ${c.id}            `);
    const [dd, cd] = await Promise.all([
      fetchChampionDetail(version, c.id).catch(() => null),
      fetchCDragonChampion(c.key, c.id).catch(() => null),
    ]);
    details[c.id] = { ddragon: dd, cdragon: cd };
  }
  console.log('');

  console.log('Fetching items...');
  const items = await fetchItems(version);
  console.log(`  ${items.length} items`);

  const meta = { version, fetchedAt: new Date().toISOString() };

  await writeFile(join(OUT_DIR, 'meta.json'), JSON.stringify(meta, null, 2));
  await writeFile(join(OUT_DIR, 'champions.json'), JSON.stringify(champions, null, 2));
  await writeFile(join(OUT_DIR, 'champion-details.json'), JSON.stringify(details));
  await writeFile(join(OUT_DIR, 'items.json'), JSON.stringify(items, null, 2));

  console.log(`\nDone. Data written to ${OUT_DIR}`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});

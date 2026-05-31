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

async function fetchCDragonChampion(key) {
  try {
    const data = await fetchJSON(
      `${CDRAGON}/plugins/rcp-be-lol-game-data/global/default/v1/champions/${key}.json`
    );
    const spells = (data.spells || []).map((s) => ({
      id: s.spellKey,
      name: s.name,
      cooldownCoefficients: s.cooldownCoefficients,
      cost: s.cost,
      range: s.range,
      dataValues: s.dataValues,
      coefficients: s.coefficients,
      effectAmounts: s.effectAmounts,
    }));
    return { spells };
  } catch (err) {
    console.warn(`  cdragon failed for key=${key}: ${err.message}`);
    return null;
  }
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

  console.log('Fetching version...');
  const version = await getLatestVersion();
  console.log(`  version: ${version}`);

  console.log('Fetching champion list...');
  const champions = await fetchChampionList(version);
  console.log(`  ${champions.length} champions`);

  console.log('Fetching champion details (this takes ~30s)...');
  const details = {};
  for (let i = 0; i < champions.length; i++) {
    const c = champions[i];
    process.stdout.write(`\r  [${i + 1}/${champions.length}] ${c.id}            `);
    const [dd, cd] = await Promise.all([
      fetchChampionDetail(version, c.id).catch(() => null),
      fetchCDragonChampion(c.key).catch(() => null),
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

// Post-build step: generate a static HTML file per champion at /c/<id>/index.html.
// Each file is a copy of the built index.html with champion-specific <title> and
// Open Graph / Twitter meta tags so links shared on Reddit (etc.) render a rich
// preview card using the champion's Data Dragon splash art. The SPA reads the
// URL on load and selects the champion, so behaviour is identical to the live app.

import { readFileSync, writeFileSync, mkdirSync } from 'fs';
import { dirname, resolve } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, '..');
const distDir = resolve(root, 'dist');

// Resolve the production origin for absolute og:url values. Vercel exposes the
// production domain at build time; fall back to SITE_URL or a sensible default.
const rawOrigin =
  process.env.SITE_URL ||
  (process.env.VERCEL_PROJECT_PRODUCTION_URL
    ? `https://${process.env.VERCEL_PROJECT_PRODUCTION_URL}`
    : 'https://boris-diff.vercel.app');
const ORIGIN = rawOrigin.replace(/\/$/, '');

const champions = JSON.parse(
  readFileSync(resolve(root, 'src/data/generated/champions.json'), 'utf8'),
);
const template = readFileSync(resolve(distDir, 'index.html'), 'utf8');

const START = '<!-- OG_META_START -->';
const END = '<!-- OG_META_END -->';
if (!template.includes(START) || !template.includes(END)) {
  console.error('[prerender] OG_META markers not found in dist/index.html — skipping.');
  process.exit(0);
}

function esc(s) {
  return String(s).replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;');
}

function metaBlock(champ) {
  const title = `${champ.name} Build & Damage Calculator – Boris Diff`;
  const desc = `Theorycraft ${champ.name}, ${champ.title}, on Boris Diff — calculate combo damage, compare builds, and optimize items.`;
  const splash = `https://ddragon.leagueoflegends.com/cdn/img/champion/splash/${champ.id}_0.jpg`;
  const url = `${ORIGIN}/c/${champ.id}`;
  return [
    START,
    `    <title>${esc(title)}</title>`,
    `    <meta name="description" content="${esc(desc)}" />`,
    `    <meta property="og:type" content="website" />`,
    `    <meta property="og:site_name" content="Boris Diff" />`,
    `    <meta property="og:title" content="${esc(title)}" />`,
    `    <meta property="og:description" content="${esc(desc)}" />`,
    `    <meta property="og:image" content="${esc(splash)}" />`,
    `    <meta property="og:url" content="${esc(url)}" />`,
    `    <link rel="canonical" href="${esc(url)}" />`,
    `    <meta name="twitter:card" content="summary_large_image" />`,
    `    <meta name="twitter:title" content="${esc(title)}" />`,
    `    <meta name="twitter:description" content="${esc(desc)}" />`,
    `    <meta name="twitter:image" content="${esc(splash)}" />`,
    `    ${END}`,
  ].join('\n');
}

const before = template.slice(0, template.indexOf(START));
const after = template.slice(template.indexOf(END) + END.length);

let count = 0;
for (const champ of champions) {
  const html = before + metaBlock(champ) + after;
  const outDir = resolve(distDir, 'c', champ.id);
  mkdirSync(outDir, { recursive: true });
  writeFileSync(resolve(outDir, 'index.html'), html);
  count++;
}

console.log(`[prerender] wrote ${count} champion pages to dist/c/<id>/index.html`);

// sitemap.xml — homepage + every champion page, for search-engine indexing.
const today = new Date().toISOString().slice(0, 10);
const urls = [
  { loc: `${ORIGIN}/`, priority: '1.0' },
  ...champions.map((c) => ({ loc: `${ORIGIN}/c/${c.id}`, priority: '0.8' })),
];
const sitemap =
  `<?xml version="1.0" encoding="UTF-8"?>\n` +
  `<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n` +
  urls
    .map(
      (u) =>
        `  <url><loc>${u.loc}</loc><lastmod>${today}</lastmod><priority>${u.priority}</priority></url>`,
    )
    .join('\n') +
  `\n</urlset>\n`;
writeFileSync(resolve(distDir, 'sitemap.xml'), sitemap);

// robots.txt — allow everything, point crawlers at the sitemap.
writeFileSync(
  resolve(distDir, 'robots.txt'),
  `User-agent: *\nAllow: /\n\nSitemap: ${ORIGIN}/sitemap.xml\n`,
);

console.log(`[prerender] wrote sitemap.xml (${urls.length} urls) and robots.txt`);

import { useEffect, useRef } from 'react';
import { championList } from '../data';
import { COMBO_TEMPLATES } from '../data/combo-templates';
import meta from '../data/generated/meta.json';
import AdBanner from './AdBanner';

const DDRAGON_IMG = `https://ddragon.leagueoflegends.com/cdn/${meta.version}/img`;
const SUPPORTED = Object.keys(COMBO_TEMPLATES);
const DISCORD_URL = 'https://discord.gg/qw3xeUfN48';

const IS_LOCAL = window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1';

function AdBanner() {
  const pushed = useRef(false);
  useEffect(() => {
    if (IS_LOCAL || pushed.current) return;
    pushed.current = true;
    try { (window.adsbygoogle = window.adsbygoogle || []).push({}); } catch { /* noop */ }
  }, []);

  if (IS_LOCAL) {
    return (
      <div style={{ margin: '24px 0', background: '#1a3a5c', border: '2px dashed #3b82f6', borderRadius: 8, padding: '24px', textAlign: 'center', color: '#60a5fa', fontSize: 14 }}>
        Ad Banner Placeholder (only visible on localhost)
      </div>
    );
  }

  return (
    <div style={{ margin: '24px 0' }}>
      <ins
        className="adsbygoogle"
        style={{ display: 'block' }}
        data-ad-client="ca-pub-8650041218363700"
        data-ad-slot="6830278012"
        data-ad-format="auto"
        data-full-width-responsive="true"
      />
    </div>
  );
}

export default function LandingPage({ onSelect }) {
  const supported = SUPPORTED.map((id) => championList.find((c) => c.id === id)).filter(Boolean);

  return (
    <div className="landing">
      <div className="landing-hero">
        <h2>Choose a Champion</h2>
        <p>Build combos, compare damage, and theorycraft item builds.</p>
      </div>

      <div className="landing-grid">
        {supported.map((c) => (
          <button
            key={c.id}
            className="champ-card"
            onClick={() => onSelect(c.id)}
            title={c.name}
          >
            <img
              src={`${DDRAGON_IMG}/champion/${c.id}.png`}
              alt={c.name}
              loading="lazy"
            />
            <span className="champ-card-name">{c.name}</span>
          </button>
        ))}
      </div>

      <AdBanner />

      <div className="landing-request">
        <h3>More champions coming soon</h3>
        <p>Want your main added next? Join our Discord and vote for the next champion.</p>
        <a
          className="landing-vote-btn"
          href={DISCORD_URL}
          target="_blank"
          rel="noopener noreferrer"
        >
          Join Discord
        </a>
      </div>

      <AdBanner slot="6830278012" label="Landing Bottom Banner" style={{ margin: '24px 0' }} />
    </div>
  );
}

import { championList } from '../data';
import { COMBO_TEMPLATES } from '../data/combo-templates';
import meta from '../data/generated/meta.json';

const DDRAGON_IMG = `https://ddragon.leagueoflegends.com/cdn/${meta.version}/img`;
const SUPPORTED = Object.keys(COMBO_TEMPLATES);
const VOTE_URL = 'https://github.com/teptnologic/testproject/issues/new?template=champion-request.md&title=Champion+Request:+';

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

      <div className="landing-request">
        <h3>More champions coming soon</h3>
        <p>Want your main added next? Vote for a champion and we'll prioritize it.</p>
        <a
          className="landing-vote-btn"
          href={VOTE_URL}
          target="_blank"
          rel="noopener noreferrer"
        >
          Request a Champion
        </a>
      </div>
    </div>
  );
}

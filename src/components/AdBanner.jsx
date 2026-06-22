import { useEffect, useRef } from 'react';

const IS_LOCAL = window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1';

export default function AdBanner({ slot, label, style }) {
  const pushed = useRef(false);
  useEffect(() => {
    if (IS_LOCAL || pushed.current) return;
    pushed.current = true;
    try { (window.adsbygoogle = window.adsbygoogle || []).push({}); } catch { /* noop */ }
  }, []);

  if (IS_LOCAL) {
    return (
      <div style={{ background: '#1a3a5c', border: '2px dashed #3b82f6', borderRadius: 8, padding: '24px', textAlign: 'center', color: '#60a5fa', fontSize: 14, ...style }}>
        {label || 'Ad Placeholder'} (localhost)
      </div>
    );
  }

  return (
    <div style={style}>
      <ins
        className="adsbygoogle"
        style={{ display: 'block' }}
        data-ad-client="ca-pub-8650041218363700"
        data-ad-slot={slot}
        data-ad-format="auto"
        data-full-width-responsive="true"
      />
    </div>
  );
}

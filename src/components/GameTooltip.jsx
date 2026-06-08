import { useState, useRef, useEffect } from 'react';
import { createPortal } from 'react-dom';

export default function GameTooltip({ title, subtitle, html, children }) {
  const anchorRef = useRef(null);
  const [visible, setVisible] = useState(false);
  const [pos, setPos] = useState({ top: 0, left: 0 });

  useEffect(() => {
    if (!visible || !anchorRef.current) return;

    function place() {
      const rect = anchorRef.current.getBoundingClientRect();
      const tooltipW = 320;
      const margin = 8;
      let left = rect.left;
      let top = rect.bottom + margin;

      if (left + tooltipW > window.innerWidth - margin) {
        left = window.innerWidth - tooltipW - margin;
      }
      if (left < margin) left = margin;

      const estimatedH = 200;
      if (top + estimatedH > window.innerHeight - margin) {
        top = rect.top - estimatedH - margin;
        if (top < margin) top = margin;
      }

      setPos({ top, left });
    }

    place();
    window.addEventListener('scroll', place, true);
    window.addEventListener('resize', place);
    return () => {
      window.removeEventListener('scroll', place, true);
      window.removeEventListener('resize', place);
    };
  }, [visible, html, title]);

  if (!html && !title) {
    return <span className="tooltip-anchor">{children}</span>;
  }

  return (
    <>
      <span
        ref={anchorRef}
        className="tooltip-anchor"
        onMouseEnter={() => setVisible(true)}
        onMouseLeave={() => setVisible(false)}
      >
        {children}
      </span>
      {visible &&
        createPortal(
          <div
            className="game-tooltip"
            style={{ top: pos.top, left: pos.left }}
            onMouseEnter={() => setVisible(true)}
            onMouseLeave={() => setVisible(false)}
          >
            {title && <div className="game-tooltip-title">{title}</div>}
            {subtitle && <div className="game-tooltip-subtitle">{subtitle}</div>}
            {html && (
              <div
                className="game-tooltip-body"
                dangerouslySetInnerHTML={{ __html: html }}
              />
            )}
          </div>,
          document.body,
        )}
    </>
  );
}

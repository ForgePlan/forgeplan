import { useState, useEffect, useRef } from 'react';
import { COLORS, octPoints } from '../tokens';

/**
 * TrustSection — R_eff scoring rings.
 * Uses CSS sticky (not GSAP pin) to avoid multi-pin conflicts.
 * Section is 200vh tall, content sticks at top for scroll room.
 */
export default function TrustSection() {
  const [progress, setProgress] = useState(0);
  const sectionRef = useRef<HTMLElement>(null);

  useEffect(() => {
    const el = sectionRef.current;
    if (!el) return;

    function onScroll() {
      const rect = el!.getBoundingClientRect();
      const sectionHeight = el!.offsetHeight;
      const vh = window.innerHeight;
      // How far we've scrolled through the section
      // 0 = section top just reached viewport top
      // 1 = section bottom is at viewport bottom
      const scrolled = -rect.top;
      const scrollRange = sectionHeight - vh;
      if (scrollRange <= 0) return;
      const raw = scrolled / scrollRange;
      setProgress(Math.max(0, Math.min(1, raw)));
    }

    window.addEventListener('scroll', onScroll, { passive: true });
    onScroll();
    return () => window.removeEventListener('scroll', onScroll);
  }, []);

  const CX = 470, CY = 450;
  const rings = [
    { r: 350, color: COLORS.fg, width: 0.5, dashArray: '4 6', maxOpacity: 0.08, start: 0 },
    { r: 300, color: COLORS.fg, width: 1.5, maxOpacity: 0.15, start: 0.1 },
    { r: 240, color: COLORS.fg, width: 1.2, maxOpacity: 0.25, start: 0.2 },
    { r: 180, color: COLORS.fg, width: 1, maxOpacity: 0.35, start: 0.3 },
    { r: 120, color: COLORS.ember, width: 1, maxOpacity: 0.5, start: 0.4 },
    { r: 60, color: COLORS.ember, width: 0, maxOpacity: 0.15, start: 0.5, fill: true },
  ];

  const textOpacity = (start: number) => Math.min(Math.max((progress - start) / 0.15, 0), 1);

  return (
    <section id="trust" ref={sectionRef} className="relative w-full bg-forge-bg border-b border-forge-line" style={{ height: '200vh' }}>
      {/* Sticky content — stays on screen while we scroll through 200vh */}
      <div className="sticky top-0 h-screen overflow-hidden">
        <div className="grid grid-cols-1 lg:grid-cols-[1fr_500px] h-full pt-[36px]">
          <div className="relative flex items-center justify-center border-r border-forge-line overflow-hidden">
            <div className="absolute inset-0 opacity-25 bg-dot-grid" aria-hidden="true" />
            <svg className="w-full max-w-[700px] h-auto" viewBox="0 0 940 900" fill="none" aria-hidden="true">
              {rings.map((ring, i) => {
                const appear = Math.min(Math.max((progress - ring.start) / 0.2, 0), 1);
                const eased = 1 - Math.pow(1 - appear, 2);
                const scale = 2 - eased;
                const currentR = ring.r * scale;
                const opacity = ring.maxOpacity * eased;
                if (ring.fill) {
                  return <polygon key={i} points={octPoints(CX, CY, currentR)} fill={ring.color} opacity={opacity} />;
                }
                return (
                  <polygon key={i} points={octPoints(CX, CY, currentR)}
                    stroke={ring.color} strokeWidth={ring.width} fill="none"
                    strokeDasharray={ring.dashArray} opacity={opacity} />
                );
              })}
              <circle cx={CX} cy={CY} r={10} fill={COLORS.ember}
                opacity={0.9 * Math.min(Math.max((progress - 0.5) / 0.15, 0), 1)} />
              <text x={CX} y={CY + 35} textAnchor="middle"
                fontFamily="Geist Mono, monospace" fontSize={12} fill={COLORS.ember}
                opacity={Math.min(Math.max((progress - 0.55) / 0.1, 0), 1)}>
                R_eff = 0.82
              </text>
            </svg>
          </div>

          <div className="flex flex-col justify-between p-8 lg:p-12">
            <h2 className="font-heading text-4xl lg:text-[68px] font-normal leading-[1.05]">
              Trust Is<br />Measured<br />Not<br />Assumed
            </h2>
            <hr className="border-forge-line my-8" />
            <div className="space-y-8">
              <div style={{ opacity: textOpacity(0.4) }}>
                <p className="font-mono text-base font-medium text-forge-ember">R_eff = min(evidence)</p>
                <p className="text-sm text-forge-dim leading-relaxed mt-1">Weakest-link scoring. Your decision is only as strong as your weakest evidence.</p>
              </div>
              <hr className="border-forge-line" />
              <div style={{ opacity: textOpacity(0.55) }}>
                <p className="text-base font-medium">Evidence Decay</p>
                <p className="text-sm text-forge-dim leading-relaxed mt-1">Evidence has a TTL. Expired evidence scores 0.1 — stale, not absent.</p>
              </div>
              <hr className="border-forge-line" />
              <div style={{ opacity: textOpacity(0.7) }}>
                <p className="text-base font-medium">Congruence Levels</p>
                <p className="text-sm text-forge-dim leading-relaxed mt-1">Evidence from the same context scores highest. Each ring = a confidence level.</p>
              </div>
            </div>
            <p className="font-mono text-[10px] tracking-[3px] text-forge-dim mt-8" style={{ opacity: textOpacity(0.8) }}>EVIDENCE</p>
          </div>
        </div>
      </div>
    </section>
  );
}

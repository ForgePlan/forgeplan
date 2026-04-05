import { useState } from 'react';
import StickySection from './StickySection';
import { COLORS, octPoints } from '../tokens';

export default function TrustSection() {
  const [progress, setProgress] = useState(0);

  const CX = 470, CY = 450;
  const rings = [
    { r: 350, color: COLORS.fg, width: 0.5, dashArray: '4 6', maxOpacity: 0.08, start: 0 },
    { r: 300, color: COLORS.fg, width: 1.5, maxOpacity: 0.15, start: 0.05 },
    { r: 240, color: COLORS.fg, width: 1.2, maxOpacity: 0.25, start: 0.15 },
    { r: 180, color: COLORS.fg, width: 1, maxOpacity: 0.35, start: 0.25 },
    { r: 120, color: COLORS.ember, width: 1, maxOpacity: 0.5, start: 0.35 },
    { r: 60, color: COLORS.ember, width: 0, maxOpacity: 0.15, start: 0.45, fill: true },
  ];

  // Text elements fade in at different scroll points
  const textOpacity = (start: number) => Math.min(Math.max((progress - start) / 0.15, 0), 1);

  return (
    <StickySection id="trust" scrollMultiplier={2} onProgress={setProgress} className="border-b border-forge-line">
      <div className="grid grid-cols-1 lg:grid-cols-[1fr_500px] h-screen pt-[36px]">
        {/* Left: Scoring rings SVG */}
        <div className="relative flex items-center justify-center border-r border-forge-line overflow-hidden">
          <div className="absolute inset-0 opacity-25 bg-dot-grid" aria-hidden="true" />
          <svg className="w-full max-w-[700px] h-auto" viewBox="0 0 940 900" fill="none" aria-hidden="true">
            {rings.map((ring, i) => {
              const appear = Math.min(Math.max((progress - ring.start) / 0.2, 0), 1);
              const eased = 1 - Math.pow(1 - appear, 2);
              // Rings scale from 2x to 1x (fly in from outside)
              const scale = 2 - eased;
              const currentR = ring.r * scale;
              const opacity = ring.maxOpacity * eased;

              if (ring.fill) {
                return (
                  <polygon key={i} points={octPoints(CX, CY, currentR)}
                    fill={ring.color} opacity={opacity} />
                );
              }
              return (
                <polygon key={i} points={octPoints(CX, CY, currentR)}
                  stroke={ring.color} strokeWidth={ring.width} fill="none"
                  strokeDasharray={ring.dashArray} opacity={opacity} />
              );
            })}

            {/* Center dot */}
            <circle cx={CX} cy={CY} r={10} fill={COLORS.ember}
              opacity={0.9 * Math.min(Math.max((progress - 0.5) / 0.15, 0), 1)} />

            {/* R_eff label */}
            <text x={CX} y={CY + 35} textAnchor="middle"
              fontFamily="Geist Mono, monospace" fontSize={12} fill={COLORS.ember}
              opacity={Math.min(Math.max((progress - 0.55) / 0.1, 0), 1)}>
              R_eff = 0.82
            </text>
          </svg>
        </div>

        {/* Right: Content */}
        <div className="flex flex-col justify-between p-8 lg:p-12">
          <h2 className="font-heading text-4xl lg:text-[68px] font-normal leading-[1.05]">
            Trust Is<br />Measured<br />Not<br />Assumed
          </h2>

          <hr className="border-forge-line my-8" />

          <div className="space-y-8">
            <div style={{ opacity: textOpacity(0.4) }}>
              <p className="font-mono text-base font-medium text-forge-ember">R_eff = min(evidence)</p>
              <p className="text-sm text-forge-dim leading-relaxed mt-1">
                Weakest-link scoring. Your decision is only as strong as your weakest evidence.
              </p>
            </div>
            <hr className="border-forge-line" />
            <div style={{ opacity: textOpacity(0.55) }}>
              <p className="text-base font-medium">Evidence Decay</p>
              <p className="text-sm text-forge-dim leading-relaxed mt-1">
                Evidence has a TTL. Expired evidence scores 0.1 — stale, not absent. Dashed outer ring = decay zone.
              </p>
            </div>
            <hr className="border-forge-line" />
            <div style={{ opacity: textOpacity(0.7) }}>
              <p className="text-base font-medium">Congruence Levels</p>
              <p className="text-sm text-forge-dim leading-relaxed mt-1">
                Evidence from the same context scores highest. Opposing context gets penalized. Each ring represents a confidence level.
              </p>
            </div>
          </div>

          <p className="font-mono text-[10px] tracking-[3px] text-forge-dim mt-8"
            style={{ opacity: textOpacity(0.8) }}>
            EVIDENCE
          </p>
        </div>
      </div>
    </StickySection>
  );
}

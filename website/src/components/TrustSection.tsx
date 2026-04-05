import { useState, useEffect, useRef } from 'react';
import { COLORS, octPoints } from '../tokens';

// Mini dashboard artifacts — show real R_eff scores
const ARTIFACTS_DEMO = [
  { id: 'PRD-018', score: 0.82, evidence: 3, status: 'healthy' },
  { id: 'RFC-003', score: 0.41, evidence: 1, status: 'weak' },
  { id: 'ADR-001', score: 0.00, evidence: 0, status: 'blind spot' },
];

// Congruence level labels for rings
const CL_LABELS = ['decay zone', 'CL0', 'CL1', 'CL2', 'CL3'];

export default function TrustSection() {
  const [progress, setProgress] = useState(0);
  const sectionRef = useRef<HTMLElement>(null);

  useEffect(() => {
    const el = sectionRef.current;
    if (!el) return;
    function onScroll() {
      const rect = el!.getBoundingClientRect();
      const scrollRange = el!.offsetHeight - window.innerHeight;
      if (scrollRange <= 0) return;
      setProgress(Math.max(0, Math.min(1, -rect.top / scrollRange)));
    }
    window.addEventListener('scroll', onScroll, { passive: true });
    onScroll();
    return () => window.removeEventListener('scroll', onScroll);
  }, []);

  const CX = 470, CY = 450;
  const rings = [
    { r: 350, color: COLORS.fg, width: 0.5, dashArray: '4 6', maxOpacity: 0.08, start: 0 },
    { r: 300, color: COLORS.fg, width: 1.5, maxOpacity: 0.15, start: 0.08 },
    { r: 240, color: COLORS.fg, width: 1.2, maxOpacity: 0.25, start: 0.16 },
    { r: 180, color: COLORS.fg, width: 1, maxOpacity: 0.35, start: 0.24 },
    { r: 120, color: COLORS.ember, width: 1, maxOpacity: 0.5, start: 0.32 },
    { r: 60, color: COLORS.ember, width: 0, maxOpacity: 0.15, start: 0.40, fill: true },
  ];

  const fade = (start: number, dur = 0.12) => Math.min(Math.max((progress - start) / dur, 0), 1);

  // Title lines appear one by one
  const titleLines = ['Trust Is', 'Measured', 'Not', 'Assumed'];
  const titleStarts = [0.02, 0.08, 0.14, 0.20];

  return (
    <section id="trust" ref={sectionRef} className="relative w-full bg-forge-bg border-b border-forge-line" style={{ height: '250vh' }}>
      <div className="sticky top-[36px] overflow-hidden" style={{ height: 'calc(100vh - 36px)' }}>
        <div className="grid grid-cols-1 lg:grid-cols-[1fr_500px] h-full">

          {/* Left: Scoring rings + CL labels */}
          <div className="relative flex items-center justify-center border-r border-forge-line overflow-hidden">
            <div className="absolute inset-0 opacity-25 bg-dot-grid" aria-hidden="true" />
            <svg className="w-full max-w-[700px] h-auto" viewBox="0 0 940 900" fill="none" aria-hidden="true">
              {rings.map((ring, i) => {
                const appear = Math.min(Math.max((progress - ring.start) / 0.15, 0), 1);
                const eased = 1 - Math.pow(1 - appear, 2);
                const scale = 2 - eased;
                const currentR = ring.r * scale;
                const opacity = ring.maxOpacity * eased;

                return ring.fill ? (
                  <polygon key={i} points={octPoints(CX, CY, currentR)} fill={ring.color} opacity={opacity} />
                ) : (
                  <polygon key={i} points={octPoints(CX, CY, currentR)}
                    stroke={ring.color} strokeWidth={ring.width} fill="none"
                    strokeDasharray={ring.dashArray} opacity={opacity} />
                );
              })}

              {/* CL labels between rings */}
              {CL_LABELS.map((label, i) => {
                const labelR = [350, 300, 240, 180, 120][i];
                const nextR = [300, 240, 180, 120, 60][i];
                const midR = (labelR + nextR) / 2;
                const labelOpacity = fade(rings[i]?.start || 0.4, 0.15);
                return (
                  <text key={`cl-${i}`} x={CX + midR * 0.7} y={CY - midR * 0.1}
                    fontFamily="Geist Mono, monospace" fontSize={9}
                    fill={i === 4 ? COLORS.ember : COLORS.dim}
                    opacity={labelOpacity * 0.6}
                    textAnchor="middle">
                    {label}
                  </text>
                );
              })}

              {/* Center dot */}
              <circle cx={CX} cy={CY} r={10} fill={COLORS.ember}
                opacity={0.9 * fade(0.45)} />

              {/* R_eff score */}
              <text x={CX} y={CY + 35} textAnchor="middle"
                fontFamily="Geist Mono, monospace" fontSize={14} fill={COLORS.ember}
                opacity={fade(0.50)}>
                R_eff = 0.82
              </text>

              {/* Formula under score */}
              <text x={CX} y={CY + 55} textAnchor="middle"
                fontFamily="Geist Mono, monospace" fontSize={10} fill={COLORS.dim}
                opacity={fade(0.55)}>
                min(evidence scores)
              </text>
            </svg>
          </div>

          {/* Right: Title + dashboard + explanations */}
          <div className="flex flex-col justify-between p-6 lg:p-10 overflow-hidden">

            {/* Title — appears line by line */}
            <div>
              {titleLines.map((line, i) => (
                <h2 key={i}
                  className="font-heading text-4xl lg:text-[60px] font-normal leading-[1.05]"
                  style={{
                    opacity: fade(titleStarts[i], 0.06),
                    transform: `translateY(${(1 - fade(titleStarts[i], 0.06)) * 15}px)`,
                  }}>
                  {line}
                </h2>
              ))}
            </div>

            {/* Separator */}
            <hr className="border-forge-line my-4" style={{ opacity: fade(0.25) }} />

            {/* Mini artifact dashboard */}
            <div className="space-y-3" style={{ opacity: fade(0.35) }}>
              <p className="font-mono text-[10px] tracking-[2px] text-forge-ember">LIVE SCORING</p>
              {ARTIFACTS_DEMO.map((art, i) => {
                const barFill = fade(0.40 + i * 0.06);
                const barWidth = art.score * 100 * barFill;
                const isRisk = art.score === 0;
                return (
                  <div key={art.id} className="flex items-center gap-3"
                    style={{ opacity: fade(0.38 + i * 0.05) }}>
                    <span className="font-mono text-xs text-forge-fg w-[72px]">{art.id}</span>
                    <div className="flex-1 h-[6px] bg-forge-surface border border-forge-line relative">
                      <div
                        className="absolute inset-y-0 left-0"
                        style={{
                          width: `${barWidth}%`,
                          backgroundColor: isRisk ? 'var(--forge-dim)' : art.score > 0.6 ? '#28C840' : COLORS.ember,
                        }}
                      />
                    </div>
                    <span className={`font-mono text-[10px] w-[36px] text-right ${isRisk ? 'text-forge-ember' : 'text-forge-dim'}`}>
                      {(art.score * barFill).toFixed(2)}
                    </span>
                    <span className="font-mono text-[9px] text-forge-dim w-[70px]">
                      {art.evidence} evidence
                    </span>
                    {isRisk && (
                      <span className="font-mono text-[8px] text-forge-ember tracking-wider">BLIND SPOT</span>
                    )}
                  </div>
                );
              })}
            </div>

            {/* Explanations — storytelling, not definitions */}
            <div className="space-y-4 mt-4">
              <div style={{ opacity: fade(0.55) }}>
                <p className="text-sm text-forge-fg leading-relaxed">
                  Your decision is only as strong as your <span className="text-forge-ember font-medium">weakest evidence</span>.
                  Not average — weakest. One untested assumption drags everything down.
                </p>
              </div>
              <hr className="border-forge-line" style={{ opacity: fade(0.60) }} />
              <div style={{ opacity: fade(0.65) }}>
                <p className="text-sm text-forge-fg leading-relaxed">
                  Last tested 6 months ago? <span className="text-forge-ember font-medium">Score drops</span>.
                  Evidence expires — silently. The dashed outer ring is the decay zone.
                </p>
              </div>
              <hr className="border-forge-line" style={{ opacity: fade(0.72) }} />
              <div style={{ opacity: fade(0.78) }}>
                <p className="text-sm text-forge-fg leading-relaxed">
                  Evidence from the <span className="text-forge-ember font-medium">same context</span> scores highest.
                  Opposing context? Penalized. Each ring = one confidence level.
                </p>
              </div>
            </div>

            <p className="font-mono text-[10px] tracking-[3px] text-forge-dim mt-4" style={{ opacity: fade(0.85) }}>
              EVIDENCE TRACKING
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}

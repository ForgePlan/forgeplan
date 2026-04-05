import { useState, useEffect, useRef } from 'react';
import { COLORS, octPoints, octVertex } from '../tokens';

const ARTIFACTS_DEMO = [
  { id: 'PRD-018', score: 0.82, evidence: 3, status: 'healthy' },
  { id: 'RFC-003', score: 0.41, evidence: 1, status: 'weak' },
  { id: 'ADR-001', score: 0.00, evidence: 0, status: 'blind' },
];

// Ring annotations — labels with dashed leaders from rings
const RING_ANNOTATIONS = [
  { ringIdx: 0, label: 'Evidence expires silently', detail: 'valid_until TTL → score 0.1', angle: -30, dist: 60 },
  { ringIdx: 1, label: 'Opposed context', detail: 'CL0 — penalty 0.9', angle: 30, dist: 50 },
  { ringIdx: 2, label: 'External docs only', detail: 'CL1 — penalty 0.4', angle: -15, dist: 40 },
  { ringIdx: 3, label: 'Related project', detail: 'CL2 — penalty 0.1', angle: 20, dist: 30 },
  { ringIdx: 4, label: 'Same context', detail: 'CL3 — no penalty', angle: -25, dist: 20 },
];

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

  const CX = 470, CY = 420;
  const ringRadii = [320, 270, 220, 170, 120, 60];
  const rings = [
    { r: ringRadii[0], color: COLORS.fg, width: 0.5, dashArray: '4 6', maxOpacity: 0.08, start: 0.02 },
    { r: ringRadii[1], color: COLORS.fg, width: 1.2, maxOpacity: 0.15, start: 0.08 },
    { r: ringRadii[2], color: COLORS.fg, width: 1, maxOpacity: 0.22, start: 0.14 },
    { r: ringRadii[3], color: COLORS.fg, width: 0.8, maxOpacity: 0.30, start: 0.20 },
    { r: ringRadii[4], color: COLORS.ember, width: 1, maxOpacity: 0.5, start: 0.26 },
    { r: ringRadii[5], color: COLORS.ember, width: 0, maxOpacity: 0.15, start: 0.32, fill: true },
  ];

  const fade = (start: number, dur = 0.10) => Math.min(Math.max((progress - start) / dur, 0), 1);

  const titleLines = ['Trust Is', 'Measured,', 'Not', 'Assumed'];
  const titleStarts = [0.02, 0.07, 0.12, 0.17];

  return (
    <section id="trust" ref={sectionRef} className="relative w-full bg-forge-bg border-b border-forge-line" style={{ height: '280vh' }}>
      <div className="sticky top-[36px] overflow-hidden" style={{ height: 'calc(100vh - 36px)' }}>
        <div className="grid grid-cols-1 lg:grid-cols-[1fr_480px] h-full">

          {/* Left: Scoring rings + annotations */}
          <div className="relative flex items-center justify-center border-r border-forge-line overflow-hidden">
            <div className="absolute inset-0 opacity-25 bg-dot-grid" aria-hidden="true" />
            <svg className="w-full max-w-[750px] h-auto" viewBox="0 0 940 840" fill="none" aria-hidden="true">

              {/* Rings */}
              {rings.map((ring, i) => {
                const appear = Math.min(Math.max((progress - ring.start) / 0.12, 0), 1);
                const eased = 1 - Math.pow(1 - appear, 2);
                const scale = 2.5 - 1.5 * eased;
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

              {/* Ring annotations — dashed leader lines + labels */}
              {RING_ANNOTATIONS.map((ann, ai) => {
                const annOpacity = fade(rings[ann.ringIdx].start + 0.08, 0.12);
                if (annOpacity <= 0) return null;
                const r = ringRadii[ann.ringIdx];
                const angleRad = (ann.angle * Math.PI) / 180;
                // Point on ring
                const px = CX + r * Math.cos(angleRad);
                const py = CY + r * Math.sin(angleRad);
                // Label position (outside ring)
                const lx = CX + (r + ann.dist + 60) * Math.cos(angleRad);
                const ly = CY + (r + ann.dist + 60) * Math.sin(angleRad);
                const textAnchor = lx > CX ? 'start' : 'end';

                return (
                  <g key={`ann-${ai}`} opacity={annOpacity}>
                    {/* Dashed leader line */}
                    <line x1={px} y1={py} x2={lx} y2={ly}
                      stroke={ann.ringIdx === 4 ? COLORS.ember : COLORS.dim}
                      strokeWidth={0.5} strokeDasharray="3 3" />
                    {/* Dot at ring end */}
                    <circle cx={px} cy={py} r={2}
                      fill={ann.ringIdx === 4 ? COLORS.ember : COLORS.dim} />
                    {/* Label */}
                    <text x={lx} y={ly - 4} textAnchor={textAnchor}
                      fontFamily="Geist Mono, monospace" fontSize={9}
                      fill={ann.ringIdx === 4 ? COLORS.ember : COLORS.fg}>
                      {ann.label}
                    </text>
                    {/* Detail */}
                    <text x={lx} y={ly + 10} textAnchor={textAnchor}
                      fontFamily="Geist Mono, monospace" fontSize={8}
                      fill={COLORS.dim}>
                      {ann.detail}
                    </text>
                  </g>
                );
              })}

              {/* Center dot */}
              <circle cx={CX} cy={CY} r={10} fill={COLORS.ember} opacity={0.9 * fade(0.38)} />

              {/* R_eff score */}
              <text x={CX} y={CY + 32} textAnchor="middle"
                fontFamily="Geist Mono, monospace" fontSize={14} fill={COLORS.ember}
                opacity={fade(0.42)}>
                R_eff = 0.82
              </text>
              <text x={CX} y={CY + 48} textAnchor="middle"
                fontFamily="Geist Mono, monospace" fontSize={9} fill={COLORS.dim}
                opacity={fade(0.45)}>
                min(evidence scores)
              </text>
            </svg>
          </div>

          {/* Right: Title + dashboard + explanations */}
          <div className="flex flex-col justify-between p-5 lg:p-8 overflow-hidden">

            {/* Title — line by line */}
            <div>
              {titleLines.map((line, i) => (
                <h2 key={i} className="font-heading text-3xl lg:text-[52px] font-normal leading-[1.1]"
                  style={{
                    opacity: fade(titleStarts[i], 0.05),
                    transform: `translateY(${(1 - fade(titleStarts[i], 0.05)) * 12}px)`,
                  }}>
                  {line}
                </h2>
              ))}
            </div>

            <hr className="border-forge-line my-3" style={{ opacity: fade(0.22) }} />

            {/* Mini artifact dashboard */}
            <div style={{ opacity: fade(0.30) }}>
              <p className="font-mono text-[10px] tracking-[2px] text-forge-ember mb-3">LIVE SCORING</p>
              <div className="space-y-2">
                {ARTIFACTS_DEMO.map((art, i) => {
                  const barFill = fade(0.34 + i * 0.05);
                  const barWidth = Math.max(art.score * 100 * barFill, art.score === 0 ? 0 : 2);
                  const isBlind = art.score === 0;
                  const barColor = isBlind ? 'var(--forge-dim)' : art.score > 0.6 ? '#28C840' : '#FF6B35';

                  return (
                    <div key={art.id} className="grid grid-cols-[70px_1fr_40px_65px] gap-2 items-center"
                      style={{ opacity: fade(0.32 + i * 0.04) }}>
                      <span className="font-mono text-[11px] text-forge-fg">{art.id}</span>
                      <div className="h-[5px] bg-forge-surface border border-forge-line relative overflow-hidden">
                        <div className="absolute inset-y-0 left-0 transition-all duration-500"
                          style={{ width: `${barWidth}%`, backgroundColor: barColor }} />
                        {/* Loader shimmer */}
                        {barFill < 1 && (
                          <div className="absolute inset-0 bg-gradient-to-r from-transparent via-forge-fg/5 to-transparent animate-pulse" />
                        )}
                      </div>
                      <span className={`font-mono text-[10px] text-right ${isBlind ? 'text-forge-ember' : 'text-forge-dim'}`}>
                        {(art.score * barFill).toFixed(2)}
                      </span>
                      <span className="font-mono text-[9px] text-forge-dim">
                        {isBlind ? (
                          <span className="text-forge-ember">NO EVIDENCE</span>
                        ) : (
                          `${art.evidence} evidences`
                        )}
                      </span>
                    </div>
                  );
                })}
              </div>
            </div>

            {/* Storytelling */}
            <div className="space-y-3 mt-3">
              <div style={{ opacity: fade(0.50) }}>
                <p className="text-sm text-forge-fg leading-relaxed">
                  Your decision is only as strong as your <span className="text-forge-ember font-medium">weakest evidence</span>.
                  Not average — minimum. One untested assumption drags the whole score down.
                </p>
              </div>
              <hr className="border-forge-line" style={{ opacity: fade(0.57) }} />
              <div style={{ opacity: fade(0.62) }}>
                <p className="text-sm text-forge-fg leading-relaxed">
                  Last benchmarked 6 months ago? <span className="text-forge-ember font-medium">Score drops to 0.1</span>.
                  Evidence expires silently — the outer dashed ring is the decay zone.
                </p>
              </div>
              <hr className="border-forge-line" style={{ opacity: fade(0.70) }} />
              <div style={{ opacity: fade(0.77) }}>
                <p className="text-sm text-forge-fg leading-relaxed">
                  Test from the <span className="text-forge-ember font-medium">same context</span> = full trust (CL3).
                  Stack Overflow answer = CL1 penalty.
                  Each ring maps to a confidence level.
                </p>
              </div>
            </div>

            <p className="font-mono text-[10px] tracking-[3px] text-forge-dim mt-3" style={{ opacity: fade(0.85) }}>
              EVIDENCE TRACKING
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}

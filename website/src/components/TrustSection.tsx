import { useState, useEffect, useRef } from 'react';
import { COLORS, octPoints, octVertex } from '../tokens';

// 5 story cards — each connected to a ring, answers a real question
const STORY_CARDS = [
  {
    title: 'Still valid?',
    text: 'That benchmark from last quarter scored 0.9 when you ran it. But valid_until expired. Now it\'s 0.1 — stale, not deleted.',
    detail: 'EVIDENCE DECAY',
    ringIdx: 0,
    side: 'left' as const,
    vertexIdx: 7,
    start: 0.10,
  },
  {
    title: 'Source matters',
    text: 'A Stack Overflow answer ≠ your own load test. Same conclusion, different trust. CL0 vs CL3 = 0.9 penalty gap.',
    detail: 'CONTEXT LEVEL',
    ringIdx: 1,
    side: 'right' as const,
    vertexIdx: 1,
    start: 0.22,
  },
  {
    title: 'Weakest link wins',
    text: '3 strong evidences + 1 weak = weak decision. R_eff = min(), never average. One blind spot drags everything.',
    detail: 'R_EFF FORMULA',
    ringIdx: 4,
    side: 'left' as const,
    vertexIdx: 6,
    start: 0.34,
  },
  {
    title: 'Prove it here',
    text: 'Evidence from the same project, same context, same conditions. No penalties. Full trust. This is CL3.',
    detail: 'SAME CONTEXT',
    ringIdx: 3,
    side: 'right' as const,
    vertexIdx: 3,
    start: 0.46,
  },
  {
    title: 'Tested where?',
    text: 'Colleague\'s PoC in another service ≠ your benchmark in your service. Related helps, but doesn\'t prove.',
    detail: 'CONGRUENCE',
    ringIdx: 2,
    side: 'left' as const,
    vertexIdx: 5,
    start: 0.58,
  },
];

// Dashboard artifacts
const DEMO_ARTIFACTS = [
  { id: 'PRD-018', score: 0.82, evidence: 3 },
  { id: 'RFC-003', score: 0.41, evidence: 1 },
  { id: 'ADR-001', score: 0.00, evidence: 0 },
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

  const fade = (start: number, dur = 0.10) => Math.min(Math.max((progress - start) / dur, 0), 1);

  // SVG dimensions (full width)
  const SVG_W = 1440, SVG_H = 800;
  const CX = SVG_W / 2, CY = SVG_H / 2;
  const ringRadii = [300, 250, 200, 150, 100];
  const ringDefs = [
    { r: ringRadii[0], color: COLORS.fg, w: 0.5, dash: '4 6', maxOp: 0.08, start: 0.05 },
    { r: ringRadii[1], color: COLORS.fg, w: 1.2, maxOp: 0.15, start: 0.12 },
    { r: ringRadii[2], color: COLORS.fg, w: 1, maxOp: 0.22, start: 0.19 },
    { r: ringRadii[3], color: COLORS.fg, w: 0.8, maxOp: 0.30, start: 0.26 },
    { r: ringRadii[4], color: COLORS.ember, w: 1, maxOp: 0.5, start: 0.33 },
  ];

  // Title lines
  const titleLines = ['Trust Is', 'Measured,', 'Not Assumed'];
  const titleStarts = [0.02, 0.06, 0.10];

  // Connector line endpoints
  // Left cards: right border ≈ 22% of viewport (left-8 + 300px max-w ≈ 332px on 1440)
  // Right cards: left border ≈ 78% of viewport (1440 - 8 - 300 ≈ 1108px on 1440)
  // Card height ~80px, center = top + 40px
  const cardEdges = STORY_CARDS.map((card, ci) => {
    const edgeX = card.side === 'left' ? 340 : SVG_W - 340;
    const topPct = 18 + ci * 14;
    const edgeY = (SVG_H * topPct) / 100 + 40; // vertical center of card
    const [ringX, ringY] = octVertex(CX, CY, ringRadii[Math.min(card.ringIdx, ringRadii.length - 1)], card.vertexIdx);
    return { edgeX, edgeY, ringX, ringY };
  });

  return (
    <section id="trust" ref={sectionRef} className="relative w-full bg-forge-bg border-b border-forge-line" style={{ height: '350vh' }}>
      <div className="sticky top-[36px] overflow-hidden" style={{ height: 'calc(100vh - 36px)' }}>

        {/* Title — top center, appears line by line */}
        <div className="absolute top-6 left-0 right-0 flex flex-col items-center z-20">
          {titleLines.map((line, i) => (
            <h2 key={i} className="font-heading text-3xl lg:text-[48px] font-normal leading-[1.1] text-center"
              style={{
                opacity: fade(titleStarts[i], 0.04),
                transform: `translateY(${(1 - fade(titleStarts[i], 0.04)) * 10}px)`,
              }}>
              {line}
            </h2>
          ))}
        </div>

        {/* Full-width SVG — rings + connector lines */}
        <svg className="absolute inset-0 w-full h-full" viewBox={`0 0 ${SVG_W} ${SVG_H}`}
          preserveAspectRatio="xMidYMid meet" aria-hidden="true">

          {/* Dot grid */}
          <rect width={SVG_W} height={SVG_H} fill="none" />

          {/* Rings */}
          {ringDefs.map((ring, i) => {
            const appear = Math.min(Math.max((progress - ring.start) / 0.12, 0), 1);
            const eased = 1 - Math.pow(1 - appear, 2);
            const scale = 2.5 - 1.5 * eased;
            const currentR = ring.r * scale;
            const opacity = ring.maxOp * eased;
            return (
              <polygon key={`ring-${i}`} points={octPoints(CX, CY, currentR)}
                stroke={ring.color} strokeWidth={ring.w} fill="none"
                strokeDasharray={ring.dash} opacity={opacity} />
            );
          })}

          {/* Center dot */}
          <circle cx={CX} cy={CY} r={10} fill={COLORS.ember} opacity={0.9 * fade(0.42)} />
          <text x={CX} y={CY + 30} textAnchor="middle"
            fontFamily="Geist Mono, monospace" fontSize={13} fill={COLORS.ember}
            opacity={fade(0.45)}>
            R_eff = 0.82
          </text>

          {/* Connector lines from cards to ring vertices */}
          {STORY_CARDS.map((card, ci) => {
            const connOp = fade(card.start, 0.08) * 0.5;
            if (connOp <= 0) return null;
            const { edgeX, edgeY, ringX, ringY } = cardEdges[ci];
            return (
              <g key={`conn-${ci}`} opacity={connOp}>
                <line x1={edgeX} y1={edgeY} x2={ringX} y2={ringY}
                  stroke={card.ringIdx === 4 ? COLORS.ember : COLORS.dim}
                  strokeWidth={0.8} strokeDasharray="4 4" />
                <circle cx={ringX} cy={ringY} r={3}
                  fill={card.ringIdx === 4 ? COLORS.ember : COLORS.dim} />
              </g>
            );
          })}
        </svg>

        {/* Story cards — HTML divs positioned over SVG */}
        {STORY_CARDS.map((card, ci) => {
          const cardOp = fade(card.start, 0.08);
          if (cardOp <= 0) return null;

          const isLeft = card.side === 'left';
          const topPx = `${18 + ci * 14}%`;

          return (
            <div key={`card-${ci}`}
              className={`absolute z-10 max-w-[300px] ${isLeft ? 'left-4 lg:left-8' : 'right-4 lg:right-8'}`}
              style={{
                top: topPx,
                opacity: cardOp,
                transform: `translateY(${(1 - cardOp) * 12}px)`,
              }}>
              <div className="border border-forge-line bg-forge-bg/95 backdrop-blur-sm px-4 py-3"
                style={{
                  borderLeft: isLeft ? '2px solid var(--color-forge-ember)' : undefined,
                  borderRight: !isLeft ? '2px solid var(--color-forge-ember)' : undefined,
                }}>
                <p className="font-heading text-base font-medium text-forge-fg">{card.title}</p>
                <p className="font-body text-xs leading-relaxed text-forge-dim mt-1">{card.text}</p>
                <div className="mt-2 flex items-center gap-1.5">
                  <div className="w-1.5 h-1.5 rounded-full bg-forge-ember" />
                  <span className="font-mono text-[8px] tracking-[2px] text-forge-dim">{card.detail}</span>
                </div>
              </div>
            </div>
          );
        })}

        {/* Dashboard — bottom overlay */}
        <div className="absolute bottom-4 left-0 right-0 flex justify-center z-20" style={{ opacity: fade(0.72) }}>
          <div className="border border-forge-line bg-forge-bg/95 backdrop-blur-sm px-6 py-4 max-w-[500px] w-full">
            <p className="font-mono text-[10px] tracking-[2px] text-forge-ember mb-3">LIVE SCORING</p>
            <div className="space-y-2">
              {DEMO_ARTIFACTS.map((art, i) => {
                const barFill = fade(0.76 + i * 0.04);
                const barWidth = Math.max(art.score * 100 * barFill, 0);
                const isBlind = art.score === 0;
                const barColor = isBlind ? 'transparent' : art.score > 0.6 ? '#28C840' : '#FF6B35';
                return (
                  <div key={art.id} className="grid grid-cols-[70px_1fr_36px_70px] gap-2 items-center"
                    style={{ opacity: fade(0.74 + i * 0.03) }}>
                    <span className="font-mono text-[11px] text-forge-fg">{art.id}</span>
                    <div className="h-[5px] bg-forge-surface border border-forge-line relative overflow-hidden">
                      <div className="absolute inset-y-0 left-0" style={{ width: `${barWidth}%`, backgroundColor: barColor }} />
                      {barFill < 1 && <div className="absolute inset-0 bg-gradient-to-r from-transparent via-forge-fg/5 to-transparent animate-pulse" />}
                    </div>
                    <span className={`font-mono text-[10px] text-right ${isBlind ? 'text-forge-ember' : 'text-forge-dim'}`}>
                      {(art.score * barFill).toFixed(2)}
                    </span>
                    <span className="font-mono text-[9px] text-forge-dim">
                      {isBlind ? <span className="text-forge-ember">BLIND SPOT</span> : `${art.evidence} evidence`}
                    </span>
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

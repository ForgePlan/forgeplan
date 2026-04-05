import { useState, useEffect, useRef } from 'react';
import { COLORS } from '../tokens';

// Real artifact graph from Forgeplan dogfood
const NODES = [
  { id: 'EPIC-001', x: 300, y: 30, type: 'epic' },
  { id: 'PRD-001', x: 100, y: 130, type: 'prd' },
  { id: 'PRD-018', x: 280, y: 130, type: 'prd' },
  { id: 'PRD-024', x: 460, y: 130, type: 'prd' },
  { id: 'RFC-001', x: 60, y: 250, type: 'rfc' },
  { id: 'RFC-003', x: 200, y: 250, type: 'rfc' },
  { id: 'RFC-002', x: 360, y: 250, type: 'rfc' },
  { id: 'ADR-001', x: 60, y: 370, type: 'adr' },
  { id: 'ADR-003', x: 360, y: 370, type: 'adr' },
  { id: 'EVID-016', x: 520, y: 250, type: 'evidence' },
  { id: 'EVID-020', x: 60, y: 450, type: 'evidence' },
  { id: 'PROB-016', x: 560, y: 130, type: 'problem' },
];

const EDGES = [
  { from: 0, to: 1 }, { from: 0, to: 2 }, { from: 0, to: 3 }, { from: 0, to: 11 },
  { from: 1, to: 4 }, { from: 1, to: 5 },
  { from: 2, to: 6 },
  { from: 11, to: 9 },
  { from: 4, to: 7 },
  { from: 6, to: 8 },
  { from: 7, to: 10, dashed: true },
];

const NODE_COLORS: Record<string, string> = {
  epic: COLORS.ember,
  prd: COLORS.fg,
  rfc: COLORS.fg,
  adr: COLORS.fg,
  evidence: COLORS.green,
  problem: COLORS.ember,
};

export default function GraphSection() {
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

  return (
    <section id="graph" ref={sectionRef} className="relative w-full bg-forge-bg border-b border-forge-line" style={{ height: '250vh' }}>
      <div className="sticky top-[36px] overflow-hidden" style={{ height: 'calc(100vh - 36px)' }}>
        <div className="grid grid-cols-1 lg:grid-cols-[420px_1fr] h-full">

          {/* Left: Text */}
          <div className="flex flex-col justify-between p-6 lg:p-10 border-r border-forge-line h-full">
            <div>
              <p className="font-mono text-[11px] tracking-[3px] text-forge-ember mb-4" style={{ opacity: fade(0.02) }}>
                DEPENDENCY GRAPH
              </p>

              <h2 className="font-heading text-4xl lg:text-[52px] font-normal leading-[1.1]" style={{ opacity: fade(0.04), transform: `translateY(${(1 - fade(0.04)) * 10}px)` }}>
                Decisions<br />Are Connected
              </h2>

              <hr className="border-forge-line my-6" style={{ opacity: fade(0.12) }} />

              <div className="space-y-4">
                <div style={{ opacity: fade(0.18) }}>
                  <p className="font-mono text-xs text-forge-ember">forgeplan graph</p>
                  <p className="text-sm text-forge-dim mt-1">Visualize the full dependency tree. See how Epic → PRD → RFC → ADR connect.</p>
                </div>
                <div style={{ opacity: fade(0.28) }}>
                  <p className="font-mono text-xs text-forge-ember">forgeplan blocked</p>
                  <p className="text-sm text-forge-dim mt-1">What's waiting on what? Unblock by resolving dependencies first.</p>
                </div>
                <div style={{ opacity: fade(0.38) }}>
                  <p className="font-mono text-xs text-forge-ember">forgeplan blindspots</p>
                  <p className="text-sm text-forge-dim mt-1">Decisions without evidence. Orphan artifacts. Risks hiding in plain sight.</p>
                </div>
                <div style={{ opacity: fade(0.48) }}>
                  <p className="font-mono text-xs text-forge-ember">forgeplan drift</p>
                  <p className="text-sm text-forge-dim mt-1">Code changed but the decision didn't. Catch divergence early.</p>
                </div>
              </div>
            </div>

            <p className="font-mono text-[10px] tracking-[3px] text-forge-dim" style={{ opacity: fade(0.60) }}>
              TRACEABILITY
            </p>
          </div>

          {/* Right: DAG visualization */}
          <div className="relative flex items-center justify-center h-full">
            <div className="absolute inset-0 opacity-15 bg-dot-grid" aria-hidden="true" />

            {/* Legend — top right corner of section */}
            <div className="absolute top-4 right-4 z-20 border border-forge-line bg-forge-bg/90 px-3 py-2 flex items-center gap-4"
              style={{ opacity: fade(0.20) }}>
              <div className="flex items-center gap-2">
                <div className="w-5 h-0 border-t border-forge-fg" />
                <span className="font-mono text-[9px] text-forge-dim">parent</span>
              </div>
              <div className="flex items-center gap-2">
                <div className="w-5 h-0 border-t border-dashed border-forge-fg" />
                <span className="font-mono text-[9px] text-forge-dim">informs</span>
              </div>
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 rounded-full bg-forge-ember" />
                <span className="font-mono text-[9px] text-forge-dim">risk</span>
              </div>
            </div>
            <svg className="w-full h-full p-4 relative z-10" viewBox="0 0 620 490" fill="none" aria-hidden="true" preserveAspectRatio="xMidYMid meet">

              {/* Edges */}
              {EDGES.map((edge, ei) => {
                const from = NODES[edge.from];
                const to = NODES[edge.to];
                const edgeOpacity = fade(0.15 + ei * 0.04) * 0.35;
                return (
                  <line key={`e-${ei}`}
                    x1={from.x} y1={from.y + 20} x2={to.x} y2={to.y - 5}
                    stroke={COLORS.fg} strokeWidth="1"
                    strokeDasharray={edge.dashed ? '4 4' : undefined}
                    opacity={edgeOpacity}
                  />
                );
              })}

              {/* Nodes */}
              {NODES.map((node, ni) => {
                const nodeOpacity = fade(0.10 + ni * 0.04);
                const color = NODE_COLORS[node.type];
                const w = node.id.length * 9 + 20;
                const isBlind = node.type === 'problem';

                return (
                  <g key={`n-${ni}`} opacity={nodeOpacity}>
                    <rect x={node.x - w / 2} y={node.y - 12} width={w} height={28} rx="0"
                      fill={COLORS.surface} stroke={color} strokeWidth={isBlind ? 1.5 : 1} />
                    <text x={node.x} y={node.y + 5} textAnchor="middle"
                      fontFamily="Geist Mono, monospace" fontSize="11" fill={color}>
                      {node.id}
                    </text>
                    {isBlind && (
                      <circle cx={node.x + w / 2 + 8} cy={node.y} r="3" fill="#FF6B35" opacity="0.8" />
                    )}
                  </g>
                );
              })}

              {/* Legend moved to HTML overlay */}
            </svg>
          </div>
        </div>
      </div>
    </section>
  );
}

import { useState, useEffect, useRef } from 'react';
import { COLORS } from '../tokens';

const ARTIFACT_TYPES = [
  { id: 'PRD', name: 'PRD', desc: 'What you build and why', detail: 'Problem, goals, target users, functional requirements. Every FR: "[Actor] can [capability]" — no implementation leakage.', lifecycle: 'draft → active → superseded', color: 'fg' },
  { id: 'RFC', name: 'RFC', desc: 'How to build it', detail: 'Architecture proposal with implementation phases. Trade-offs documented. Adversarial review on Deep+.', lifecycle: 'draft → active → superseded', color: 'fg' },
  { id: 'ADR', name: 'ADR', desc: 'Why this approach', detail: 'Architecture Decision Record. Context, options considered, decision rationale. On Deep+: includes invariants and rollback plan.', lifecycle: 'draft → active → superseded', color: 'fg' },
  { id: 'Epic', name: 'Epic', desc: 'Group of work', detail: 'Aggregates PRDs, RFCs, ADRs. Tracks progress across children. Used for multi-week initiatives.', lifecycle: 'draft → active → deprecated', color: 'ember' },
  { id: 'Spec', name: 'Spec', desc: 'API contracts', detail: 'Exact interface definitions, data models, versioning. Delta-specs for changes: ADDED/MODIFIED/REMOVED.', lifecycle: 'draft → active → superseded', color: 'fg' },
  { id: 'Problem', name: 'Problem', desc: 'Signal with context', detail: 'Bug, risk, or observation. Anti-Goodhart indicators prevent gaming metrics. Links to evidence for investigation.', lifecycle: 'draft → active → deprecated', color: 'ember' },
  { id: 'Evidence', name: 'Evidence', desc: 'Test and prove', detail: 'Benchmark results, test output, audit findings. Verdict: supports/weakens/refutes. Congruence level CL0-CL3. Expires via valid_until.', lifecycle: 'draft → active', color: 'ember' },
  { id: 'Solution', name: 'Solution', desc: '2-3 variants compared', detail: 'Solution portfolio with weakest-link scoring. Each variant analyzed for strengths, risks, and evidence gaps.', lifecycle: 'draft → active → superseded', color: 'fg' },
  { id: 'Note', name: 'Note', desc: 'Quick micro-decision', detail: 'Lightweight. Auto-expires in 90 days. No validation gate needed for activation. Perfect for tactical decisions.', lifecycle: 'draft → active → stale', color: 'dim' },
  { id: 'Refresh', name: 'Refresh', desc: 'Re-evaluate stale', detail: 'Triggered when valid_until expires. Re-assess: is the decision still valid? Renew with new evidence or reopen as new artifact.', lifecycle: 'stale → refresh → renew/reopen', color: 'dim' },
];

export default function ArtifactsSection() {
  const [progress, setProgress] = useState(0);
  const [selected, setSelected] = useState(0);
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
  const art = ARTIFACT_TYPES[selected];

  return (
    <section id="artifacts" ref={sectionRef} className="relative w-full bg-forge-bg border-b border-forge-line" style={{ height: '300vh' }}>
      <div className="sticky top-[36px] overflow-hidden" style={{ height: 'calc(100vh - 36px)' }}>
        <div className="grid grid-cols-1 lg:grid-cols-[1fr_480px] h-full">

          {/* Left: Selected artifact preview */}
          <div className="flex flex-col justify-between p-6 lg:p-10 border-r border-forge-line h-full">
            <div>
              <p className="font-mono text-[11px] tracking-[3px] text-forge-ember mb-4" style={{ opacity: fade(0.02) }}>
                ARTIFACT PREVIEW
              </p>

              <h2 className="font-heading text-4xl lg:text-[52px] font-normal leading-none" style={{ opacity: fade(0.05) }}>
                {art.name}
              </h2>
              <p className="text-lg text-forge-dim mt-2" style={{ opacity: fade(0.08) }}>
                {art.desc}
              </p>

              <hr className="border-forge-line my-6" style={{ opacity: fade(0.10) }} />

              <p className="text-sm text-forge-fg leading-relaxed max-w-[480px]" style={{ opacity: fade(0.12) }}>
                {art.detail}
              </p>
            </div>

            {/* Lifecycle */}
            <div style={{ opacity: fade(0.15) }}>
              <p className="font-mono text-[10px] tracking-[2px] text-forge-dim mb-2">LIFECYCLE</p>
              <div className="flex items-center gap-2">
                {art.lifecycle.split(' → ').map((stage, si) => (
                  <span key={si} className="flex items-center gap-2">
                    {si > 0 && <span className="text-forge-line text-xs">→</span>}
                    <span className={`font-mono text-xs px-2 py-1 border ${
                      stage === 'active' ? 'border-forge-ember text-forge-ember' :
                      stage === 'draft' ? 'border-forge-line text-forge-dim' :
                      'border-forge-line text-forge-dim'
                    }`}>
                      {stage}
                    </span>
                  </span>
                ))}
              </div>

              <p className="font-mono text-[10px] tracking-[3px] text-forge-dim mt-6" style={{ opacity: fade(0.55) }}>
                SELECT AN ARTIFACT →
              </p>
              <p className="font-mono text-[10px] tracking-[3px] text-forge-dim mt-4" style={{ opacity: fade(0.65) }}>
                ARTIFACT MODEL
              </p>
            </div>
          </div>

          {/* Right: 2×5 grid of artifact cards */}
          <div className="flex flex-col p-4 lg:p-6 h-full">
            <p className="font-mono text-[11px] tracking-[3px] text-forge-ember mb-3" style={{ opacity: fade(0.02) }}>
              10 ARTIFACT TYPES
            </p>
            <p className="text-xs text-forge-dim mb-4" style={{ opacity: fade(0.05) }}>
              Every decision gets the right container. Click to explore.
            </p>

            <div className="grid grid-cols-2 gap-0 flex-1">
              {ARTIFACT_TYPES.map((type, i) => {
                const isSelected = i === selected;
                const cardOpacity = fade(0.08 + i * 0.03);
                const isEmber = type.color === 'ember';

                return (
                  <button
                    key={type.id}
                    onClick={() => setSelected(i)}
                    className={`text-left border border-forge-line p-3 lg:p-4 flex flex-col justify-between transition-all duration-200 cursor-pointer ${
                      isSelected ? 'bg-forge-surface' : 'hover:bg-forge-surface/50'
                    }`}
                    style={{
                      opacity: cardOpacity,
                      borderLeftColor: isSelected ? COLORS.ember : undefined,
                      borderLeftWidth: isSelected ? '3px' : undefined,
                    }}
                  >
                    <div>
                      <p className={`font-heading text-base lg:text-lg font-medium ${
                        isEmber ? 'text-forge-ember' : ''
                      }`}>
                        {type.name}
                      </p>
                      <p className="text-[11px] text-forge-dim mt-1 leading-relaxed">{type.desc}</p>
                    </div>
                    <div className="mt-2 flex items-center gap-1.5">
                      <div className={`w-1.5 h-1.5 rounded-full ${isEmber ? 'bg-forge-ember' : 'bg-forge-dim'}`} />
                      <span className="font-mono text-[10px] tracking-wider text-forge-dim">{type.id}</span>
                    </div>
                  </button>
                );
              })}
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

import { useState, useEffect, useRef } from 'react';

/**
 * PipelineSection — Depth routing + ADI reasoning.
 * No pin — scroll-driven via scroll position.
 */
export default function PipelineSection() {
  const [progress, setProgress] = useState(0);
  const sectionRef = useRef<HTMLElement>(null);

  useEffect(() => {
    const el = sectionRef.current;
    if (!el) return;

    function onScroll() {
      const rect = el!.getBoundingClientRect();
      const vh = window.innerHeight;
      const raw = 1 - rect.top / vh;
      setProgress(Math.max(0, Math.min(1, raw)));
    }

    window.addEventListener('scroll', onScroll, { passive: true });
    onScroll();
    return () => window.removeEventListener('scroll', onScroll);
  }, []);

  const fadeIn = (start: number) => Math.min(Math.max((progress - start) / 0.12, 0), 1);
  const slideUp = (start: number) => {
    const t = fadeIn(start);
    return { opacity: t, transform: `translateY(${(1 - t) * 20}px)` };
  };

  const depths = [
    { tag: 'TACTICAL', desc: 'Quick fix, 1 file. No artifact needed — just code and ship.', tagStyle: 'border border-forge-line text-forge-dim', start: 0.1 },
    { tag: 'STANDARD', desc: 'Feature 1-3 days, has choices. PRD → RFC. ADI recommended.', tagStyle: 'border border-forge-fg', start: 0.2 },
    { tag: 'DEEP', desc: 'New module, 1-2 weeks. PRD → Spec → RFC → ADR. ADI mandatory.', tagStyle: 'bg-forge-ember text-forge-bg', start: 0.3 },
    { tag: 'CRITICAL', desc: 'Cross-team, strategy. Epic → PRD[] → Spec[] → RFC[] → ADR[]. Full review.', tagStyle: 'bg-forge-ember text-forge-bg', start: 0.4 },
  ];

  const adiCards = [
    { title: 'Abduction', desc: 'Generate 3+ hypotheses. What could work?', ember: false, start: 0.55 },
    { title: 'Deduction', desc: 'Derive testable predictions for each.', ember: false, start: 0.65 },
    { title: 'Induction', desc: 'Check evidence. Score: supports / weakens / refutes.', ember: true, start: 0.75 },
  ];

  return (
    <section id="pipeline" ref={sectionRef} className="relative w-full min-h-screen bg-forge-bg border-b border-forge-line">
      <div className="grid grid-cols-1 lg:grid-cols-[500px_1fr] min-h-screen pt-[36px]">
        <div className="flex flex-col justify-between p-8 lg:p-12 border-r border-forge-line">
          <h2 className="font-heading text-5xl lg:text-[72px] font-normal leading-none">
            SHAPE<br />VALIDATE<br />REASON<br />BUILD<br />PROVE
          </h2>
          <div className="mt-auto space-y-4" style={{ opacity: fadeIn(0.85) }}>
            <p className="text-sm text-forge-dim leading-relaxed">
              Every decision has a lifecycle. Forgeplan enforces Shape → Validate → Reason → Code → Evidence → Activate — no shortcuts, no stubs, no blind spots.
            </p>
            <p className="font-mono text-sm font-medium text-forge-ember">
              forgeplan route "your task" →
            </p>
          </div>
        </div>

        <div className="flex flex-col justify-center p-8 lg:p-12 space-y-8">
          <p className="font-mono text-[11px] tracking-[3px] text-forge-ember">DEPTH ROUTING</p>
          <div className="space-y-0">
            {depths.map((d, i) => (
              <div key={i} className="flex items-center gap-4 border-b border-forge-line py-4" style={slideUp(d.start)}>
                <span className={`font-mono text-[10px] tracking-[2px] px-2 py-1 whitespace-nowrap ${d.tagStyle}`}>{d.tag}</span>
                <p className="text-sm">{d.desc}</p>
              </div>
            ))}
          </div>
          <p className="font-mono text-[11px] tracking-[3px] text-forge-ember mt-8" style={{ opacity: fadeIn(0.5) }}>ADI REASONING CYCLE</p>
          <div className="grid grid-cols-3 gap-4">
            {adiCards.map((card, i) => (
              <div key={i} className={`border p-4 space-y-2 ${card.ember ? 'border-forge-ember' : 'border-forge-line'}`} style={slideUp(card.start)}>
                <p className={`font-medium ${card.ember ? 'text-forge-ember' : ''}`}>{card.title}</p>
                <p className="text-xs text-forge-dim leading-relaxed">{card.desc}</p>
              </div>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}

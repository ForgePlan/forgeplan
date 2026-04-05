import { useState, useEffect, useRef } from 'react';

// Pipeline steps — vertical timeline stations
const STEPS = [
  { word: 'SHAPE', desc: 'Define what you\'re building and why. PRD, Brief, or nothing — route decides.', start: 0.04 },
  { word: 'VALIDATE', desc: 'Check against quality gates. 30+ rules per artifact type. No stubs allowed.', start: 0.14 },
  { word: 'REASON', desc: 'Generate 3+ hypotheses. Test predictions. Reach justified conclusions — not gut calls.', start: 0.24 },
  { word: 'BUILD', desc: 'Code with confidence. Every pub fn gets a test. Format, lint, audit before commit.', start: 0.34 },
  { word: 'PROVE', desc: 'Create evidence. Link it. Score it. R_eff > 0 or the work isn\'t done.', start: 0.44 },
];

const DEPTHS = [
  { tag: 'TACTICAL', desc: 'Quick fix, 1 file — just ship', tagStyle: 'border border-forge-line text-forge-dim', start: 0.20 },
  { tag: 'STANDARD', desc: 'Feature 1-3 days → PRD → RFC', tagStyle: 'border border-forge-fg', start: 0.28 },
  { tag: 'DEEP', desc: 'New module → PRD → Spec → RFC → ADR', tagStyle: 'bg-forge-ember text-forge-bg', start: 0.36 },
  { tag: 'CRITICAL', desc: 'Cross-team → Epic → PRD[] → Spec[] → RFC[] → ADR[]', tagStyle: 'bg-forge-ember text-forge-bg', start: 0.44 },
];

const ADI = [
  { title: 'Abduction', desc: 'Generate 3+ hypotheses', ember: false, start: 0.58 },
  { title: 'Deduction', desc: 'Derive testable predictions', ember: false, start: 0.65 },
  { title: 'Induction', desc: 'Check evidence, score results', ember: true, start: 0.72 },
];

export default function PipelineSection() {
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

  const fade = (start: number, dur = 0.08) => Math.min(Math.max((progress - start) / dur, 0), 1);
  const slideUp = (start: number) => {
    const t = fade(start);
    return { opacity: t, transform: `translateY(${(1 - t) * 15}px)` };
  };

  return (
    <section id="pipeline" ref={sectionRef} className="relative w-full bg-forge-bg border-b border-forge-line" style={{ height: '300vh' }}>
      <div className="sticky top-[36px] overflow-hidden" style={{ height: 'calc(100vh - 36px)' }}>
        <div className="grid grid-cols-1 lg:grid-cols-[1fr_1fr] h-full">

          {/* Left: Pipeline timeline */}
          <div className="relative flex flex-col justify-center p-8 lg:p-12 border-r border-forge-line">
            {/* Vertical timeline line */}
            <div className="absolute left-12 lg:left-16 top-[12%] bottom-[12%] w-px bg-forge-line"
              style={{ opacity: fade(0.02) }} />

            <div className="space-y-6 lg:space-y-8 pl-10 lg:pl-14">
              {STEPS.map((step, i) => {
                const stepFade = fade(step.start, 0.08);
                const isLast = i === STEPS.length - 1;
                return (
                  <div key={i} className="relative" style={{ opacity: stepFade, transform: `translateY(${(1 - stepFade) * 12}px)` }}>
                    {/* Timeline dot */}
                    <div className={`absolute -left-10 lg:-left-14 top-1 w-3 h-3 rounded-full border-2 ${
                      isLast ? 'border-forge-ember bg-forge-ember' : 'border-forge-line bg-forge-bg'
                    }`} style={{ opacity: stepFade }} />

                    {/* Step word */}
                    <h3 className={`font-heading text-3xl lg:text-[48px] font-normal leading-none ${
                      isLast ? 'text-forge-ember' : ''
                    }`}>
                      {step.word}
                    </h3>

                    {/* Step description */}
                    <p className="text-sm text-forge-dim leading-relaxed mt-1 max-w-[400px]"
                      style={{ opacity: fade(step.start + 0.04, 0.06) }}>
                      {step.desc}
                    </p>
                  </div>
                );
              })}
            </div>

            {/* Bottom tagline */}
            <div className="mt-auto pt-6" style={{ opacity: fade(0.82) }}>
              <p className="font-mono text-sm font-medium text-forge-ember">
                forgeplan route "your task" →
              </p>
            </div>
          </div>

          {/* Right: Depth routing + ADI */}
          <div className="flex flex-col justify-center p-8 lg:p-12 space-y-6">

            {/* Depth routing */}
            <p className="font-mono text-[11px] tracking-[3px] text-forge-ember" style={{ opacity: fade(0.15) }}>
              DEPTH ROUTING
            </p>
            <p className="text-sm text-forge-dim leading-relaxed" style={{ opacity: fade(0.17) }}>
              Describe your task — Forgeplan determines the right pipeline depth automatically.
            </p>

            <div className="space-y-0">
              {DEPTHS.map((d, i) => (
                <div key={i} className="flex items-center gap-4 border-b border-forge-line py-3" style={slideUp(d.start)}>
                  <span className={`font-mono text-[10px] tracking-[2px] px-2 py-1 whitespace-nowrap ${d.tagStyle}`}>
                    {d.tag}
                  </span>
                  <p className="text-sm text-forge-dim">{d.desc}</p>
                </div>
              ))}
            </div>

            {/* ADI */}
            <p className="font-mono text-[11px] tracking-[3px] text-forge-ember mt-6" style={{ opacity: fade(0.53) }}>
              ADI REASONING CYCLE
            </p>
            <p className="text-sm text-forge-dim leading-relaxed" style={{ opacity: fade(0.55) }}>
              Before building — reason through alternatives. Not gut feeling, structured thinking.
            </p>

            <div className="grid grid-cols-3 gap-3">
              {ADI.map((card, i) => (
                <div key={i}
                  className={`border p-3 space-y-1 ${card.ember ? 'border-forge-ember' : 'border-forge-line'}`}
                  style={slideUp(card.start)}>
                  <p className={`font-medium text-sm ${card.ember ? 'text-forge-ember' : ''}`}>{card.title}</p>
                  <p className="text-xs text-forge-dim leading-relaxed">{card.desc}</p>
                </div>
              ))}
            </div>

            {/* Bottom */}
            <p className="font-mono text-[10px] tracking-[3px] text-forge-dim mt-4" style={{ opacity: fade(0.85) }}>
              METHODOLOGY
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}

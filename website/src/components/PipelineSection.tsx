import { useState, useEffect, useRef } from 'react';
import { COLORS } from '../tokens';

// Pipeline steps — vertical timeline stations
const STEPS = [
  { word: 'SHAPE', desc: 'Define what you\'re building and why. PRD, Brief, or nothing — route decides.', start: 0.04 },
  { word: 'VALIDATE', desc: 'Check against quality gates. 30+ rules per artifact type. No stubs allowed.', start: 0.14 },
  { word: 'REASON', desc: 'Generate 3+ hypotheses. Test predictions. Reach justified conclusions — not gut calls.', start: 0.24 },
  { word: 'BUILD', desc: 'Code with confidence. Every pub fn gets a test. Format, lint, audit before commit.', start: 0.34 },
  { word: 'PROVE', desc: 'Create evidence. Link it. Score it. R_eff > 0 or the work isn\'t done.', start: 0.44 },
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

          {/* Left: Pipeline timeline — distributed across full height */}
          <div className="relative flex flex-col p-8 lg:py-10 lg:px-12 border-r border-forge-line h-full">
            <div className="flex flex-col justify-between h-full">
              {/* Steps — distributed across full height */}
              <div className="flex flex-col justify-between h-full">
                {STEPS.map((step, i) => {
                  const stepFade = fade(step.start, 0.08);
                  const isLast = i === STEPS.length - 1;
                  return (
                    <div key={i} className="flex flex-col flex-1" style={{ opacity: stepFade, transform: `translateY(${(1 - stepFade) * 10}px)` }}>
                      {/* Dot row: dot + dashed line + title */}
                      <div className="flex items-center flex-shrink-0">
                        <div className="flex-shrink-0 flex justify-center" style={{ width: '38px' }}>
                          <div className={`w-[12px] h-[12px] rounded-full border-2 ${
                            isLast ? 'border-forge-ember bg-forge-ember' : 'border-forge-fg bg-forge-bg'
                          }`} />
                        </div>
                        <div className="flex-shrink-0" style={{ width: '20px', borderTop: '1px dashed var(--forge-line)', opacity: 0.6 }} />
                        <h3 className={`pl-3 font-heading text-2xl lg:text-[42px] font-normal leading-none ${
                          isLast ? 'text-forge-ember' : ''
                        }`}>
                          {step.word}
                        </h3>
                      </div>
                      {/* Description + vertical line to next dot */}
                      <div className={`flex ${isLast ? '' : 'flex-1'}`}>
                        <div className="flex-shrink-0 flex justify-center" style={{ width: '38px' }}>
                          {!isLast && <div className="w-[2px] bg-forge-line h-full" style={{ opacity: 0.3 }} />}
                        </div>
                        <div className="flex-shrink-0" style={{ width: '20px' }} />
                        <p className="pl-3 text-xs lg:text-sm text-forge-dim leading-relaxed pt-1 max-w-[360px]"
                          style={{ opacity: fade(step.start + 0.04, 0.06) }}>
                          {step.desc}
                        </p>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>

            {/* Bottom tagline */}
            <div className="mt-auto pt-6" style={{ opacity: fade(0.82) }}>
              <p className="font-mono text-sm font-medium text-forge-ember">
                forgeplan route "your task" →
              </p>
            </div>
          </div>

          {/* Right: Git-like branching graph + ADI — full height, aligned with left */}
          <div className="relative flex flex-col justify-between p-6 lg:py-10 lg:px-10 h-full">
            <div className="absolute inset-0 opacity-15 bg-dot-grid" aria-hidden="true" />

            {/* Top block: DEPTH ROUTING label + tree */}
            <div className="relative z-10 flex-1 flex flex-col">
              <p className="font-mono text-[11px] tracking-[3px] text-forge-ember mb-2" style={{ opacity: fade(0.12) }}>
                DEPTH ROUTING
              </p>
              <svg className="w-full flex-1" viewBox="0 0 600 700" fill="none" aria-hidden="true" preserveAspectRatio="xMinYMin meet" style={{ marginLeft: '-30px' }}>
              {/* Main trunk — ends with ember dot at bottom */}
              <line x1="35" y1="25" x2="35" y2="670" stroke={COLORS.fg} strokeWidth="2" opacity={fade(0.14) * 0.4} />
              <circle cx="35" cy="670" r="7" fill={COLORS.ember} opacity={fade(0.50)} />

              {/* Route dot (top) */}
              <circle cx="35" cy="25" r="6" fill={COLORS.ember} opacity={fade(0.14)} />
              <text x="55" y="30" fontFamily="Geist Mono, monospace" fontSize="17" fill={COLORS.ember} opacity={fade(0.14)}>
                forgeplan route "your task"
              </text>

              {/* Tactical — no artifacts, just ship */}
              <g opacity={fade(0.20)}>
                <line x1="35" y1="80" x2="90" y2="80" stroke={COLORS.dim} strokeWidth="1.5" />
                <circle cx="90" cy="80" r="6" fill={COLORS.dim} />
                <text x="108" y="85" fontFamily="Geist Mono, monospace" fontSize="16" fill={COLORS.dim}>Tactical</text>
                <line x1="200" y1="80" x2="460" y2="80" stroke={COLORS.dim} strokeWidth="0.5" strokeDasharray="4 4" />
                <text x="470" y="85" fontFamily="Geist Mono, monospace" fontSize="15" fill={COLORS.dim}>→ Ship</text>
              </g>

              {/* Standard — 2 artifacts: PRD, RFC */}
              <g opacity={fade(0.28)}>
                <line x1="35" y1="170" x2="90" y2="170" stroke={COLORS.fg} strokeWidth="1.5" />
                <circle cx="90" cy="170" r="6" fill={COLORS.fg} />
                <text x="108" y="175" fontFamily="Geist Mono, monospace" fontSize="16" fill={COLORS.fg}>Standard</text>
                <line x1="90" y1="170" x2="90" y2="225" stroke={COLORS.fg} strokeWidth="1" />
                <line x1="90" y1="200" x2="145" y2="200" stroke={COLORS.fg} strokeWidth="1" />
                <text x="155" y="205" fontFamily="Geist Mono, monospace" fontSize="15" fill={COLORS.fg}>PRD</text>
                <line x1="90" y1="225" x2="145" y2="225" stroke={COLORS.fg} strokeWidth="1" />
                <text x="155" y="230" fontFamily="Geist Mono, monospace" fontSize="15" fill={COLORS.fg}>RFC</text>
                <line x1="195" y1="212" x2="460" y2="212" stroke={COLORS.fg} strokeWidth="0.5" strokeDasharray="4 4" />
                <text x="470" y="217" fontFamily="Geist Mono, monospace" fontSize="15" fill={COLORS.fg}>→ Evidence</text>
              </g>

              {/* Deep — 4 artifacts: PRD, Spec, RFC, ADR */}
              <g opacity={fade(0.36)}>
                <line x1="35" y1="310" x2="90" y2="310" stroke={COLORS.ember} strokeWidth="1.5" />
                <circle cx="90" cy="310" r="6" fill={COLORS.ember} />
                <text x="108" y="315" fontFamily="Geist Mono, monospace" fontSize="16" fill={COLORS.ember}>Deep</text>
                <line x1="90" y1="310" x2="90" y2="415" stroke={COLORS.ember} strokeWidth="1" />
                <line x1="90" y1="340" x2="145" y2="340" stroke={COLORS.ember} strokeWidth="1" />
                <text x="155" y="345" fontFamily="Geist Mono, monospace" fontSize="15" fill={COLORS.fg}>PRD</text>
                <line x1="90" y1="365" x2="145" y2="365" stroke={COLORS.ember} strokeWidth="1" />
                <text x="155" y="370" fontFamily="Geist Mono, monospace" fontSize="15" fill={COLORS.fg}>Spec</text>
                <line x1="90" y1="390" x2="145" y2="390" stroke={COLORS.ember} strokeWidth="1" />
                <text x="155" y="395" fontFamily="Geist Mono, monospace" fontSize="15" fill={COLORS.fg}>RFC</text>
                <line x1="90" y1="415" x2="145" y2="415" stroke={COLORS.ember} strokeWidth="1" />
                <text x="155" y="420" fontFamily="Geist Mono, monospace" fontSize="15" fill={COLORS.fg}>ADR</text>
                <line x1="195" y1="377" x2="460" y2="377" stroke={COLORS.ember} strokeWidth="0.5" strokeDasharray="4 4" />
                <text x="470" y="382" fontFamily="Geist Mono, monospace" fontSize="15" fill={COLORS.ember}>→ Full Review</text>
              </g>

              {/* Critical — 5 artifacts: Epic, PRD[], Spec[], RFC[], ADR[] */}
              <g opacity={fade(0.44)}>
                <line x1="35" y1="500" x2="90" y2="500" stroke={COLORS.ember} strokeWidth="2" />
                <circle cx="90" cy="500" r="6" fill={COLORS.ember} />
                <text x="108" y="505" fontFamily="Geist Mono, monospace" fontSize="16" fontWeight="600" fill={COLORS.ember}>Critical</text>
                <line x1="90" y1="500" x2="90" y2="630" stroke={COLORS.ember} strokeWidth="1" />
                <line x1="90" y1="530" x2="145" y2="530" stroke={COLORS.ember} strokeWidth="1" />
                <text x="155" y="535" fontFamily="Geist Mono, monospace" fontSize="15" fill={COLORS.fg}>Epic</text>
                <line x1="90" y1="555" x2="145" y2="555" stroke={COLORS.ember} strokeWidth="1" />
                <text x="155" y="560" fontFamily="Geist Mono, monospace" fontSize="15" fill={COLORS.fg}>PRD[]</text>
                <line x1="90" y1="580" x2="145" y2="580" stroke={COLORS.ember} strokeWidth="1" />
                <text x="155" y="585" fontFamily="Geist Mono, monospace" fontSize="15" fill={COLORS.fg}>Spec[]</text>
                <line x1="90" y1="605" x2="145" y2="605" stroke={COLORS.ember} strokeWidth="1" />
                <text x="155" y="610" fontFamily="Geist Mono, monospace" fontSize="15" fill={COLORS.fg}>RFC[]</text>
                <line x1="90" y1="630" x2="145" y2="630" stroke={COLORS.ember} strokeWidth="1" />
                <text x="155" y="635" fontFamily="Geist Mono, monospace" fontSize="15" fill={COLORS.fg}>ADR[]</text>
                <line x1="210" y1="580" x2="460" y2="580" stroke={COLORS.ember} strokeWidth="0.5" strokeDasharray="4 4" />
                <text x="470" y="585" fontFamily="Geist Mono, monospace" fontSize="15" fill={COLORS.ember}>→ Adversarial</text>
              </g>
            </svg>

            </div>

            {/* Bottom block: ADI */}
            <div className="relative z-10 flex-shrink-0 mt-4">
              <p className="font-mono text-[11px] tracking-[3px] text-forge-ember mb-3" style={{ opacity: fade(0.53) }}>
                ADI REASONING CYCLE
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
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

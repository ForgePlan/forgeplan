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

          {/* Left: Pipeline timeline */}
          <div className="relative flex flex-col justify-center p-8 lg:p-12 border-r border-forge-line">
            {/* Timeline: vertical line + dots ON line + dashed horizontal to text */}
            <div className="relative h-full flex flex-col justify-center">
              {/* Vertical line — center at 38px (dot center = paddingLeft 31 + radius 7 = 38) */}
              <div className="absolute top-[8%] bottom-[8%] w-[2px] bg-forge-line"
                style={{ left: '37px', opacity: fade(0.02) }} />

              <div className="space-y-5 lg:space-y-7">
                {STEPS.map((step, i) => {
                  const stepFade = fade(step.start, 0.08);
                  const isLast = i === STEPS.length - 1;
                  return (
                    <div key={i} className="flex items-start gap-0" style={{ opacity: stepFade, transform: `translateY(${(1 - stepFade) * 10}px)` }}>
                      {/* Dot centered on line: paddingLeft=31, dot=14px, center=31+7=38px = line position */}
                      <div className="flex-shrink-0 flex items-center" style={{ width: '45px', paddingLeft: '31px' }}>
                        <div className={`w-[14px] h-[14px] rounded-full border-2 ${
                          isLast ? 'border-forge-ember bg-forge-ember' : 'border-forge-fg bg-forge-bg'
                        }`} />
                      </div>

                      {/* Dashed horizontal line */}
                      <div className="flex-shrink-0 border-t border-dashed border-forge-line self-center"
                        style={{ width: '24px', marginTop: '1px', opacity: 0.5 }} />

                      {/* Text */}
                      <div className="pl-3">
                        <h3 className={`font-heading text-2xl lg:text-[42px] font-normal leading-none ${
                          isLast ? 'text-forge-ember' : ''
                        }`}>
                          {step.word}
                        </h3>
                        <p className="text-xs lg:text-sm text-forge-dim leading-relaxed mt-1 max-w-[360px]"
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

          {/* Right: Git-like branching graph + ADI */}
          <div className="relative flex flex-col justify-center p-6 lg:p-10">
            <div className="absolute inset-0 opacity-15 bg-dot-grid" aria-hidden="true" />

            <p className="font-mono text-[11px] tracking-[3px] text-forge-ember mb-4 relative z-10" style={{ opacity: fade(0.12) }}>
              DEPTH ROUTING
            </p>

            {/* Git branching SVG */}
            <svg className="w-full relative z-10" viewBox="0 0 600 500" fill="none" aria-hidden="true">
              {/* Main trunk */}
              <line x1="35" y1="25" x2="35" y2="480" stroke={COLORS.fg} strokeWidth="2" opacity={fade(0.14) * 0.4} />

              {/* Route dot */}
              <circle cx="35" cy="25" r="6" fill={COLORS.ember} opacity={fade(0.14)} />
              <text x="55" y="30" fontFamily="Geist Mono, monospace" fontSize="14" fill={COLORS.ember} opacity={fade(0.14)}>
                forgeplan route "your task"
              </text>

              {/* Tactical */}
              <g opacity={fade(0.20)}>
                <line x1="35" y1="85" x2="90" y2="85" stroke={COLORS.dim} strokeWidth="1.5" />
                <circle cx="90" cy="85" r="5" fill={COLORS.dim} />
                <text x="108" y="90" fontFamily="Geist Mono, monospace" fontSize="13" fill={COLORS.dim}>Tactical</text>
                <line x1="200" y1="85" x2="460" y2="85" stroke={COLORS.dim} strokeWidth="0.5" strokeDasharray="4 4" />
                <text x="470" y="90" fontFamily="Geist Mono, monospace" fontSize="12" fill={COLORS.dim}>→ Ship</text>
              </g>

              {/* Standard */}
              <g opacity={fade(0.28)}>
                <line x1="35" y1="155" x2="90" y2="155" stroke={COLORS.fg} strokeWidth="1.5" />
                <circle cx="90" cy="155" r="5" fill={COLORS.fg} />
                <text x="108" y="160" fontFamily="Geist Mono, monospace" fontSize="13" fill={COLORS.fg}>Standard</text>
                <line x1="90" y1="155" x2="90" y2="195" stroke={COLORS.fg} strokeWidth="1" />
                <line x1="90" y1="180" x2="145" y2="180" stroke={COLORS.fg} strokeWidth="1" />
                <text x="155" y="185" fontFamily="Geist Mono, monospace" fontSize="12" fill={COLORS.fg}>PRD</text>
                <line x1="195" y1="180" x2="230" y2="180" stroke={COLORS.fg} strokeWidth="0.8" />
                <text x="240" y="185" fontFamily="Geist Mono, monospace" fontSize="12" fill={COLORS.fg}>RFC</text>
                <line x1="275" y1="180" x2="460" y2="180" stroke={COLORS.fg} strokeWidth="0.5" strokeDasharray="4 4" />
                <text x="470" y="185" fontFamily="Geist Mono, monospace" fontSize="12" fill={COLORS.fg}>→ Evidence</text>
              </g>

              {/* Deep */}
              <g opacity={fade(0.36)}>
                <line x1="35" y1="245" x2="90" y2="245" stroke={COLORS.ember} strokeWidth="1.5" />
                <circle cx="90" cy="245" r="5" fill={COLORS.ember} />
                <text x="108" y="250" fontFamily="Geist Mono, monospace" fontSize="13" fill={COLORS.ember}>Deep</text>
                <line x1="90" y1="245" x2="90" y2="310" stroke={COLORS.ember} strokeWidth="1" />
                <line x1="90" y1="270" x2="145" y2="270" stroke={COLORS.ember} strokeWidth="1" />
                <text x="155" y="275" fontFamily="Geist Mono, monospace" fontSize="12" fill={COLORS.fg}>PRD → Spec</text>
                <line x1="90" y1="295" x2="145" y2="295" stroke={COLORS.ember} strokeWidth="1" />
                <text x="155" y="300" fontFamily="Geist Mono, monospace" fontSize="12" fill={COLORS.fg}>RFC → ADR</text>
                <line x1="280" y1="282" x2="460" y2="282" stroke={COLORS.ember} strokeWidth="0.5" strokeDasharray="4 4" />
                <text x="470" y="287" fontFamily="Geist Mono, monospace" fontSize="12" fill={COLORS.ember}>→ Full Review</text>
              </g>

              {/* Critical */}
              <g opacity={fade(0.44)}>
                <line x1="35" y1="360" x2="90" y2="360" stroke={COLORS.ember} strokeWidth="2" />
                <circle cx="90" cy="360" r="6" fill={COLORS.ember} />
                <text x="108" y="365" fontFamily="Geist Mono, monospace" fontSize="13" fontWeight="600" fill={COLORS.ember}>Critical</text>
                <line x1="90" y1="360" x2="90" y2="470" stroke={COLORS.ember} strokeWidth="1" />
                <line x1="90" y1="385" x2="145" y2="385" stroke={COLORS.ember} strokeWidth="1" />
                <text x="155" y="390" fontFamily="Geist Mono, monospace" fontSize="12" fill={COLORS.fg}>Epic → PRD[]</text>
                <line x1="90" y1="410" x2="145" y2="410" stroke={COLORS.ember} strokeWidth="1" />
                <text x="155" y="415" fontFamily="Geist Mono, monospace" fontSize="12" fill={COLORS.fg}>Spec[] → RFC[]</text>
                <line x1="90" y1="435" x2="145" y2="435" stroke={COLORS.ember} strokeWidth="1" />
                <text x="155" y="440" fontFamily="Geist Mono, monospace" fontSize="12" fill={COLORS.fg}>ADR[] → Review</text>
                <line x1="310" y1="410" x2="460" y2="410" stroke={COLORS.ember} strokeWidth="0.5" strokeDasharray="4 4" />
                <text x="470" y="415" fontFamily="Geist Mono, monospace" fontSize="12" fill={COLORS.ember}>→ Adversarial</text>
              </g>
            </svg>

            {/* ADI below graph */}
            <div className="relative z-10 mt-6">
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

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
            {/* Vertical timeline line — aligned with dots */}
            <div className="absolute left-[44px] lg:left-[52px] top-[10%] bottom-[10%] w-[1px] bg-forge-line"
              style={{ opacity: fade(0.02) }} />

            <div className="space-y-6 lg:space-y-8 pl-12 lg:pl-16">
              {STEPS.map((step, i) => {
                const stepFade = fade(step.start, 0.08);
                const isLast = i === STEPS.length - 1;
                return (
                  <div key={i} className="relative" style={{ opacity: stepFade, transform: `translateY(${(1 - stepFade) * 12}px)` }}>
                    {/* Timeline dot — centered on the line */}
                    <div className={`absolute -left-[26px] lg:-left-[34px] top-[6px] w-3 h-3 rounded-full border-2 ${
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

          {/* Right: Git-like branching graph + ADI */}
          <div className="relative flex flex-col justify-center p-6 lg:p-10">
            <div className="absolute inset-0 opacity-15 bg-dot-grid" aria-hidden="true" />

            <p className="font-mono text-[11px] tracking-[3px] text-forge-ember mb-4 relative z-10" style={{ opacity: fade(0.12) }}>
              DEPTH ROUTING
            </p>

            {/* Git branching SVG */}
            <svg className="w-full max-w-[550px] relative z-10" viewBox="0 0 550 420" fill="none" aria-hidden="true">
              {/* Main trunk line */}
              <line x1="30" y1="20" x2="30" y2="400" stroke={COLORS.fg} strokeWidth="1.5" opacity={fade(0.14) * 0.4} />

              {/* Route dot (top) */}
              <circle cx="30" cy="20" r="5" fill={COLORS.ember} opacity={fade(0.14)} />
              <text x="45" y="25" fontFamily="Geist Mono, monospace" fontSize="11" fill={COLORS.ember} opacity={fade(0.14)}>
                route "your task"
              </text>

              {/* Branch 1: Tactical */}
              <g opacity={fade(0.20)}>
                <line x1="30" y1="70" x2="80" y2="70" stroke={COLORS.dim} strokeWidth="1" />
                <circle cx="80" cy="70" r="4" fill={COLORS.dim} />
                <text x="95" y="74" fontFamily="Geist Mono, monospace" fontSize="10" fill={COLORS.dim}>Tactical</text>
                <line x1="170" y1="70" x2="420" y2="70" stroke={COLORS.dim} strokeWidth="0.5" strokeDasharray="4 4" />
                <text x="430" y="74" fontFamily="Geist Mono, monospace" fontSize="9" fill={COLORS.dim}>→ Ship</text>
              </g>

              {/* Branch 2: Standard */}
              <g opacity={fade(0.28)}>
                <line x1="30" y1="130" x2="80" y2="130" stroke={COLORS.fg} strokeWidth="1" />
                <circle cx="80" cy="130" r="4" fill={COLORS.fg} />
                <text x="95" y="134" fontFamily="Geist Mono, monospace" fontSize="10" fill={COLORS.fg}>Standard</text>
                {/* Sub-branches */}
                <line x1="80" y1="130" x2="80" y2="170" stroke={COLORS.fg} strokeWidth="0.8" />
                <line x1="80" y1="150" x2="130" y2="150" stroke={COLORS.fg} strokeWidth="0.8" />
                <text x="140" y="154" fontFamily="Geist Mono, monospace" fontSize="9" fill={COLORS.fg}>PRD</text>
                <line x1="175" y1="150" x2="210" y2="150" stroke={COLORS.fg} strokeWidth="0.5" />
                <text x="220" y="154" fontFamily="Geist Mono, monospace" fontSize="9" fill={COLORS.fg}>RFC</text>
                <line x1="250" y1="150" x2="420" y2="150" stroke={COLORS.fg} strokeWidth="0.5" strokeDasharray="4 4" />
                <text x="430" y="154" fontFamily="Geist Mono, monospace" fontSize="9" fill={COLORS.fg}>→ Evidence</text>
              </g>

              {/* Branch 3: Deep */}
              <g opacity={fade(0.36)}>
                <line x1="30" y1="210" x2="80" y2="210" stroke={COLORS.ember} strokeWidth="1" />
                <circle cx="80" cy="210" r="4" fill={COLORS.ember} />
                <text x="95" y="214" fontFamily="Geist Mono, monospace" fontSize="10" fill={COLORS.ember}>Deep</text>
                <line x1="80" y1="210" x2="80" y2="280" stroke={COLORS.ember} strokeWidth="0.8" />
                <line x1="80" y1="230" x2="130" y2="230" stroke={COLORS.ember} strokeWidth="0.8" />
                <text x="140" y="234" fontFamily="Geist Mono, monospace" fontSize="9" fill={COLORS.fg}>PRD → Spec</text>
                <line x1="80" y1="255" x2="130" y2="255" stroke={COLORS.ember} strokeWidth="0.8" />
                <text x="140" y="259" fontFamily="Geist Mono, monospace" fontSize="9" fill={COLORS.fg}>RFC → ADR</text>
                <line x1="250" y1="242" x2="420" y2="242" stroke={COLORS.ember} strokeWidth="0.5" strokeDasharray="4 4" />
                <text x="430" y="246" fontFamily="Geist Mono, monospace" fontSize="9" fill={COLORS.ember}>→ Full Review</text>
              </g>

              {/* Branch 4: Critical */}
              <g opacity={fade(0.44)}>
                <line x1="30" y1="320" x2="80" y2="320" stroke={COLORS.ember} strokeWidth="1.5" />
                <circle cx="80" cy="320" r="5" fill={COLORS.ember} />
                <text x="95" y="324" fontFamily="Geist Mono, monospace" fontSize="10" fontWeight="bold" fill={COLORS.ember}>Critical</text>
                <line x1="80" y1="320" x2="80" y2="395" stroke={COLORS.ember} strokeWidth="0.8" />
                <line x1="80" y1="340" x2="130" y2="340" stroke={COLORS.ember} strokeWidth="0.8" />
                <text x="140" y="344" fontFamily="Geist Mono, monospace" fontSize="9" fill={COLORS.fg}>Epic → PRD[]</text>
                <line x1="80" y1="360" x2="130" y2="360" stroke={COLORS.ember} strokeWidth="0.8" />
                <text x="140" y="364" fontFamily="Geist Mono, monospace" fontSize="9" fill={COLORS.fg}>Spec[] → RFC[]</text>
                <line x1="80" y1="380" x2="130" y2="380" stroke={COLORS.ember} strokeWidth="0.8" />
                <text x="140" y="384" fontFamily="Geist Mono, monospace" fontSize="9" fill={COLORS.fg}>ADR[] → Review</text>
                <line x1="280" y1="360" x2="420" y2="360" stroke={COLORS.ember} strokeWidth="0.5" strokeDasharray="4 4" />
                <text x="430" y="364" fontFamily="Geist Mono, monospace" fontSize="9" fill={COLORS.ember}>→ Adversarial</text>
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

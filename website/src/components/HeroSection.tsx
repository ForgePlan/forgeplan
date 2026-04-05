import { useState } from 'react';
import StickySection from './StickySection';
import CrystallizationAnimation from './CrystallizationAnimation';

// Paired narrative blocks — left + right appear simultaneously
// Each pair highlights a pain point connected to specific artifacts
const NARRATIVE_PAIRS = [
  {
    start: 0.02, end: 0.14,
    left:  { text: 'Decisions scattered across Slack, Docs, and memory', label: 'LOST CONTEXT', dotIdx: 0 },
    right: { text: 'PRDs that nobody reads after the first week', label: 'DEAD DOCS', dotIdx: 1 },
  },
  {
    start: 0.10, end: 0.22,
    left:  { text: 'Architecture choices made on gut feeling alone', label: 'NO EVIDENCE', dotIdx: 2 },
    right: { text: 'RFCs without follow-up or validation', label: 'NO PROOF', dotIdx: 6 },
  },
  {
    start: 0.18, end: 0.30,
    left:  { text: 'Stale specs that diverged from reality months ago', label: 'DECAY', dotIdx: 4 },
    right: { text: '"Why did we decide this?" — no one remembers', label: 'LOST WHY', dotIdx: 5 },
  },
  {
    start: 0.26, end: 0.40,
    left:  { text: 'What if every decision had structure and a reliability score?', label: 'THE QUESTION', dotIdx: 3 },
    right: { text: 'What if evidence decayed visibly — not silently?', label: 'THE SHIFT', dotIdx: 7 },
  },
];

function pairOpacity(progress: number, start: number, end: number): number {
  const fadeIn = Math.min(Math.max((progress - start) / 0.03, 0), 1);
  const fadeOut = Math.min(Math.max((end - progress) / 0.03, 0), 1);
  return fadeIn * fadeOut;
}

export default function HeroSection() {
  const [progress, setProgress] = useState(0);

  return (
    <StickySection id="hero" scrollMultiplier={8} onProgress={setProgress} className="h-screen flex flex-col border-b border-forge-line">
      {/* Canvas */}
      <div className="relative w-full flex-1 overflow-hidden pt-[88px]">
        <div className="absolute inset-0 opacity-25 bg-dot-grid" aria-hidden="true" />
        <CrystallizationAnimation progress={progress} />

        {/* Narrative pairs — left + right simultaneously */}
        {NARRATIVE_PAIRS.map((pair, pi) => {
          const opacity = pairOpacity(progress, pair.start, pair.end);
          if (opacity <= 0) return null;

          // Vertical position: stagger pairs
          const topPercent = 18 + pi * 18;

          return (
            <div key={pi} className="absolute inset-x-0 pointer-events-none" style={{ top: `${topPercent}%`, opacity }}>
              <div className="flex justify-between items-start px-6 lg:px-10">
                {/* Left block + dashed line → center */}
                <div className="flex items-center" style={{ transform: `translateY(${(1 - opacity) * 10}px)` }}>
                  <div
                    className="border border-forge-line bg-forge-bg/90 backdrop-blur-sm px-4 py-3 max-w-[320px]"
                    style={{ borderLeft: '2px solid var(--color-forge-ember)' }}
                  >
                    <p className="font-body text-xs md:text-sm leading-relaxed text-forge-fg">{pair.left.text}</p>
                    <div className="mt-1.5 flex items-center gap-1.5">
                      <div className="w-1.5 h-1.5 rounded-full bg-forge-ember" />
                      <span className="font-mono text-[8px] tracking-[2px] text-forge-dim">{pair.left.label}</span>
                    </div>
                  </div>
                  {/* Dashed connector line from card → toward center */}
                  <div className="h-[1px] w-12 lg:w-20 border-t border-dashed border-forge-ember/40 flex-shrink-0" />
                  <div className="w-1.5 h-1.5 rounded-full bg-forge-ember/40 flex-shrink-0" />
                </div>

                {/* Right block + dashed line ← center */}
                <div className="flex items-center" style={{ transform: `translateY(${(1 - opacity) * 10}px)` }}>
                  <div className="w-1.5 h-1.5 rounded-full bg-forge-ember/40 flex-shrink-0" />
                  <div className="h-[1px] w-12 lg:w-20 border-t border-dashed border-forge-ember/40 flex-shrink-0" />
                  <div
                    className="border border-forge-line bg-forge-bg/90 backdrop-blur-sm px-4 py-3 max-w-[320px] text-right"
                    style={{ borderRight: '2px solid var(--color-forge-ember)' }}
                  >
                    <p className="font-body text-xs md:text-sm leading-relaxed text-forge-fg">{pair.right.text}</p>
                    <div className="mt-1.5 flex items-center justify-end gap-1.5">
                      <span className="font-mono text-[8px] tracking-[2px] text-forge-dim">{pair.right.label}</span>
                      <div className="w-1.5 h-1.5 rounded-full bg-forge-ember" />
                    </div>
                  </div>
                </div>
              </div>
            </div>
          );
        })}
      </div>

      {/* Bottom text block */}
      <div className="border-t border-forge-line grid grid-cols-1 md:grid-cols-[1fr_500px] h-[220px] bg-forge-bg relative z-10 shrink-0">
        <div className="flex items-end p-6 md:p-8 border-r border-forge-line">
          <h1 className="font-heading text-4xl md:text-[58px] font-normal leading-[1.15]">
            From Raw Idea<br />
            To Proven Decision <span className="text-forge-ember" aria-hidden="true">&gt;&gt;&gt;</span>
          </h1>
        </div>
        <div className="flex flex-col justify-between p-6 md:p-8">
          <p className="text-sm leading-relaxed text-forge-fg">
            Forgeplan turns unstructured thinking into structured artifacts you can trust, test, and ship — with quality scoring, evidence tracking, and semantic search built in.
          </p>
          <p className="text-sm font-bold text-forge-ember mt-4">
            Shape. Validate. Reason. Build. Prove.
          </p>
          <a href="/getting-started/installation" className="text-lg md:text-xl mt-4 text-forge-fg hover:text-forge-ember transition-colors">
            Get started <span aria-hidden="true">→</span>
          </a>
        </div>
      </div>
    </StickySection>
  );
}

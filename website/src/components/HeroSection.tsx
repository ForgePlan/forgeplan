import { useState } from 'react';
import StickySection from './StickySection';
import CrystallizationAnimation from './CrystallizationAnimation';

// Narrative blocks — problem → insight → hope
// Appear over chaos animation, fade out before crystallization
const NARRATIVE = [
  {
    text: 'Your decisions are scattered across Slack threads, Google Docs, and someone\'s memory',
    start: 0.02,
    end: 0.18,
    position: 'left' as const,
  },
  {
    text: 'PRDs nobody maintains. RFCs without evidence. Architecture choices made on gut feeling',
    start: 0.10,
    end: 0.26,
    position: 'right' as const,
  },
  {
    text: 'What if every decision had structure, proof, and a reliability score?',
    start: 0.20,
    end: 0.35,
    position: 'center' as const,
  },
];

function narrativeOpacity(progress: number, start: number, end: number): number {
  const fadeIn = Math.min(Math.max((progress - start) / 0.04, 0), 1);
  const fadeOut = Math.min(Math.max((end - progress) / 0.04, 0), 1);
  return fadeIn * fadeOut;
}

export default function HeroSection() {
  const [progress, setProgress] = useState(0);

  return (
    <StickySection id="hero" scrollMultiplier={3} onProgress={setProgress} className="h-screen flex flex-col border-b border-forge-line">
      {/* Canvas area */}
      <div className="relative w-full flex-1 overflow-hidden pt-[88px]">
        <div className="absolute inset-0 opacity-25 bg-dot-grid" aria-hidden="true" />
        <CrystallizationAnimation progress={progress} />

        {/* Narrative text blocks — appear over chaos, fade before crystallization */}
        <div className="absolute inset-0 pointer-events-none flex items-center justify-center" style={{ paddingTop: '88px' }}>
          {NARRATIVE.map((block, i) => {
            const opacity = narrativeOpacity(progress, block.start, block.end);
            if (opacity <= 0) return null;

            const positionClasses = {
              left: 'self-center mr-auto ml-[8%]',
              right: 'self-center ml-auto mr-[8%]',
              center: 'self-center mx-auto',
            }[block.position];

            return (
              <div
                key={i}
                className={`absolute max-w-[420px] ${positionClasses}`}
                style={{
                  opacity,
                  transform: `translateY(${(1 - opacity) * 15}px)`,
                  transition: 'none',
                }}
              >
                <div className="border border-forge-line bg-forge-bg/90 backdrop-blur-sm px-6 py-5">
                  <p className="font-body text-base md:text-lg leading-relaxed text-forge-fg">
                    {block.text}
                  </p>
                  <div className="mt-3 flex items-center gap-2">
                    <div className="w-2 h-2 rounded-full bg-forge-ember" />
                    <span className="font-mono text-[10px] tracking-[2px] text-forge-dim uppercase">
                      {i === 0 ? 'the problem' : i === 1 ? 'the cost' : 'the question'}
                    </span>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
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

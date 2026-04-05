import { useState } from 'react';
import StickySection from './StickySection';
import CrystallizationAnimation from './CrystallizationAnimation';

export default function HeroSection() {
  const [progress, setProgress] = useState(0);

  return (
    <StickySection id="hero" scrollMultiplier={3} onProgress={setProgress} className="h-screen flex flex-col">
      {/* Canvas area — fills space between header and bottom block */}
      <div className="relative w-full flex-1 overflow-hidden pt-[88px]">
        <div className="absolute inset-0 opacity-25 bg-dot-grid" aria-hidden="true" />
        <CrystallizationAnimation progress={progress} />
      </div>

      {/* Bottom text block — fixed 220px */}
      <div className="border-t border-forge-line grid grid-cols-1 md:grid-cols-[1fr_480px] h-[220px] bg-forge-bg relative z-10 shrink-0">
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

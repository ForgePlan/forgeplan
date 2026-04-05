import { useState, useEffect, useRef } from 'react';

const AI_COMMANDS = [
  { cmd: 'forgeplan route "Payment system auth"', output: 'Depth: Deep\nPipeline: PRD → Spec → RFC → ADR\nConfidence: 92%', start: 0.08 },
  { cmd: 'forgeplan reason PRD-026', output: 'Hypothesis 1: JWT tokens with refresh\nHypothesis 2: Session-based with Redis\nHypothesis 3: OAuth2 delegation\n→ Testing predictions...', start: 0.25 },
  { cmd: 'forgeplan decompose PRD-026', output: 'RFC-010: Auth middleware design\nRFC-011: Token storage strategy\nRFC-012: Rate limiting approach', start: 0.42 },
];

const MCP_CATEGORIES = [
  { name: 'Create', tools: ['artifact_create', 'artifact_from_description', 'evidence_create'], start: 0.55 },
  { name: 'Analyze', tools: ['validate', 'score', 'health', 'blindspots'], start: 0.62 },
  { name: 'Navigate', tools: ['search', 'graph', 'blocked', 'context'], start: 0.69 },
  { name: 'Decide', tools: ['route', 'reason', 'decompose', 'capture'], start: 0.76 },
];

export default function AISection() {
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

  return (
    <section id="ai" ref={sectionRef} className="relative w-full bg-forge-bg border-b border-forge-line" style={{ height: '250vh' }}>
      <div className="sticky top-[36px] overflow-hidden" style={{ height: 'calc(100vh - 36px)' }}>
        <div className="grid grid-cols-1 lg:grid-cols-[1fr_1fr] h-full">

          {/* Left: Terminal demo */}
          <div className="flex flex-col justify-between p-6 lg:p-10 border-r border-forge-line h-full">
            <div>
              <p className="font-mono text-[11px] tracking-[3px] text-forge-ember mb-4" style={{ opacity: fade(0.02) }}>
                AI-NATIVE
              </p>
              <h2 className="font-heading text-4xl lg:text-[48px] font-normal leading-[1.1] mb-2"
                style={{ opacity: fade(0.04), transform: `translateY(${(1 - fade(0.04)) * 10}px)` }}>
                AI Amplifies,<br />Doesn't Replace
              </h2>
              <p className="text-sm text-forge-dim max-w-[400px]" style={{ opacity: fade(0.08) }}>
                28 MCP tools let Claude, GPT, or any AI agent create, validate, and manage artifacts.
                But decisions are yours. Structure + AI = force multiplier.
              </p>
            </div>

            {/* Terminal */}
            <div className="border border-forge-line bg-forge-surface mt-4 flex-1 max-h-[400px] overflow-hidden" style={{ opacity: fade(0.12) }}>
              <div className="flex items-center gap-2 px-4 py-2 border-b border-forge-line">
                <div className="w-2.5 h-2.5 rounded-full bg-[#FF5F57]" />
                <div className="w-2.5 h-2.5 rounded-full bg-[#FEBC2E]" />
                <div className="w-2.5 h-2.5 rounded-full bg-[#28C840]" />
                <span className="font-mono text-[10px] text-forge-dim ml-2">forgeplan + AI</span>
              </div>
              <div className="p-4 space-y-4 font-mono text-xs">
                {AI_COMMANDS.map((cmd, ci) => {
                  const cmdOpacity = fade(cmd.start, 0.10);
                  if (cmdOpacity <= 0) return null;
                  return (
                    <div key={ci} style={{ opacity: cmdOpacity }}>
                      <p className="text-forge-ember">$ {cmd.cmd}</p>
                      {cmd.output.split('\n').map((line, li) => (
                        <p key={li} className="text-forge-dim ml-2">{line}</p>
                      ))}
                    </div>
                  );
                })}
              </div>
            </div>

            <p className="font-mono text-[10px] tracking-[3px] text-forge-dim mt-4" style={{ opacity: fade(0.88) }}>
              MCP INTEGRATION
            </p>
          </div>

          {/* Right: MCP tools grid */}
          <div className="flex flex-col justify-between p-6 lg:p-10 h-full">
            <div>
              <p className="font-mono text-[11px] tracking-[3px] text-forge-ember mb-4" style={{ opacity: fade(0.50) }}>
                28 MCP TOOLS
              </p>
              <p className="text-sm text-forge-dim mb-6" style={{ opacity: fade(0.52) }}>
                Model Context Protocol — any AI assistant can manage your artifacts.
              </p>

              <div className="space-y-4">
                {MCP_CATEGORIES.map((cat, ci) => (
                  <div key={ci} style={{ opacity: fade(cat.start), transform: `translateY(${(1 - fade(cat.start)) * 12}px)` }}>
                    <p className="font-heading text-lg font-medium mb-2">{cat.name}</p>
                    <div className="flex flex-wrap gap-2">
                      {cat.tools.map((tool, ti) => (
                        <span key={ti} className="font-mono text-[11px] px-2 py-1 border border-forge-line text-forge-dim">
                          {tool}
                        </span>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            </div>

            {/* Bottom statement */}
            <div className="border border-forge-ember p-4 mt-4" style={{ opacity: fade(0.76) }}>
              <p className="font-heading text-lg text-forge-ember">Structure + AI = Force Multiplier</p>
              <p className="text-xs text-forge-dim mt-1">
                AI generates hypotheses. Forgeplan validates them. Evidence proves them. You decide.
              </p>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}

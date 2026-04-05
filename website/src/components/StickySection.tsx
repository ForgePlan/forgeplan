import { useEffect, useRef, type ReactNode } from 'react';
import { gsap } from 'gsap';
import { ScrollTrigger } from 'gsap/ScrollTrigger';

gsap.registerPlugin(ScrollTrigger);

interface StickySectionProps {
  id: string;
  children: ReactNode;
  /** Scroll distance as multiplier of viewport height. 2.0 = 2x screen of scrolling */
  scrollMultiplier?: number;
  /** Called with progress 0..1 */
  onProgress?: (progress: number) => void;
  className?: string;
}

export default function StickySection({
  id,
  children,
  scrollMultiplier = 1.5,
  onProgress,
  className = '',
}: StickySectionProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const onProgressRef = useRef(onProgress);
  onProgressRef.current = onProgress;

  useEffect(() => {
    if (!containerRef.current) return;
    if (scrollMultiplier <= 0) return;

    // Explicit pixel distance — no ambiguity
    const scrollDistance = window.innerHeight * scrollMultiplier;

    const trigger = ScrollTrigger.create({
      trigger: containerRef.current,
      start: 'top top',
      end: `+=${scrollDistance}px`,
      pin: true,
      pinSpacing: true,
      anticipatePin: 1,
      onUpdate: (self) => {
        onProgressRef.current?.(self.progress);
      },
    });

    return () => { trigger.kill(); };
  }, [scrollMultiplier]);

  return (
    <section
      id={id}
      ref={containerRef}
      className={`relative w-full overflow-hidden h-screen bg-forge-bg ${className}`}
      style={{ zIndex: 10 }}
    >
      {children}
    </section>
  );
}

import { useEffect, useRef, type ReactNode } from 'react';
import { gsap } from 'gsap';
import { ScrollTrigger } from 'gsap/ScrollTrigger';

gsap.registerPlugin(ScrollTrigger);

interface StickySectionProps {
  id: string;
  children: ReactNode;
  /** How much scroll distance this section consumes. "150%" = 1.5x viewport height */
  scrollLength?: string;
  /** Called with progress 0..1 as user scrolls through pinned section */
  onProgress?: (progress: number) => void;
  /** Additional CSS classes */
  className?: string;
}

/**
 * StickySection — pins its content to viewport while user scrolls.
 * Exit only after scrollLength is consumed (progress reaches 1.0).
 *
 * SOLID: Single Responsibility — only handles pin/unpin + progress tracking.
 * Each section's animation logic lives in its own component via onProgress callback.
 */
export default function StickySection({
  id,
  children,
  scrollLength = '100%',
  onProgress,
  className = '',
}: StickySectionProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!containerRef.current) return;

    // Only create ScrollTrigger if scrollLength > 0
    if (scrollLength === '0%' || scrollLength === '0') {
      return;
    }

    const trigger = ScrollTrigger.create({
      trigger: containerRef.current,
      start: 'top top',
      end: `+=${scrollLength}`,
      pin: true,
      scrub: true,
      onUpdate: (self) => {
        onProgress?.(self.progress);
      },
    });

    return () => {
      trigger.kill();
    };
  }, [scrollLength, onProgress]);

  return (
    <section
      id={id}
      ref={containerRef}
      className={`relative w-full overflow-hidden ${className}`}
    >
      {children}
    </section>
  );
}

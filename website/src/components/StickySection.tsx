import { useEffect, useRef, type ReactNode } from 'react';
import { gsap } from 'gsap';
import { ScrollTrigger } from 'gsap/ScrollTrigger';

gsap.registerPlugin(ScrollTrigger);

interface StickySectionProps {
  id: string;
  children: ReactNode;
  scrollLength?: string;
  onProgress?: (progress: number) => void;
  className?: string;
}

export default function StickySection({
  id,
  children,
  scrollLength = '100%',
  onProgress,
  className = '',
}: StickySectionProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const onProgressRef = useRef(onProgress);
  onProgressRef.current = onProgress;

  useEffect(() => {
    if (!containerRef.current) return;
    if (scrollLength === '0%' || scrollLength === '0') return;

    const trigger = ScrollTrigger.create({
      trigger: containerRef.current,
      start: 'top top',
      end: `+=${scrollLength}`,
      pin: true,
      scrub: true,
      onUpdate: (self) => {
        onProgressRef.current?.(self.progress);
      },
    });

    return () => { trigger.kill(); };
  }, [scrollLength]); // onProgress via ref, no re-registration

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

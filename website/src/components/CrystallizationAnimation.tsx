import { useEffect, useRef } from 'react';
import { gsap } from 'gsap';
import { ScrollTrigger } from 'gsap/ScrollTrigger';

gsap.registerPlugin(ScrollTrigger);

const COLORS = {
  bg: '#0D0D0D',
  fg: '#E8E8E8',
  line: '#3A3A3A',
  dim: '#949494',
  ember: '#FF6B35',
  surface: '#161616',
};

const ARTIFACTS = [
  { id: 'PRD', label: 'PRD', color: COLORS.fg },
  { id: 'RFC', label: 'RFC', color: COLORS.fg },
  { id: 'ADR', label: 'ADR', color: COLORS.fg },
  { id: 'Epic', label: 'Epic', color: COLORS.ember },
  { id: 'Spec', label: 'Spec', color: COLORS.fg },
  { id: 'Problem', label: 'Problem', color: COLORS.ember },
  { id: 'Evidence', label: 'Evidence', color: COLORS.ember },
  { id: 'Solution', label: 'Solution', color: COLORS.fg },
  { id: 'Note', label: 'Note', color: COLORS.dim },
  { id: 'Refresh', label: 'Refresh', color: COLORS.dim },
];

function hexVertex(cx: number, cy: number, r: number, i: number): [number, number] {
  const angle = (Math.PI / 3) * i - Math.PI / 2;
  return [cx + r * Math.cos(angle), cy + r * Math.sin(angle)];
}

// --- Geometry: minimum distance between two line segments ---
function closestPointOnSegment(
  px: number, py: number,
  ax: number, ay: number, bx: number, by: number
): [number, number] {
  const dx = bx - ax, dy = by - ay;
  const lenSq = dx * dx + dy * dy;
  if (lenSq === 0) return [ax, ay];
  let t = ((px - ax) * dx + (py - ay) * dy) / lenSq;
  t = Math.max(0, Math.min(1, t));
  return [ax + t * dx, ay + t * dy];
}

function segmentSegmentDist(
  a1x: number, a1y: number, a2x: number, a2y: number,
  b1x: number, b1y: number, b2x: number, b2y: number
): { dist: number; cpA: [number, number]; cpB: [number, number] } {
  // Check 4 point-to-segment combos, take minimum
  const candidates: { dist: number; cpA: [number, number]; cpB: [number, number] }[] = [];

  // a1 to segB
  const pB1 = closestPointOnSegment(a1x, a1y, b1x, b1y, b2x, b2y);
  candidates.push({ dist: Math.hypot(a1x - pB1[0], a1y - pB1[1]), cpA: [a1x, a1y], cpB: pB1 });

  // a2 to segB
  const pB2 = closestPointOnSegment(a2x, a2y, b1x, b1y, b2x, b2y);
  candidates.push({ dist: Math.hypot(a2x - pB2[0], a2y - pB2[1]), cpA: [a2x, a2y], cpB: pB2 });

  // b1 to segA
  const pA1 = closestPointOnSegment(b1x, b1y, a1x, a1y, a2x, a2y);
  candidates.push({ dist: Math.hypot(b1x - pA1[0], b1y - pA1[1]), cpA: pA1, cpB: [b1x, b1y] });

  // b2 to segA
  const pA2 = closestPointOnSegment(b2x, b2y, a1x, a1y, a2x, a2y);
  candidates.push({ dist: Math.hypot(b2x - pA2[0], b2y - pA2[1]), cpA: pA2, cpB: [b2x, b2y] });

  candidates.sort((a, b) => a.dist - b.dist);
  return candidates[0];
}

// --- Physics ---
const MAX_SPEED = 0.35;
const MAX_ANGULAR = 0.002;
const COLLISION_DIST = 30; // px — repulsion when segments closer than this
const WALL_PAD = 30;

interface LineState {
  cx: number; cy: number;
  vx: number; vy: number;
  angle: number;
  va: number;
  len: number;
  dotProgress: number;
  dotDir: number;
  dotSpeed: number;
  speed: number;
}

function createLineState(w: number, h: number): LineState {
  const speed = 0.15 + Math.random() * 0.7;
  return {
    cx: 80 + Math.random() * (w - 160),
    cy: 80 + Math.random() * (h - 160),
    vx: (Math.random() - 0.5) * MAX_SPEED * 2 * speed,
    vy: (Math.random() - 0.5) * MAX_SPEED * 2 * speed,
    angle: Math.random() * Math.PI * 2,
    va: (Math.random() - 0.5) * MAX_ANGULAR * 2 * speed,
    len: 100 + Math.random() * 220,
    dotProgress: Math.random(),
    dotDir: Math.random() > 0.5 ? 1 : -1,
    dotSpeed: 0.0004 + Math.random() * 0.0012, // 2x slower
    speed,
  };
}

// Get line endpoints from state
function endpoints(st: LineState): [number, number, number, number] {
  const hlen = st.len / 2;
  const cos = Math.cos(st.angle), sin = Math.sin(st.angle);
  return [st.cx - cos * hlen, st.cy - sin * hlen, st.cx + cos * hlen, st.cy + sin * hlen];
}

function clamp(v: number, max: number): number {
  return Math.max(-max, Math.min(max, v));
}

export default function CrystallizationAnimation() {
  const containerRef = useRef<HTMLDivElement>(null);
  const svgRef = useRef<SVGSVGElement>(null);

  useEffect(() => {
    if (!svgRef.current || !containerRef.current) return;

    const W = 1440, H = 800;
    const CX = W / 2, CY = H / 2;
    const hexRadii = [200, 160, 120, 70];
    const svg = svgRef.current;
    let animating = true;
    let scrollProgress = 0;

    // States
    const totalLines = ARTIFACTS.length + 6;
    const states: LineState[] = [];
    for (let i = 0; i < totalLines; i++) states.push(createLineState(W, H));

    // SVG: Lines
    const lines: SVGLineElement[] = [];
    states.forEach((_, idx) => {
      const line = document.createElementNS('http://www.w3.org/2000/svg', 'line');
      line.setAttribute('stroke', idx < ARTIFACTS.length ? COLORS.fg : COLORS.line);
      line.setAttribute('stroke-width', idx < ARTIFACTS.length ? '0.8' : '0.4');
      line.setAttribute('opacity', idx < ARTIFACTS.length ? '0.3' : '0.15');
      svg.appendChild(line);
      lines.push(line);
    });

    // SVG: Dots
    const dots: SVGCircleElement[] = [];
    ARTIFACTS.forEach((art) => {
      const dot = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
      dot.setAttribute('r', String(3 + Math.random() * 5));
      dot.setAttribute('fill', art.color);
      dot.setAttribute('opacity', '0.7');
      svg.appendChild(dot);
      dots.push(dot);
    });

    // SVG: Labels (with animatable inner elements)
    const labels: SVGGElement[] = [];
    const labelPointers: SVGLineElement[] = [];
    const labelRects: SVGRectElement[] = [];
    const labelTexts: SVGTextElement[] = [];
    const DEFAULT_OX = 15, DEFAULT_OY = -12;

    ARTIFACTS.forEach((art) => {
      const g = document.createElementNS('http://www.w3.org/2000/svg', 'g');

      const pointer = document.createElementNS('http://www.w3.org/2000/svg', 'line');
      pointer.setAttribute('x1', '0'); pointer.setAttribute('y1', '0');
      pointer.setAttribute('x2', String(DEFAULT_OX)); pointer.setAttribute('y2', String(DEFAULT_OY));
      pointer.setAttribute('stroke', art.color);
      pointer.setAttribute('stroke-width', '0.5');
      pointer.setAttribute('opacity', '0.5');
      g.appendChild(pointer);
      labelPointers.push(pointer);

      const textLen = art.label.length * 7.5 + 12;
      const rect = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
      rect.setAttribute('x', String(DEFAULT_OX)); rect.setAttribute('y', String(DEFAULT_OY - 10));
      rect.setAttribute('width', String(textLen)); rect.setAttribute('height', '20');
      rect.setAttribute('fill', COLORS.surface);
      rect.setAttribute('stroke', art.color === COLORS.ember ? COLORS.ember : COLORS.line);
      rect.setAttribute('stroke-width', '0.5');
      g.appendChild(rect);
      labelRects.push(rect);

      const text = document.createElementNS('http://www.w3.org/2000/svg', 'text');
      text.setAttribute('x', String(DEFAULT_OX + 6)); text.setAttribute('y', String(DEFAULT_OY + 4));
      text.setAttribute('font-family', 'Geist Mono, monospace');
      text.setAttribute('font-size', '10');
      text.setAttribute('fill', art.color);
      text.textContent = art.label;
      g.appendChild(text);
      labelTexts.push(text);

      svg.appendChild(g);
      labels.push(g);
    });

    // SVG: Hexagons (hidden)
    const hexElements: SVGPolygonElement[] = [];
    hexRadii.forEach((r, i) => {
      const points = Array.from({ length: 6 }, (_, vi) => hexVertex(CX, CY, r, vi))
        .map(([x, y]) => `${x},${y}`).join(' ');
      const poly = document.createElementNS('http://www.w3.org/2000/svg', 'polygon');
      poly.setAttribute('points', points);
      poly.setAttribute('fill', 'none');
      poly.setAttribute('stroke', i === 3 ? COLORS.ember : COLORS.fg);
      poly.setAttribute('stroke-width', i === 0 ? '1' : i === 3 ? '1' : '0.6');
      poly.setAttribute('opacity', '0');
      svg.appendChild(poly);
      hexElements.push(poly);
    });

    // SVG: Center dot
    const centerDot = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
    centerDot.setAttribute('cx', String(CX)); centerDot.setAttribute('cy', String(CY));
    centerDot.setAttribute('r', '10');
    centerDot.setAttribute('fill', COLORS.ember);
    centerDot.setAttribute('opacity', '0');
    svg.appendChild(centerDot);

    // SVG: Isometric cube lines inside ember hex (5 lines from CENTER to vertices)
    // SOLID (front, visible — like hex contour):
    //   v5 (left-top) → center
    //   center → v3 (bottom)
    // DASHED (back, hidden behind cube):
    //   v0 (top) → center
    //   center → v4 (left-bottom)
    //   center → v2 (right-bottom)
    const isoLines: SVGLineElement[] = [];
    const isoDefs = [
      { vertex: 5, dashed: false }, // v5 → center (solid, front-left)
      { vertex: 3, dashed: false }, // center → v3 (solid, front-down)
      { vertex: 0, dashed: true },  // v0 → center (dashed, back-top)
      { vertex: 4, dashed: true },  // center → v4 (dashed, back-left)
      { vertex: 2, dashed: true },  // center → v2 (dashed, back-right-bottom)
      { vertex: 1, dashed: false }, // center → v1 (solid, front-right-top)
    ];
    isoDefs.forEach((def) => {
      const [vx, vy] = hexVertex(CX, CY, hexRadii[3], def.vertex);
      const isoLine = document.createElementNS('http://www.w3.org/2000/svg', 'line');
      isoLine.setAttribute('x1', String(CX)); isoLine.setAttribute('y1', String(CY));
      isoLine.setAttribute('x2', String(vx)); isoLine.setAttribute('y2', String(vy));
      isoLine.setAttribute('stroke', COLORS.ember);
      if (def.dashed) {
        isoLine.setAttribute('stroke-width', '0.5');
        isoLine.setAttribute('stroke-dasharray', '4 4');
      } else {
        isoLine.setAttribute('stroke-width', '1');
      }
      isoLine.setAttribute('opacity', '0');
      svg.appendChild(isoLine);
      isoLines.push(isoLine);
    });

    // SVG: Logo hex (replaces center dot in final state)
    const logoHex = document.createElementNS('http://www.w3.org/2000/svg', 'polygon');
    const logoR = 18;
    const logoPoints = Array.from({ length: 6 }, (_, vi) => hexVertex(CX, CY, logoR, vi))
      .map(([x, y]) => `${x},${y}`).join(' ');
    logoHex.setAttribute('points', logoPoints);
    logoHex.setAttribute('stroke', COLORS.ember);
    logoHex.setAttribute('stroke-width', '2');
    logoHex.setAttribute('fill', 'none');
    logoHex.setAttribute('opacity', '0');
    svg.appendChild(logoHex);

    // SVG: "Forge your plan" text
    const brandText = document.createElementNS('http://www.w3.org/2000/svg', 'text');
    brandText.setAttribute('x', String(CX + 60));
    brandText.setAttribute('y', String(CY + 8));
    brandText.setAttribute('font-family', 'Space Grotesk, system-ui, sans-serif');
    brandText.setAttribute('font-size', '42');
    brandText.setAttribute('font-weight', '400');
    brandText.setAttribute('fill', COLORS.fg);
    brandText.setAttribute('opacity', '0');
    brandText.textContent = 'Forge your plan';
    svg.appendChild(brandText);

    // Final hex positions + label offsets (away from center)
    const finalLinePos = ARTIFACTS.map((_, i) => {
      const vi = i % 6;
      const r = i < 6 ? hexRadii[0] : hexRadii[1];
      const [x1, y1] = hexVertex(CX, CY, r, vi);
      const [x2, y2] = hexVertex(CX, CY, r, (vi + 1) % 6);

      // Label offset: push AWAY from center based on vertex position
      const dirX = x1 - CX;
      const dirY = y1 - CY;
      const dirLen = Math.hypot(dirX, dirY);
      const labelDist = 45; // px away from vertex
      const labelOffX = (dirX / dirLen) * labelDist;
      const labelOffY = (dirY / dirLen) * labelDist;

      return { x1, y1, x2, y2, dotX: x1, dotY: y1, labelOffX, labelOffY };
    });

    // --- TICK ---
    function tick() {
      if (!animating) return;
      const ease = 1 - scrollProgress;

      // --- Line-to-line collision (segment geometry) ---
      if (ease > 0.05) {
        for (let a = 0; a < totalLines; a++) {
          const epA = endpoints(states[a]);
          for (let b = a + 1; b < totalLines; b++) {
            const epB = endpoints(states[b]);
            const { dist, cpA, cpB } = segmentSegmentDist(
              epA[0], epA[1], epA[2], epA[3],
              epB[0], epB[1], epB[2], epB[3]
            );

            if (dist < COLLISION_DIST && dist > 0) {
              const force = (COLLISION_DIST - dist) / COLLISION_DIST * 0.08;
              // Direction from closest point on A to closest point on B
              const nx = (cpB[0] - cpA[0]) / dist;
              const ny = (cpB[1] - cpA[1]) / dist;

              // Push centers apart
              states[a].vx -= nx * force;
              states[a].vy -= ny * force;
              states[b].vx += nx * force;
              states[b].vy += ny * force;

              // Torque: if collision is at tip, spin the line away
              const relAx = cpA[0] - states[a].cx;
              const relAy = cpA[1] - states[a].cy;
              const torqueA = (relAx * (-ny) + relAy * nx) * force * 0.00005;
              states[a].va += torqueA;

              const relBx = cpB[0] - states[b].cx;
              const relBy = cpB[1] - states[b].cy;
              const torqueB = (relBx * ny + relBy * (-nx)) * force * 0.00005;
              states[b].va += torqueB;
            }
          }
        }
      }

      for (let i = 0; i < totalLines; i++) {
        const st = states[i];

        if (ease > 0.05) {
          st.cx += st.vx * ease;
          st.cy += st.vy * ease;
          st.angle += st.va * ease;

          // Clamp speeds
          st.vx = clamp(st.vx, MAX_SPEED);
          st.vy = clamp(st.vy, MAX_SPEED);
          st.va = clamp(st.va, MAX_ANGULAR);

          // Wall bounce (check ENDPOINTS not just center)
          const [ex1, ey1, ex2, ey2] = endpoints(st);
          const minX = Math.min(ex1, ex2), maxX = Math.max(ex1, ex2);
          const minY = Math.min(ey1, ey2), maxY = Math.max(ey1, ey2);

          if (minX < WALL_PAD) { st.cx += WALL_PAD - minX; st.vx = Math.abs(st.vx) * 0.7; }
          if (maxX > W - WALL_PAD) { st.cx -= maxX - (W - WALL_PAD); st.vx = -Math.abs(st.vx) * 0.7; }
          if (minY < WALL_PAD) { st.cy += WALL_PAD - minY; st.vy = Math.abs(st.vy) * 0.7; }
          if (maxY > H - WALL_PAD) { st.cy -= maxY - (H - WALL_PAD); st.vy = -Math.abs(st.vy) * 0.7; }
        }

        // Line endpoints (current)
        const [lx1, ly1, lx2, ly2] = endpoints(st);
        let rx1 = lx1, ry1 = ly1, rx2 = lx2, ry2 = ly2;

        // Scroll: interpolate to hex edges
        if (i < ARTIFACTS.length && scrollProgress > 0) {
          const fp = finalLinePos[i];
          const t = Math.min(scrollProgress * 2.5, 1);
          rx1 = rx1 + (fp.x1 - rx1) * t;
          ry1 = ry1 + (fp.y1 - ry1) * t;
          rx2 = rx2 + (fp.x2 - rx2) * t;
          ry2 = ry2 + (fp.y2 - ry2) * t;
        }

        lines[i].setAttribute('x1', String(rx1));
        lines[i].setAttribute('y1', String(ry1));
        lines[i].setAttribute('x2', String(rx2));
        lines[i].setAttribute('y2', String(ry2));

        // Fade ambient lines
        if (i >= ARTIFACTS.length) {
          lines[i].setAttribute('opacity', String(0.15 * ease));
        }

        // Dot travels along rendered line (independent of line trajectory)
        if (i < ARTIFACTS.length) {
          st.dotProgress += st.dotDir * st.dotSpeed;
          if (st.dotProgress > 1) { st.dotProgress = 1; st.dotDir = -1; }
          if (st.dotProgress < 0) { st.dotProgress = 0; st.dotDir = 1; }

          let dx = rx1 + (rx2 - rx1) * st.dotProgress;
          let dy = ry1 + (ry2 - ry1) * st.dotProgress;

          if (scrollProgress > 0) {
            const fp = finalLinePos[i];
            const t = Math.min(scrollProgress * 2.5, 1);
            dx = dx + (fp.dotX - dx) * t;
            dy = dy + (fp.dotY - dy) * t;
          }

          dots[i].setAttribute('cx', String(dx));
          dots[i].setAttribute('cy', String(dy));

          // Label group at dot position
          labels[i].setAttribute('transform', `translate(${dx}, ${dy})`);

          // Animate inner offset: chaos = (15,-12), crystallized = outward from center
          if (scrollProgress > 0 && i < finalLinePos.length) {
            const fp = finalLinePos[i];
            const t = Math.min(scrollProgress * 2.5, 1);
            const ox = DEFAULT_OX + (fp.labelOffX - DEFAULT_OX) * t;
            const oy = DEFAULT_OY + (fp.labelOffY - DEFAULT_OY) * t;

            labelPointers[i].setAttribute('x2', String(ox));
            labelPointers[i].setAttribute('y2', String(oy));
            labelRects[i].setAttribute('x', String(ox));
            labelRects[i].setAttribute('y', String(oy - 10));
            labelTexts[i].setAttribute('x', String(ox + 6));
            labelTexts[i].setAttribute('y', String(oy + 4));
          }
        }
      }

      // --- PHASE LOGIC ---
      // Phase 1 (0-35%): Lines converge, form hex edges
      // Phase 2 (35-65%): Hexagons FLY IN from outside, biggest first → smallest last
      //                    Lines/dots/labels fade out
      // Phase 3 (65-80%): Outer hexagons fade, only ember hex stays, center dot → logo
      // Phase 4 (80-100%): Hex shifts left, "Forge your plan" types in

      // Phase 2: Hexagons fly in from outside toward center
      // Each hex starts from a scale of 5x (way off screen) and shrinks to 1x
      const hexOp = [0.4, 0.3, 0.25, 0.7];
      const hexStartTimes = [0.3, 0.38, 0.46, 0.54]; // staggered entry, biggest first

      hexElements.forEach((hex, i) => {
        const startT = hexStartTimes[i];
        const endT = startT + 0.15; // each takes 15% of scroll to arrive

        if (scrollProgress < startT) {
          hex.setAttribute('opacity', '0');
          return;
        }

        const progress = Math.min((scrollProgress - startT) / (endT - startT), 1);
        // Ease out cubic for smooth deceleration
        const eased = 1 - Math.pow(1 - progress, 3);
        // Scale: starts at 4x, ends at 1x
        const scale = 4 - 3 * eased;
        const currentR = hexRadii[i] * scale;

        const points = Array.from({ length: 6 }, (_, vi) => hexVertex(CX, CY, currentR, vi))
          .map(([x, y]) => `${x},${y}`).join(' ');
        hex.setAttribute('points', points);

        let opacity = hexOp[i] * eased;

        // Phase 3: Outer hexagons (0,1,2) fade out
        if (i < 3 && scrollProgress > 0.65) {
          const fadeOut = Math.min((scrollProgress - 0.65) / 0.12, 1);
          opacity *= (1 - fadeOut);
        }

        hex.setAttribute('opacity', String(opacity));
      });

      // Phase 2: Fade lines, dots, labels as hexagons arrive
      if (scrollProgress > 0.4) {
        const fadeAll = Math.min((scrollProgress - 0.4) / 0.2, 1);
        for (let li = 0; li < ARTIFACTS.length; li++) {
          lines[li].setAttribute('opacity', String(0.3 * (1 - fadeAll)));
          dots[li].setAttribute('opacity', String(0.7 * (1 - fadeAll)));
          labels[li].setAttribute('opacity', String(1 - fadeAll));
        }
      }

      // Phase 3: Center dot + iso lines appear (dot + ember hex + iso = logo)
      const cdA = Math.max(0, (scrollProgress - 0.5) / 0.15);
      const dotOpacity = 0.9 * Math.min(cdA, 1);
      logoHex.setAttribute('opacity', '0');

      // Iso lines appear with ember hex (slightly after)
      const isoAppear = Math.max(0, (scrollProgress - 0.62) / 0.15);
      const isoOpacity = 0.4 * Math.min(isoAppear, 1);

      // Phase 4: Ember hex + dot + iso lines shift left, text appears
      if (scrollProgress > 0.8) {
        const t = Math.min((scrollProgress - 0.8) / 0.18, 1);
        const easedT = 1 - Math.pow(1 - t, 2);
        const shiftX = -220 * easedT;

        // Move ember hex (index 3)
        const fPoints = Array.from({ length: 6 }, (_, vi) => hexVertex(CX + shiftX, CY, hexRadii[3], vi))
          .map(([x, y]) => `${x},${y}`).join(' ');
        hexElements[3].setAttribute('points', fPoints);

        // Move center dot
        centerDot.setAttribute('cx', String(CX + shiftX));
        centerDot.setAttribute('opacity', String(dotOpacity));

        // Move iso lines (center → vertex), shifted with hex
        isoDefs.forEach((def, idx) => {
          const [vx, vy] = hexVertex(CX + shiftX, CY, hexRadii[3], def.vertex);
          isoLines[idx].setAttribute('x1', String(CX + shiftX)); isoLines[idx].setAttribute('y1', String(CY));
          isoLines[idx].setAttribute('x2', String(vx)); isoLines[idx].setAttribute('y2', String(vy));
          isoLines[idx].setAttribute('opacity', String(isoOpacity));
        });

        // "Forge your plan" — typewriter
        brandText.setAttribute('x', String(CX + shiftX + 100));
        const fullText = 'Forge your plan';
        const chars = Math.floor(easedT * (fullText.length + 2));
        brandText.textContent = fullText.substring(0, Math.min(chars, fullText.length));
        brandText.setAttribute('opacity', String(Math.min(easedT * 2, 1)));
      } else {
        centerDot.setAttribute('cx', String(CX));
        centerDot.setAttribute('opacity', String(dotOpacity));
        brandText.setAttribute('opacity', '0');
        brandText.textContent = '';

        // Reset ember hex + iso lines to center
        const resetPoints = Array.from({ length: 6 }, (_, vi) => hexVertex(CX, CY, hexRadii[3], vi))
          .map(([x, y]) => `${x},${y}`).join(' ');
        hexElements[3].setAttribute('points', resetPoints);

        isoDefs.forEach((def, idx) => {
          const [vx, vy] = hexVertex(CX, CY, hexRadii[3], def.vertex);
          isoLines[idx].setAttribute('x1', String(CX)); isoLines[idx].setAttribute('y1', String(CY));
          isoLines[idx].setAttribute('x2', String(vx)); isoLines[idx].setAttribute('y2', String(vy));
          isoLines[idx].setAttribute('opacity', String(isoOpacity));
        });
      }

      requestAnimationFrame(tick);
    }

    requestAnimationFrame(tick);

    ScrollTrigger.create({
      trigger: containerRef.current,
      start: 'top top',
      end: '+=150%',
      scrub: true,
      pin: true,
      onUpdate: (self) => { scrollProgress = self.progress; },
    });

    return () => {
      animating = false;
      ScrollTrigger.getAll().forEach(st => st.kill());
    };
  }, []);

  return (
    <div ref={containerRef} className="relative w-full min-h-screen flex items-center justify-center overflow-hidden">
      <div className="absolute inset-0 opacity-25 bg-dot-grid" aria-hidden="true" />
      <svg
        ref={svgRef}
        className="w-full h-full absolute inset-0"
        viewBox="0 0 1440 800"
        preserveAspectRatio="xMidYMid meet"
        aria-hidden="true"
      />
    </div>
  );
}

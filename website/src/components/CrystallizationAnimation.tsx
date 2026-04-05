import { useEffect, useRef } from 'react';

// Forge palette
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

// --- Physics ---
const MAX_SPEED = 0.35;
const MAX_ANGULAR = 0.002;
const COLLISION_DIST = 30;
const WALL_PAD = 30;

interface LineState {
  cx: number; cy: number;
  vx: number; vy: number;
  angle: number; va: number;
  len: number;
  dotProgress: number; dotDir: number; dotSpeed: number;
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
    dotSpeed: 0.0004 + Math.random() * 0.0012,
    speed,
  };
}

function endpoints(st: LineState): [number, number, number, number] {
  const hlen = st.len / 2;
  const cos = Math.cos(st.angle), sin = Math.sin(st.angle);
  return [st.cx - cos * hlen, st.cy - sin * hlen, st.cx + cos * hlen, st.cy + sin * hlen];
}

function clamp(v: number, max: number): number {
  return Math.max(-max, Math.min(max, v));
}

function closestPointOnSegment(px: number, py: number, ax: number, ay: number, bx: number, by: number): [number, number] {
  const dx = bx - ax, dy = by - ay;
  const lenSq = dx * dx + dy * dy;
  if (lenSq === 0) return [ax, ay];
  const t = Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / lenSq));
  return [ax + t * dx, ay + t * dy];
}

function segSegDist(a1x: number, a1y: number, a2x: number, a2y: number, b1x: number, b1y: number, b2x: number, b2y: number) {
  const candidates = [
    { cpA: [a1x, a1y] as [number, number], cpB: closestPointOnSegment(a1x, a1y, b1x, b1y, b2x, b2y) },
    { cpA: [a2x, a2y] as [number, number], cpB: closestPointOnSegment(a2x, a2y, b1x, b1y, b2x, b2y) },
    { cpA: closestPointOnSegment(b1x, b1y, a1x, a1y, a2x, a2y), cpB: [b1x, b1y] as [number, number] },
    { cpA: closestPointOnSegment(b2x, b2y, a1x, a1y, a2x, a2y), cpB: [b2x, b2y] as [number, number] },
  ].map(c => ({ ...c, dist: Math.hypot(c.cpA[0] - c.cpB[0], c.cpA[1] - c.cpB[1]) }));
  candidates.sort((a, b) => a.dist - b.dist);
  return candidates[0];
}

interface Props {
  /** Scroll progress 0..1, driven by parent StickySection */
  progress: number;
}

export default function CrystallizationAnimation({ progress }: Props) {
  const svgRef = useRef<SVGSVGElement>(null);
  const statesRef = useRef<LineState[]>([]);
  const animatingRef = useRef(true);
  const progressRef = useRef(0);

  // Keep progress ref in sync
  progressRef.current = progress;

  const W = 1440, H = 800;
  const CX = W / 2, CY = H / 2;
  const hexRadii = [200, 160, 120, 70];

  const isoDefs = [
    { vertex: 5, dashed: false },
    { vertex: 3, dashed: false },
    { vertex: 0, dashed: true },
    { vertex: 4, dashed: true },
    { vertex: 2, dashed: true },
    { vertex: 1, dashed: false },
  ];

  const DEFAULT_OX = 15, DEFAULT_OY = -12;

  useEffect(() => {
    if (!svgRef.current) return;
    const svg = svgRef.current;
    animatingRef.current = true;

    const totalLines = ARTIFACTS.length + 6;
    const states: LineState[] = [];
    for (let i = 0; i < totalLines; i++) states.push(createLineState(W, H));
    statesRef.current = states;

    // --- Build SVG elements ---
    const lines: SVGLineElement[] = [];
    states.forEach((_, idx) => {
      const line = document.createElementNS('http://www.w3.org/2000/svg', 'line');
      line.setAttribute('stroke', idx < ARTIFACTS.length ? COLORS.fg : COLORS.line);
      line.setAttribute('stroke-width', idx < ARTIFACTS.length ? '0.8' : '0.4');
      line.setAttribute('opacity', idx < ARTIFACTS.length ? '0.3' : '0.15');
      svg.appendChild(line); lines.push(line);
    });

    const dots: SVGCircleElement[] = [];
    ARTIFACTS.forEach((art) => {
      const dot = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
      dot.setAttribute('r', String(3 + Math.random() * 5));
      dot.setAttribute('fill', art.color); dot.setAttribute('opacity', '0.7');
      svg.appendChild(dot); dots.push(dot);
    });

    const labels: SVGGElement[] = [];
    const labelPointers: SVGLineElement[] = [];
    const labelRects: SVGRectElement[] = [];
    const labelTexts: SVGTextElement[] = [];
    ARTIFACTS.forEach((art) => {
      const g = document.createElementNS('http://www.w3.org/2000/svg', 'g');
      const pointer = document.createElementNS('http://www.w3.org/2000/svg', 'line');
      pointer.setAttribute('x1', '0'); pointer.setAttribute('y1', '0');
      pointer.setAttribute('x2', String(DEFAULT_OX)); pointer.setAttribute('y2', String(DEFAULT_OY));
      pointer.setAttribute('stroke', art.color); pointer.setAttribute('stroke-width', '0.5'); pointer.setAttribute('opacity', '0.5');
      g.appendChild(pointer); labelPointers.push(pointer);

      const textLen = art.label.length * 7.5 + 12;
      const rect = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
      rect.setAttribute('x', String(DEFAULT_OX)); rect.setAttribute('y', String(DEFAULT_OY - 10));
      rect.setAttribute('width', String(textLen)); rect.setAttribute('height', '20');
      rect.setAttribute('fill', COLORS.surface);
      rect.setAttribute('stroke', art.color === COLORS.ember ? COLORS.ember : COLORS.line);
      rect.setAttribute('stroke-width', '0.5');
      g.appendChild(rect); labelRects.push(rect);

      const text = document.createElementNS('http://www.w3.org/2000/svg', 'text');
      text.setAttribute('x', String(DEFAULT_OX + 6)); text.setAttribute('y', String(DEFAULT_OY + 4));
      text.setAttribute('font-family', 'Geist Mono, monospace'); text.setAttribute('font-size', '10');
      text.setAttribute('fill', art.color); text.textContent = art.label;
      g.appendChild(text); labelTexts.push(text);
      svg.appendChild(g); labels.push(g);
    });

    const hexElements: SVGPolygonElement[] = [];
    hexRadii.forEach((r, i) => {
      const points = Array.from({ length: 6 }, (_, vi) => hexVertex(CX, CY, r, vi)).map(([x, y]) => `${x},${y}`).join(' ');
      const poly = document.createElementNS('http://www.w3.org/2000/svg', 'polygon');
      poly.setAttribute('points', points); poly.setAttribute('fill', 'none');
      poly.setAttribute('stroke', i === 3 ? COLORS.ember : COLORS.fg);
      poly.setAttribute('stroke-width', i === 0 ? '1' : i === 3 ? '1' : '0.6');
      poly.setAttribute('opacity', '0');
      svg.appendChild(poly); hexElements.push(poly);
    });

    const isoLines: SVGLineElement[] = [];
    isoDefs.forEach((def) => {
      const [vx, vy] = hexVertex(CX, CY, hexRadii[3], def.vertex);
      const isoLine = document.createElementNS('http://www.w3.org/2000/svg', 'line');
      isoLine.setAttribute('x1', String(CX)); isoLine.setAttribute('y1', String(CY));
      isoLine.setAttribute('x2', String(vx)); isoLine.setAttribute('y2', String(vy));
      isoLine.setAttribute('stroke', COLORS.ember);
      isoLine.setAttribute('stroke-width', def.dashed ? '0.5' : '1');
      if (def.dashed) isoLine.setAttribute('stroke-dasharray', '4 4');
      isoLine.setAttribute('opacity', '0');
      svg.appendChild(isoLine); isoLines.push(isoLine);
    });

    const centerDot = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
    centerDot.setAttribute('cx', String(CX)); centerDot.setAttribute('cy', String(CY));
    centerDot.setAttribute('r', '10'); centerDot.setAttribute('fill', COLORS.ember); centerDot.setAttribute('opacity', '0');
    svg.appendChild(centerDot);

    const brandText = document.createElementNS('http://www.w3.org/2000/svg', 'text');
    brandText.setAttribute('x', String(CX + 60)); brandText.setAttribute('y', String(CY + 8));
    brandText.setAttribute('font-family', 'Space Grotesk, system-ui, sans-serif');
    brandText.setAttribute('font-size', '42'); brandText.setAttribute('font-weight', '400');
    brandText.setAttribute('fill', COLORS.fg); brandText.setAttribute('opacity', '0');
    svg.appendChild(brandText);

    // Final positions
    const finalLinePos = ARTIFACTS.map((_, i) => {
      const vi = i % 6;
      const r = i < 6 ? hexRadii[0] : hexRadii[1];
      const [x1, y1] = hexVertex(CX, CY, r, vi);
      const [x2, y2] = hexVertex(CX, CY, r, (vi + 1) % 6);
      const dirX = x1 - CX, dirY = y1 - CY;
      const dirLen = Math.hypot(dirX, dirY);
      return { x1, y1, x2, y2, dotX: x1, dotY: y1, labelOffX: (dirX / dirLen) * 45, labelOffY: (dirY / dirLen) * 45 };
    });

    // --- TICK ---
    function tick() {
      if (!animatingRef.current) return;
      const sp = progressRef.current;
      const ease = 1 - sp;

      // Collision
      if (ease > 0.05) {
        for (let a = 0; a < totalLines; a++) {
          const epA = endpoints(states[a]);
          for (let b = a + 1; b < totalLines; b++) {
            const epB = endpoints(states[b]);
            const { dist, cpA, cpB } = segSegDist(epA[0], epA[1], epA[2], epA[3], epB[0], epB[1], epB[2], epB[3]);
            if (dist < COLLISION_DIST && dist > 0) {
              const force = (COLLISION_DIST - dist) / COLLISION_DIST * 0.08;
              const nx = (cpB[0] - cpA[0]) / dist, ny = (cpB[1] - cpA[1]) / dist;
              states[a].vx -= nx * force; states[a].vy -= ny * force;
              states[b].vx += nx * force; states[b].vy += ny * force;
              const relAx = cpA[0] - states[a].cx, relAy = cpA[1] - states[a].cy;
              states[a].va += (relAx * (-ny) + relAy * nx) * force * 0.00005;
              const relBx = cpB[0] - states[b].cx, relBy = cpB[1] - states[b].cy;
              states[b].va += (relBx * ny + relBy * (-nx)) * force * 0.00005;
            }
          }
        }
      }

      for (let i = 0; i < totalLines; i++) {
        const st = states[i];
        if (ease > 0.05) {
          st.cx += st.vx * ease; st.cy += st.vy * ease; st.angle += st.va * ease;
          st.vx = clamp(st.vx, MAX_SPEED); st.vy = clamp(st.vy, MAX_SPEED); st.va = clamp(st.va, MAX_ANGULAR);
          const [ex1, ey1, ex2, ey2] = endpoints(st);
          const minX = Math.min(ex1, ex2), maxX = Math.max(ex1, ex2);
          const minY = Math.min(ey1, ey2), maxY = Math.max(ey1, ey2);
          if (minX < WALL_PAD) { st.cx += WALL_PAD - minX; st.vx = Math.abs(st.vx) * 0.7; }
          if (maxX > W - WALL_PAD) { st.cx -= maxX - (W - WALL_PAD); st.vx = -Math.abs(st.vx) * 0.7; }
          if (minY < WALL_PAD) { st.cy += WALL_PAD - minY; st.vy = Math.abs(st.vy) * 0.7; }
          if (maxY > H - WALL_PAD) { st.cy -= maxY - (H - WALL_PAD); st.vy = -Math.abs(st.vy) * 0.7; }
        }

        const [lx1, ly1, lx2, ly2] = endpoints(st);
        let rx1 = lx1, ry1 = ly1, rx2 = lx2, ry2 = ly2;
        if (i < ARTIFACTS.length && sp > 0) {
          const fp = finalLinePos[i];
          const t = Math.min(sp * 2.5, 1);
          rx1 += (fp.x1 - rx1) * t; ry1 += (fp.y1 - ry1) * t;
          rx2 += (fp.x2 - rx2) * t; ry2 += (fp.y2 - ry2) * t;
        }
        lines[i].setAttribute('x1', String(rx1)); lines[i].setAttribute('y1', String(ry1));
        lines[i].setAttribute('x2', String(rx2)); lines[i].setAttribute('y2', String(ry2));
        if (i >= ARTIFACTS.length) lines[i].setAttribute('opacity', String(0.15 * ease));

        if (i < ARTIFACTS.length) {
          st.dotProgress += st.dotDir * st.dotSpeed;
          if (st.dotProgress > 1) { st.dotProgress = 1; st.dotDir = -1; }
          if (st.dotProgress < 0) { st.dotProgress = 0; st.dotDir = 1; }
          let dx = rx1 + (rx2 - rx1) * st.dotProgress, dy = ry1 + (ry2 - ry1) * st.dotProgress;
          if (sp > 0) {
            const fp = finalLinePos[i]; const t = Math.min(sp * 2.5, 1);
            dx += (fp.dotX - dx) * t; dy += (fp.dotY - dy) * t;
          }
          dots[i].setAttribute('cx', String(dx)); dots[i].setAttribute('cy', String(dy));
          labels[i].setAttribute('transform', `translate(${dx}, ${dy})`);
          if (sp > 0 && i < finalLinePos.length) {
            const fp = finalLinePos[i]; const t = Math.min(sp * 2.5, 1);
            const ox = DEFAULT_OX + (fp.labelOffX - DEFAULT_OX) * t;
            const oy = DEFAULT_OY + (fp.labelOffY - DEFAULT_OY) * t;
            labelPointers[i].setAttribute('x2', String(ox)); labelPointers[i].setAttribute('y2', String(oy));
            labelRects[i].setAttribute('x', String(ox)); labelRects[i].setAttribute('y', String(oy - 10));
            labelTexts[i].setAttribute('x', String(ox + 6)); labelTexts[i].setAttribute('y', String(oy + 4));
          }
        }
      }

      // Hexagons fly in
      const hexOp = [0.4, 0.3, 0.25, 0.7];
      const hexStart = [0.3, 0.38, 0.46, 0.54];
      hexElements.forEach((hex, i) => {
        if (sp < hexStart[i]) { hex.setAttribute('opacity', '0'); return; }
        const p = Math.min((sp - hexStart[i]) / 0.15, 1);
        const eased = 1 - Math.pow(1 - p, 3);
        const scale = 4 - 3 * eased;
        const pts = Array.from({ length: 6 }, (_, vi) => hexVertex(CX, CY, hexRadii[i] * scale, vi)).map(([x, y]) => `${x},${y}`).join(' ');
        hex.setAttribute('points', pts);
        let op = hexOp[i] * eased;
        if (i < 3 && sp > 0.65) op *= Math.max(0, 1 - (sp - 0.65) / 0.12);
        hex.setAttribute('opacity', String(op));
      });

      // Fade lines/dots/labels
      if (sp > 0.4) {
        const f = Math.min((sp - 0.4) / 0.2, 1);
        for (let li = 0; li < ARTIFACTS.length; li++) {
          lines[li].setAttribute('opacity', String(0.3 * (1 - f)));
          dots[li].setAttribute('opacity', String(0.7 * (1 - f)));
          labels[li].setAttribute('opacity', String(1 - f));
        }
      }

      // Center dot + iso lines
      const cdA = Math.max(0, (sp - 0.5) / 0.15);
      const dotOp = 0.9 * Math.min(cdA, 1);
      const isoA = Math.max(0, (sp - 0.62) / 0.15);
      const isoOp = 0.4 * Math.min(isoA, 1);

      if (sp > 0.8) {
        const t = Math.min((sp - 0.8) / 0.18, 1);
        const et = 1 - Math.pow(1 - t, 2);
        const sx = -220 * et;
        hexElements[3].setAttribute('points', Array.from({ length: 6 }, (_, vi) => hexVertex(CX + sx, CY, hexRadii[3], vi)).map(([x, y]) => `${x},${y}`).join(' '));
        centerDot.setAttribute('cx', String(CX + sx)); centerDot.setAttribute('opacity', String(dotOp));
        isoDefs.forEach((def, idx) => {
          const [vx, vy] = hexVertex(CX + sx, CY, hexRadii[3], def.vertex);
          isoLines[idx].setAttribute('x1', String(CX + sx)); isoLines[idx].setAttribute('y1', String(CY));
          isoLines[idx].setAttribute('x2', String(vx)); isoLines[idx].setAttribute('y2', String(vy));
          isoLines[idx].setAttribute('opacity', String(isoOp));
        });
        brandText.setAttribute('x', String(CX + sx + 100));
        const full = 'Forge your plan';
        brandText.textContent = full.substring(0, Math.min(Math.floor(et * (full.length + 2)), full.length));
        brandText.setAttribute('opacity', String(Math.min(et * 2, 1)));
      } else {
        centerDot.setAttribute('cx', String(CX)); centerDot.setAttribute('opacity', String(dotOp));
        brandText.setAttribute('opacity', '0'); brandText.textContent = '';
        hexElements[3].setAttribute('points', Array.from({ length: 6 }, (_, vi) => hexVertex(CX, CY, hexRadii[3], vi)).map(([x, y]) => `${x},${y}`).join(' '));
        isoDefs.forEach((def, idx) => {
          const [vx, vy] = hexVertex(CX, CY, hexRadii[3], def.vertex);
          isoLines[idx].setAttribute('x1', String(CX)); isoLines[idx].setAttribute('y1', String(CY));
          isoLines[idx].setAttribute('x2', String(vx)); isoLines[idx].setAttribute('y2', String(vy));
          isoLines[idx].setAttribute('opacity', String(isoOp));
        });
      }

      requestAnimationFrame(tick);
    }

    requestAnimationFrame(tick);
    return () => { animatingRef.current = false; };
  }, []);

  return (
    <svg
      ref={svgRef}
      className="w-full h-full absolute inset-0"
      viewBox={`0 0 ${W} ${H}`}
      preserveAspectRatio="xMidYMid meet"
      aria-hidden="true"
    />
  );
}

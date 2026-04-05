import { useEffect, useRef } from 'react';
import { COLORS, hexVertex } from '../tokens';

// --- Constants (module-level, never change) ---
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

const W = 1440, H = 800;
const CX = W / 2, CY = H / 2;
const HEX_RADII = [200, 160, 120, 70];
const MAX_SPEED = 0.35;
const MAX_ANGULAR = 0.002;
const COLLISION_DIST = 30;
const COLLISION_EPSILON = 0.5; // prevents NaN from near-zero dist
const WALL_PAD = 30;
const DEFAULT_OX = 15, DEFAULT_OY = -12;

const ISO_DEFS = [
  { vertex: 5, dashed: false },
  { vertex: 3, dashed: false },
  { vertex: 0, dashed: true },
  { vertex: 4, dashed: true },
  { vertex: 2, dashed: true },
  { vertex: 1, dashed: false },
];

// --- Physics types ---
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

function closestPointOnSegment(
  px: number, py: number, ax: number, ay: number, bx: number, by: number
): [number, number] {
  const dx = bx - ax, dy = by - ay;
  const lenSq = dx * dx + dy * dy;
  if (lenSq === 0) return [ax, ay];
  const t = Math.max(0, Math.min(1, ((px - ax) * dx + (py - ay) * dy) / lenSq));
  return [ax + t * dx, ay + t * dy];
}

// Zero-alloc min-distance between two segments (fixes GC pressure)
function segSegDist(
  a1x: number, a1y: number, a2x: number, a2y: number,
  b1x: number, b1y: number, b2x: number, b2y: number
): { dist: number; cpAx: number; cpAy: number; cpBx: number; cpBy: number } {
  let minDist = Infinity, bestAx = 0, bestAy = 0, bestBx = 0, bestBy = 0;

  // a1 → segB
  const [p1x, p1y] = closestPointOnSegment(a1x, a1y, b1x, b1y, b2x, b2y);
  let d = Math.hypot(a1x - p1x, a1y - p1y);
  if (d < minDist) { minDist = d; bestAx = a1x; bestAy = a1y; bestBx = p1x; bestBy = p1y; }

  // a2 → segB
  const [p2x, p2y] = closestPointOnSegment(a2x, a2y, b1x, b1y, b2x, b2y);
  d = Math.hypot(a2x - p2x, a2y - p2y);
  if (d < minDist) { minDist = d; bestAx = a2x; bestAy = a2y; bestBx = p2x; bestBy = p2y; }

  // b1 → segA
  const [p3x, p3y] = closestPointOnSegment(b1x, b1y, a1x, a1y, a2x, a2y);
  d = Math.hypot(b1x - p3x, b1y - p3y);
  if (d < minDist) { minDist = d; bestAx = p3x; bestAy = p3y; bestBx = b1x; bestBy = b1y; }

  // b2 → segA
  const [p4x, p4y] = closestPointOnSegment(b2x, b2y, a1x, a1y, a2x, a2y);
  d = Math.hypot(b2x - p4x, b2y - p4y);
  if (d < minDist) { minDist = d; bestAx = p4x; bestAy = p4y; bestBx = b2x; bestBy = b2y; }

  return { dist: minDist, cpAx: bestAx, cpAy: bestAy, cpBx: bestBx, cpBy: bestBy };
}

// Final positions for scroll convergence
const FINAL_LINE_POS = ARTIFACTS.map((_, i) => {
  const vi = i % 6;
  const r = i < 6 ? HEX_RADII[0] : HEX_RADII[1];
  const [x1, y1] = hexVertex(CX, CY, r, vi);
  const [x2, y2] = hexVertex(CX, CY, r, (vi + 1) % 6);
  const dirX = x1 - CX, dirY = y1 - CY;
  const dirLen = Math.hypot(dirX, dirY) || 1; // guard zero
  return { x1, y1, x2, y2, dotX: x1, dotY: y1, labelOffX: (dirX / dirLen) * 45, labelOffY: (dirY / dirLen) * 45 };
});

// --- Component ---
interface Props {
  progress: number;
}

export default function CrystallizationAnimation({ progress }: Props) {
  const svgRef = useRef<SVGSVGElement>(null);
  const progressRef = useRef(0);
  const rafRef = useRef(0);
  const animatingRef = useRef(true);

  progressRef.current = progress;

  useEffect(() => {
    if (!svgRef.current) return;

    const svg = svgRef.current;
    animatingRef.current = true;

    const totalLines = ARTIFACTS.length + 6;
    const states: LineState[] = [];
    for (let i = 0; i < totalLines; i++) states.push(createLineState(W, H));

    // === BUILD SVG ELEMENTS ===
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
      const countW = 28; // space for "×XX"
      const textLen = art.label.length * 7.5 + 12 + countW;
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

      // Count badge "×XX" — grows during chaos
      const count = document.createElementNS('http://www.w3.org/2000/svg', 'text');
      count.setAttribute('x', String(DEFAULT_OX + art.label.length * 7.5 + 14));
      count.setAttribute('y', String(DEFAULT_OY + 4));
      count.setAttribute('font-family', 'Geist Mono, monospace'); count.setAttribute('font-size', '9');
      count.setAttribute('fill', COLORS.dim);
      count.textContent = '';
      g.appendChild(count);

      svg.appendChild(g); labels.push(g);
    });

    // Base counts for each artifact (randomized, grow during chaos)
    const baseCounts = ARTIFACTS.map(() => 8 + Math.floor(Math.random() * 40));

    const hexElements: SVGPolygonElement[] = [];
    HEX_RADII.forEach((r, i) => {
      const points = Array.from({ length: 6 }, (_, vi) => hexVertex(CX, CY, r, vi)).map(([x, y]) => `${x},${y}`).join(' ');
      const poly = document.createElementNS('http://www.w3.org/2000/svg', 'polygon');
      poly.setAttribute('points', points); poly.setAttribute('fill', 'none');
      poly.setAttribute('stroke', i === 3 ? COLORS.ember : COLORS.fg);
      poly.setAttribute('stroke-width', i === 0 ? '1' : i === 3 ? '1' : '0.6');
      poly.setAttribute('opacity', '0');
      svg.appendChild(poly); hexElements.push(poly);
    });

    // DAG edges between hex layers (vertex-to-vertex connections)
    // Connect each vertex of outer hex to same vertex of next inner hex
    const dagEdges: SVGLineElement[] = [];
    for (let layer = 0; layer < HEX_RADII.length - 1; layer++) {
      for (let vi = 0; vi < 6; vi += 2) { // every other vertex = 3 edges per layer gap
        const [x1, y1] = hexVertex(CX, CY, HEX_RADII[layer], vi);
        const [x2, y2] = hexVertex(CX, CY, HEX_RADII[layer + 1], vi);
        const edge = document.createElementNS('http://www.w3.org/2000/svg', 'line');
        edge.setAttribute('x1', String(x1)); edge.setAttribute('y1', String(y1));
        edge.setAttribute('x2', String(x2)); edge.setAttribute('y2', String(y2));
        edge.setAttribute('stroke', COLORS.fg);
        edge.setAttribute('stroke-width', '0.4');
        edge.setAttribute('stroke-dasharray', '3 3');
        edge.setAttribute('opacity', '0');
        svg.appendChild(edge);
        dagEdges.push(edge);
      }
    }

    const isoLines: SVGLineElement[] = [];
    ISO_DEFS.forEach((def) => {
      const [vx, vy] = hexVertex(CX, CY, HEX_RADII[3], def.vertex);
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
    centerDot.setAttribute('r', '12'); centerDot.setAttribute('fill', COLORS.ember); centerDot.setAttribute('opacity', '0');
    svg.appendChild(centerDot);

    // Brand text as two parts: "F" + dot (replaces "o") + "rge your plan"
    const brandF = document.createElementNS('http://www.w3.org/2000/svg', 'text');
    brandF.setAttribute('font-family', 'Space Grotesk, system-ui, sans-serif');
    brandF.setAttribute('font-size', '42'); brandF.setAttribute('font-weight', '400');
    brandF.setAttribute('fill', COLORS.fg); brandF.setAttribute('opacity', '0');
    brandF.textContent = 'F';
    svg.appendChild(brandF);

    const brandRest = document.createElementNS('http://www.w3.org/2000/svg', 'text');
    brandRest.setAttribute('font-family', 'Space Grotesk, system-ui, sans-serif');
    brandRest.setAttribute('font-size', '42'); brandRest.setAttribute('font-weight', '400');
    brandRest.setAttribute('fill', COLORS.fg); brandRest.setAttribute('opacity', '0');
    svg.appendChild(brandRest);

    // Subtitle with shuffle/decode effect: "Structure. Evidence. Trust."
    const subtitle = document.createElementNS('http://www.w3.org/2000/svg', 'text');
    subtitle.setAttribute('font-family', 'Geist Mono, monospace');
    subtitle.setAttribute('font-size', '14'); subtitle.setAttribute('font-weight', '400');
    subtitle.setAttribute('fill', COLORS.ember); subtitle.setAttribute('opacity', '0');
    subtitle.setAttribute('letter-spacing', '2');
    svg.appendChild(subtitle);

    const SUBTITLE_TEXT = 'Structure. Evidence. Trust.';
    const SHUFFLE_CHARS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789.!@#$%';

    // Narrative connectors — dashed lines from edges to artifact dots (4 pairs × 2)
    const CONNECTOR_DEFS = [
      // Pair 1 (scroll 2-12%)
      { dotIdx: 0, edgeX: 80,     edgeYPct: 0.22, start: 0.02, end: 0.12 },
      { dotIdx: 1, edgeX: W - 80, edgeYPct: 0.22, start: 0.02, end: 0.12 },
      // Pair 2 (scroll 9-19%)
      { dotIdx: 2, edgeX: 80,     edgeYPct: 0.40, start: 0.09, end: 0.19 },
      { dotIdx: 6, edgeX: W - 80, edgeYPct: 0.40, start: 0.09, end: 0.19 },
      // Pair 3 (scroll 16-26%)
      { dotIdx: 4, edgeX: 80,     edgeYPct: 0.58, start: 0.16, end: 0.26 },
      { dotIdx: 5, edgeX: W - 80, edgeYPct: 0.58, start: 0.16, end: 0.26 },
      // Pair 4 (scroll 23-35%)
      { dotIdx: 3, edgeX: 80,     edgeYPct: 0.76, start: 0.23, end: 0.35 },
      { dotIdx: 7, edgeX: W - 80, edgeYPct: 0.76, start: 0.23, end: 0.35 },
    ];
    const connectors: SVGLineElement[] = [];
    CONNECTOR_DEFS.forEach(() => {
      const cl = document.createElementNS('http://www.w3.org/2000/svg', 'line');
      cl.setAttribute('stroke', COLORS.ember);
      cl.setAttribute('stroke-width', '0.5');
      cl.setAttribute('stroke-dasharray', '4 4');
      cl.setAttribute('opacity', '0');
      svg.appendChild(cl);
      connectors.push(cl);
    });

    // === ANIMATION LOOP ===
    function tick() {
      if (!animatingRef.current) return;
      const sp = progressRef.current;
      const ease = 1 - sp;

      // Collision (pre-compute endpoints once per line)
      const allEp: [number, number, number, number][] = [];
      for (let i = 0; i < totalLines; i++) allEp.push(endpoints(states[i]));

      if (ease > 0.05) {
        for (let a = 0; a < totalLines; a++) {
          for (let b = a + 1; b < totalLines; b++) {
            const { dist, cpAx, cpAy, cpBx, cpBy } = segSegDist(
              allEp[a][0], allEp[a][1], allEp[a][2], allEp[a][3],
              allEp[b][0], allEp[b][1], allEp[b][2], allEp[b][3]
            );
            if (dist < COLLISION_DIST && dist > COLLISION_EPSILON) {
              const force = (COLLISION_DIST - dist) / COLLISION_DIST * 0.08;
              const nx = (cpBx - cpAx) / dist, ny = (cpBy - cpAy) / dist;
              states[a].vx -= nx * force; states[a].vy -= ny * force;
              states[b].vx += nx * force; states[b].vy += ny * force;
              const relAx = cpAx - states[a].cx, relAy = cpAy - states[a].cy;
              states[a].va += (relAx * (-ny) + relAy * nx) * force * 0.00005;
              const relBx = cpBx - states[b].cx, relBy = cpBy - states[b].cy;
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
          const ep = allEp[i];
          const minX = Math.min(ep[0], ep[2]), maxX = Math.max(ep[0], ep[2]);
          const minY = Math.min(ep[1], ep[3]), maxY = Math.max(ep[1], ep[3]);
          if (minX < WALL_PAD) { st.cx += WALL_PAD - minX; st.vx = Math.abs(st.vx) * 0.7; }
          if (maxX > W - WALL_PAD) { st.cx -= maxX - (W - WALL_PAD); st.vx = -Math.abs(st.vx) * 0.7; }
          if (minY < WALL_PAD) { st.cy += WALL_PAD - minY; st.vy = Math.abs(st.vy) * 0.7; }
          if (maxY > H - WALL_PAD) { st.cy -= maxY - (H - WALL_PAD); st.vy = -Math.abs(st.vy) * 0.7; }
        }

        const [lx1, ly1, lx2, ly2] = endpoints(st);
        let rx1 = lx1, ry1 = ly1, rx2 = lx2, ry2 = ly2;
        if (i < ARTIFACTS.length && sp > 0) {
          const fp = FINAL_LINE_POS[i];
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
            const fp = FINAL_LINE_POS[i]; const t = Math.min(sp * 2.5, 1);
            dx += (fp.dotX - dx) * t; dy += (fp.dotY - dy) * t;
          }
          dots[i].setAttribute('cx', String(dx)); dots[i].setAttribute('cy', String(dy));
          labels[i].setAttribute('transform', `translate(${dx}, ${dy})`);

          // Count badge — ticks up/down based on dot direction
          const countEl = labels[i].querySelector('text:last-child') as SVGTextElement;
          if (countEl && sp < 0.4) {
            // dotProgress 0→1 = count goes up, 1→0 = count goes down
            const base = baseCounts[i];
            const swing = Math.floor(st.dotProgress * 30);
            countEl.textContent = `×${base + swing}`;
          } else if (countEl) {
            countEl.textContent = '';
          }
          if (sp > 0 && i < FINAL_LINE_POS.length) {
            const fp = FINAL_LINE_POS[i]; const t = Math.min(sp * 2.5, 1);
            const ox = DEFAULT_OX + (fp.labelOffX - DEFAULT_OX) * t;
            const oy = DEFAULT_OY + (fp.labelOffY - DEFAULT_OY) * t;
            labelPointers[i].setAttribute('x2', String(ox)); labelPointers[i].setAttribute('y2', String(oy));
            labelRects[i].setAttribute('x', String(ox)); labelRects[i].setAttribute('y', String(oy - 10));
            labelTexts[i].setAttribute('x', String(ox + 6)); labelTexts[i].setAttribute('y', String(oy + 4));
          }
        }
      }

      // Narrative connectors — dashed lines from edge to artifact dots
      CONNECTOR_DEFS.forEach((cd, ci) => {
        const fadeIn = Math.min(Math.max((sp - cd.start) / 0.03, 0), 1);
        const fadeOut = Math.min(Math.max((cd.end - sp) / 0.03, 0), 1);
        const connOp = fadeIn * fadeOut * 0.4;

        if (connOp > 0) {
          const dot = dots[cd.dotIdx];
          const dx = parseFloat(dot.getAttribute('cx') || '0');
          const dy = parseFloat(dot.getAttribute('cy') || '0');
          connectors[ci].setAttribute('x1', String(cd.edgeX));
          connectors[ci].setAttribute('y1', String(H * cd.edgeYPct));
          connectors[ci].setAttribute('x2', String(dx));
          connectors[ci].setAttribute('y2', String(dy));
        }
        connectors[ci].setAttribute('opacity', String(connOp));
      });

      // Hexagons fly in
      const hexOp = [0.4, 0.3, 0.25, 0.7];
      const hexStart = [0.3, 0.38, 0.46, 0.54];
      hexElements.forEach((hex, i) => {
        if (sp < hexStart[i]) { hex.setAttribute('opacity', '0'); return; }
        const p = Math.min((sp - hexStart[i]) / 0.15, 1);
        const eased = 1 - Math.pow(1 - p, 3);
        const scale = 4 - 3 * eased;
        const pts = Array.from({ length: 6 }, (_, vi) => hexVertex(CX, CY, HEX_RADII[i] * scale, vi)).map(([x, y]) => `${x},${y}`).join(' ');
        hex.setAttribute('points', pts);
        let op = hexOp[i] * eased;
        if (i < 3 && sp > 0.65) op *= Math.max(0, 1 - (sp - 0.65) / 0.12);
        hex.setAttribute('opacity', String(op));
      });

      // DAG edges — appear after hex formation, fade with outer hexes
      dagEdges.forEach((edge) => {
        const dagAppear = Math.max(0, (sp - 0.55) / 0.1);
        let dagOp = 0.3 * Math.min(dagAppear, 1);
        // Fade with outer hexes
        if (sp > 0.65) dagOp *= Math.max(0, 1 - (sp - 0.65) / 0.12);
        edge.setAttribute('opacity', String(dagOp));
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
        // Phase 4a: cube shifts left (80-88%)
        const cubeT = Math.min((sp - 0.8) / 0.08, 1);
        const cubeEased = 1 - Math.pow(1 - cubeT, 2);
        const sx = -220 * cubeEased;

        // Move hex + iso lines
        hexElements[3].setAttribute('points', Array.from({ length: 6 }, (_, vi) => hexVertex(CX + sx, CY, HEX_RADII[3], vi)).map(([x, y]) => `${x},${y}`).join(' '));
        ISO_DEFS.forEach((def, idx) => {
          const [vx, vy] = hexVertex(CX + sx, CY, HEX_RADII[3], def.vertex);
          isoLines[idx].setAttribute('x1', String(CX + sx)); isoLines[idx].setAttribute('y1', String(CY));
          isoLines[idx].setAttribute('x2', String(vx)); isoLines[idx].setAttribute('y2', String(vy));
          isoLines[idx].setAttribute('opacity', String(isoOp));
        });

        // Layout: "F" + ● (dot) + "rge your plan"
        const textX = CX - 120;
        const baseY = CY + 8;                     // text baseline
        const fWidth = 24;                         // "F" glyph width at 42px
        const dotR = 12;                           // dot radius
        const gap = 4;                             // space around dot
        const dotTargetX = textX + fWidth + gap + dotR;  // dot center X
        const rgeX = dotTargetX + dotR + gap;      // "rge your plan" start X
        const textCenterY = baseY - 42 * 0.25;    // vertical center of lowercase (rge)

        // Phase 4b: dot flies from cube to "o" position (88-93%)
        if (sp < 0.88) {
          centerDot.setAttribute('cx', String(CX + sx));
          centerDot.setAttribute('cy', String(CY));
        } else {
          const dt = Math.min((sp - 0.88) / 0.05, 1);
          const de = 1 - Math.pow(1 - dt, 3);
          centerDot.setAttribute('cx', String((CX + sx) + (dotTargetX - (CX + sx)) * de));
          centerDot.setAttribute('cy', String(CY + (textCenterY - CY) * de));
        }
        centerDot.setAttribute('opacity', String(dotOp));

        // Phase 4c: "F" + "rge your plan" typewriter (88-100%)
        if (sp > 0.88) {
          const tt = Math.min((sp - 0.88) / 0.12, 1);
          const op = Math.min(tt * 3, 1);

          // "F" appears first
          brandF.setAttribute('x', String(textX));
          brandF.setAttribute('y', String(baseY));
          brandF.setAttribute('opacity', String(op));

          // "rge your plan" typewriter with slight delay
          const rest = 'rge your plan';
          const chars = Math.max(0, Math.floor((tt - 0.12) / 0.88 * (rest.length + 1)));
          brandRest.setAttribute('x', String(rgeX));
          brandRest.setAttribute('y', String(baseY));
          brandRest.textContent = rest.substring(0, Math.min(chars, rest.length));
          brandRest.setAttribute('opacity', String(op));

          // Subtitle: "Structure. Evidence. Trust." — shuffle/decode effect
          subtitle.setAttribute('x', String(textX));
          subtitle.setAttribute('y', String(baseY + 28));
          const subT = Math.max(0, (tt - 0.85) / 0.15); // starts only after full text visible
          if (subT > 0) {
            subtitle.setAttribute('opacity', String(Math.min(subT * 2, 1)));
            // Shuffle: each char resolves progressively
            let decoded = '';
            for (let ci = 0; ci < SUBTITLE_TEXT.length; ci++) {
              const charProgress = subT * SUBTITLE_TEXT.length * 1.5 - ci;
              if (charProgress > 1) {
                decoded += SUBTITLE_TEXT[ci]; // resolved
              } else if (charProgress > 0) {
                decoded += SHUFFLE_CHARS[Math.floor(Math.random() * SHUFFLE_CHARS.length)]; // shuffling
              }
              // else: not yet started
            }
            subtitle.textContent = decoded;
          } else {
            subtitle.setAttribute('opacity', '0');
            subtitle.textContent = '';
          }
        } else {
          brandF.setAttribute('opacity', '0');
          brandRest.setAttribute('opacity', '0');
          brandRest.textContent = '';
          subtitle.setAttribute('opacity', '0');
          subtitle.textContent = '';
        }
      } else {
        // Before phase 4: everything at center
        centerDot.setAttribute('cx', String(CX));
        centerDot.setAttribute('cy', String(CY));
        centerDot.setAttribute('opacity', String(dotOp));
        brandF.setAttribute('opacity', '0');
        brandRest.setAttribute('opacity', '0'); brandRest.textContent = '';
        subtitle.setAttribute('opacity', '0'); subtitle.textContent = '';
        hexElements[3].setAttribute('points', Array.from({ length: 6 }, (_, vi) => hexVertex(CX, CY, HEX_RADII[3], vi)).map(([x, y]) => `${x},${y}`).join(' '));
        ISO_DEFS.forEach((def, idx) => {
          const [vx, vy] = hexVertex(CX, CY, HEX_RADII[3], def.vertex);
          isoLines[idx].setAttribute('x1', String(CX)); isoLines[idx].setAttribute('y1', String(CY));
          isoLines[idx].setAttribute('x2', String(vx)); isoLines[idx].setAttribute('y2', String(vy));
          isoLines[idx].setAttribute('opacity', String(isoOp));
        });
      }

      rafRef.current = requestAnimationFrame(tick);
    }

    rafRef.current = requestAnimationFrame(tick);

    // === CLEANUP: cancel rAF + remove SVG children ===
    return () => {
      animatingRef.current = false;
      cancelAnimationFrame(rafRef.current);
      while (svg.firstChild) svg.removeChild(svg.firstChild);
    };
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

/**
 * Forge Design Tokens — single source of truth for JS contexts.
 * CSS equivalents: global.css @theme { --color-forge-* }
 * SVG attributes can use var(--color-forge-*) directly.
 */
export const COLORS = {
  bg: '#0D0D0D',
  fg: '#E8E8E8',
  line: '#3A3A3A',
  dim: '#949494',
  ember: '#FF6B35',
  surface: '#161616',
  green: '#28C840',
} as const;

/**
 * Polygon vertex calculator.
 * sides=6 → hexagon, sides=8 → octagon.
 */
export function polyVertex(
  cx: number, cy: number, r: number, i: number, sides: number
): [number, number] {
  const angle = (2 * Math.PI / sides) * i - Math.PI / 2;
  return [cx + r * Math.cos(angle), cy + r * Math.sin(angle)];
}

export function hexVertex(cx: number, cy: number, r: number, i: number): [number, number] {
  return polyVertex(cx, cy, r, i, 6);
}

export function octVertex(cx: number, cy: number, r: number, i: number): [number, number] {
  return polyVertex(cx, cy, r, i, 8);
}

export function octPoints(cx: number, cy: number, r: number): string {
  return Array.from({ length: 8 }, (_, i) => octVertex(cx, cy, r, i))
    .map(([x, y]) => `${x},${y}`).join(' ');
}

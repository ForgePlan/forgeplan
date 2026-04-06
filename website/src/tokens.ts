/**
 * Forge Design Tokens — CSS variable references for theme-aware rendering.
 * These resolve to actual colors via :root / .dark / .light in global.css.
 * Use in SVG attributes: stroke={COLORS.fg} → "var(--forge-fg)"
 */
export const COLORS = {
  bg: 'var(--forge-bg)',
  fg: 'var(--forge-fg)',
  line: 'var(--forge-line)',
  dim: 'var(--forge-dim)',
  surface: 'var(--forge-surface)',
  ember: '#FF6B35',  // static — doesn't change with theme
  green: '#28C840',  // static
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

import { epsilon } from '@/core/geometry/epsilon.js';

const EPS = function () {
  return epsilon.EPS;
};

/**
 * Intersection of two infinite lines defined by two segments.
 * Returns a single point, null (parallel), or null (coincident).
 * @returns {{x:number,y:number}|null}
 */
function lineLine(a, b) {
  const x1 = a.p1.x,
    y1 = a.p1.y,
    x2 = a.p2.x,
    y2 = a.p2.y;
  const x3 = b.p1.x,
    y3 = b.p1.y,
    x4 = b.p2.x,
    y4 = b.p2.y;
  const denom = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);
  if (Math.abs(denom) < EPS()) return null;
  const t = ((x1 - x3) * (y3 - y4) - (y1 - y3) * (x3 - x4)) / denom;
  return { x: x1 + t * (x2 - x1), y: y1 + t * (y2 - y1) };
}

/** @returns {Array<{x:number,y:number}>} (0..2) */
function lineCircle(line, circle) {
  const dx = line.p2.x - line.p1.x;
  const dy = line.p2.y - line.p1.y;
  const fx = line.p1.x - circle.center.x;
  const fy = line.p1.y - circle.center.y;
  const a = dx * dx + dy * dy;
  const b = 2 * (fx * dx + fy * dy);
  const c = fx * fx + fy * fy - circle.r * circle.r;
  let disc = b * b - 4 * a * c;
  if (disc < 0) return [];
  if (Math.abs(disc) < EPS()) {
    const t = -b / (2 * a);
    return [{ x: line.p1.x + t * dx, y: line.p1.y + t * dy }];
  }
  disc = Math.sqrt(disc);
  const t1 = (-b + disc) / (2 * a);
  const t2 = (-b - disc) / (2 * a);
  return [
    { x: line.p1.x + t1 * dx, y: line.p1.y + t1 * dy },
    { x: line.p1.x + t2 * dx, y: line.p1.y + t2 * dy },
  ];
}

/** @returns {Array<{x:number,y:number}>} (0..2) */
function circleCircle(a, b) {
  const dx = b.center.x - a.center.x;
  const dy = b.center.y - a.center.y;
  const d = Math.sqrt(dx * dx + dy * dy);
  if (d < EPS()) return [];
  if (d > a.r + b.r || d < Math.abs(a.r - b.r)) return [];
  const ax = (a.r * a.r - b.r * b.r + d * d) / (2 * d);
  const h2 = a.r * a.r - ax * ax;
  if (h2 < 0) return [];
  const h = Math.sqrt(h2);
  const cx = a.center.x + (ax * dx) / d;
  const cy = a.center.y + (ax * dy) / d;
  return [
    { x: cx + (h * dy) / d, y: cy - (h * dx) / d },
    { x: cx - (h * dy) / d, y: cy + (h * dx) / d },
  ];
}

export const intersect = { lineLine: lineLine, lineCircle: lineCircle, circleCircle: circleCircle };

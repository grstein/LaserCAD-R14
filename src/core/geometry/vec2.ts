import { epsilon } from '@/core/geometry/epsilon.js';
import type { Vec2 } from '@/core/types.js';

export const vec2 = {
  make(x: number, y: number): Vec2 {
    return { x, y };
  },

  add(a: Vec2, b: Vec2): Vec2 {
    return { x: a.x + b.x, y: a.y + b.y };
  },

  sub(a: Vec2, b: Vec2): Vec2 {
    return { x: a.x - b.x, y: a.y - b.y };
  },

  scale(a: Vec2, k: number): Vec2 {
    return { x: a.x * k, y: a.y * k };
  },

  dot(a: Vec2, b: Vec2): number {
    return a.x * b.x + a.y * b.y;
  },

  cross(a: Vec2, b: Vec2): number {
    return a.x * b.y - a.y * b.x;
  },

  lenSq(a: Vec2): number {
    return a.x * a.x + a.y * a.y;
  },

  len(a: Vec2): number {
    return Math.sqrt(a.x * a.x + a.y * a.y);
  },

  normalize(a: Vec2): Vec2 {
    const lsq = a.x * a.x + a.y * a.y;
    if (lsq <= epsilon.EPS * epsilon.EPS) return { x: 0, y: 0 };
    const k = 1 / Math.sqrt(lsq);
    return { x: a.x * k, y: a.y * k };
  },

  equals(a: Vec2, b: Vec2, tol?: number): boolean {
    return epsilon.eq(a.x, b.x, tol) && epsilon.eq(a.y, b.y, tol);
  },
};

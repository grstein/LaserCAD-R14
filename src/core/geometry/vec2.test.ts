import { describe, expect, it } from 'vitest';
import { vec2 } from './vec2.js';

describe('vec2', () => {
  it('add', () => {
    expect(vec2.add({ x: 1, y: 2 }, { x: 3, y: 4 })).toEqual({ x: 4, y: 6 });
  });

  it('sub', () => {
    expect(vec2.sub({ x: 5, y: 7 }, { x: 1, y: 2 })).toEqual({ x: 4, y: 5 });
  });

  it('len', () => {
    expect(vec2.len({ x: 3, y: 4 })).toBe(5);
  });

  it('normalize zero-vector returns zero', () => {
    expect(vec2.normalize({ x: 0, y: 0 })).toEqual({ x: 0, y: 0 });
  });

  it('normalize unit length', () => {
    const n = vec2.normalize({ x: 3, y: 4 });
    expect(vec2.len(n)).toBeCloseTo(1, 9);
  });

  it('cross', () => {
    expect(vec2.cross({ x: 1, y: 0 }, { x: 0, y: 1 })).toBe(1);
  });

  it('equals within tolerance', () => {
    expect(vec2.equals({ x: 1, y: 2 }, { x: 1 + 1e-12, y: 2 })).toBe(true);
  });
});

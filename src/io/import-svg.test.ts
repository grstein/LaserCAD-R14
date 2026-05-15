import { describe, expect, it } from 'vitest';
import { importSvg } from './import-svg.js';
import { exportSvg } from './export-svg.js';

function makeState(entities: unknown[], bounds = { w: 128, h: 128 }) {
  return {
    schemaVersion: 1,
    units: 'mm' as const,
    documentBounds: bounds,
    entities,
  };
}

describe('importSvg.parse', () => {
  it('imports a single <line>', () => {
    const svg = `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="80mm" viewBox="0 0 100 80">
  <g id="CUT" fill="none" stroke="#ff0000" stroke-width="0.5">
    <line x1="10" y1="20" x2="40" y2="50"/>
  </g>
</svg>`;
    const r = importSvg.parse(svg);
    expect(r.ok).toBe(true);
    if (!r.ok || !r.entities) return;
    expect(r.bounds).toEqual({ w: 100, h: 80 });
    expect(r.entities).toHaveLength(1);
    expect(r.entities[0]).toMatchObject({
      type: 'line',
      p1: { x: 10, y: 20 },
      p2: { x: 40, y: 50 },
    });
  });

  it('imports a <circle>', () => {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="50mm" height="50mm" viewBox="0 0 50 50">
      <circle cx="25" cy="25" r="10"/>
    </svg>`;
    const r = importSvg.parse(svg);
    expect(r.ok).toBe(true);
    if (!r.ok || !r.entities) return;
    expect(r.entities[0]).toMatchObject({ type: 'circle', center: { x: 25, y: 25 }, r: 10 });
  });

  it('imports an arc <path>', () => {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100">
      <path d="M 60 50 A 10 10 0 0 1 50 60"/>
    </svg>`;
    const r = importSvg.parse(svg);
    expect(r.ok).toBe(true);
    if (!r.ok || !r.entities) return;
    const a = r.entities[0] as { type: string; center: { x: number; y: number }; r: number; ccw: boolean };
    expect(a.type).toBe('arc');
    expect(a.center.x).toBeCloseTo(50, 6);
    expect(a.center.y).toBeCloseTo(50, 6);
    expect(a.r).toBeCloseTo(10, 6);
    expect(a.ccw).toBe(true);
  });

  it('roundtrips export → import for line/circle/arc', () => {
    const startState = makeState([
      { id: 'e_1', type: 'line', p1: { x: 5, y: 5 }, p2: { x: 95, y: 5 } },
      { id: 'e_2', type: 'circle', center: { x: 50, y: 50 }, r: 20 },
      {
        id: 'e_3',
        type: 'arc',
        center: { x: 30, y: 30 },
        r: 15,
        startAngle: 0,
        endAngle: Math.PI / 2,
        ccw: true,
      },
    ]);
    const svg = exportSvg.serialize(startState as never, { preset: 'cut' });
    const r = importSvg.parse(svg);
    expect(r.ok).toBe(true);
    if (!r.ok || !r.entities) return;
    expect(r.entities).toHaveLength(3);
    expect(r.entities[0]).toMatchObject({ type: 'line' });
    expect(r.entities[1]).toMatchObject({ type: 'circle', r: 20 });
    expect(r.entities[2].type).toBe('arc');
    const arc = r.entities[2] as { center: { x: number; y: number }; r: number; ccw: boolean };
    expect(arc.center.x).toBeCloseTo(30, 3);
    expect(arc.center.y).toBeCloseTo(30, 3);
    expect(arc.r).toBeCloseTo(15, 3);
    expect(arc.ccw).toBe(true);
  });

  it('rejects empty input', () => {
    const r = importSvg.parse('');
    expect(r.ok).toBe(false);
  });

  it('rejects missing width/height', () => {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"></svg>`;
    const r = importSvg.parse(svg);
    expect(r.ok).toBe(false);
    if (r.ok) return;
    expect(r.error).toBeTruthy();
    expect(r.error).toMatch(/width\/height/i);
  });

  it('rejects width/height without mm units gracefully accepts unitless numbers but matched viewBox', () => {
    // width without "mm" suffix is still treated as mm per LaserCAD convention
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100">
      <line x1="0" y1="0" x2="10" y2="10"/>
    </svg>`;
    const r = importSvg.parse(svg);
    expect(r.ok).toBe(true);
  });

  it('rejects unsupported element <rect>', () => {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm">
      <rect x="0" y="0" width="50" height="50"/>
    </svg>`;
    const r = importSvg.parse(svg);
    expect(r.ok).toBe(false);
    if (r.ok) return;
    expect(r.error).toBeTruthy();
    expect(r.error).toMatch(/rect/i);
  });

  it('rejects unsupported <text>', () => {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm">
      <text x="10" y="20">hello</text>
    </svg>`;
    expect(importSvg.parse(svg).ok).toBe(false);
  });

  it('rejects <polyline> (not in exported subset)', () => {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm">
      <polyline points="0,0 10,10 20,0"/>
    </svg>`;
    expect(importSvg.parse(svg).ok).toBe(false);
  });

  it('rejects non-arc path data', () => {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm">
      <path d="M 0 0 L 10 10 L 20 0 Z"/>
    </svg>`;
    const r = importSvg.parse(svg);
    expect(r.ok).toBe(false);
    if (r.ok) return;
    expect(r.error).toBeTruthy();
    expect(r.error).toMatch(/path/i);
  });

  it('rejects <line> with zero length', () => {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm">
      <line x1="5" y1="5" x2="5" y2="5"/>
    </svg>`;
    expect(importSvg.parse(svg).ok).toBe(false);
  });

  it('rejects <circle> with non-positive radius', () => {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm">
      <circle cx="10" cy="10" r="0"/>
    </svg>`;
    expect(importSvg.parse(svg).ok).toBe(false);
  });

  it('rejects element with non-identity transform', () => {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm">
      <g transform="rotate(45)">
        <line x1="0" y1="0" x2="10" y2="10"/>
      </g>
    </svg>`;
    const r = importSvg.parse(svg);
    expect(r.ok).toBe(false);
    if (r.ok) return;
    expect(r.error).toBeTruthy();
    expect(r.error).toMatch(/transform/i);
  });

  it('rejects mismatched viewBox vs width/height', () => {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 200 200">
      <line x1="0" y1="0" x2="10" y2="10"/>
    </svg>`;
    expect(importSvg.parse(svg).ok).toBe(false);
  });

  it('rejects viewBox with non-zero offset', () => {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="10 10 100 100">
      <line x1="0" y1="0" x2="10" y2="10"/>
    </svg>`;
    expect(importSvg.parse(svg).ok).toBe(false);
  });
});

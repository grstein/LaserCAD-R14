/**
 * Imports the SVG subset emitted by LaserCAD R14 (`src/io/export-svg.ts`)
 * back into editable entities. Anything outside the subset is rejected.
 *
 * Supported subset:
 *   - root <svg> with `width`/`height` in mm and optional `viewBox="0 0 W H"`
 *   - plain <g> (container only; no `transform`)
 *   - <line x1 y1 x2 y2/>
 *   - <circle cx cy r/>
 *   - <path d="M sx sy A r r 0 large sweep ex ey"/>  (single arc)
 *
 * Everything else (text, image, mask, filter, clipPath, polyline, polygon, rect,
 * ellipse, use, style, defs, symbol, transform≠identity, nested transforms…)
 * is rejected with a message and the current document is left untouched.
 */

import type { Entity } from '@/core/types.js';

const SVG_NS = 'http://www.w3.org/2000/svg';
const ALLOWED_TAGS = new Set(['svg', 'g', 'line', 'circle', 'path']);

export interface ImportResult {
  ok: boolean;
  bounds?: { w: number; h: number };
  entities?: Entity[];
  error?: string;
}

function parseMm(raw: string | null): number | null {
  if (!raw) return null;
  const m = /^\s*(-?\d+(?:\.\d+)?)\s*(mm)?\s*$/i.exec(raw);
  if (!m) return null;
  const n = parseFloat(m[1]);
  if (!isFinite(n) || n <= 0) return null;
  return n;
}

function transformError(el: Element): string | null {
  const t = el.getAttribute('transform');
  if (!t) return null;
  const trimmed = t.trim();
  if (trimmed === '') return null;
  if (/^matrix\(\s*1[\s,]+0[\s,]+0[\s,]+1[\s,]+0[\s,]+0\s*\)$/.test(trimmed)) return null;
  if (/^translate\(\s*0[\s,]*0?\s*\)$/.test(trimmed)) return null;
  return `Unsupported transform on <${el.localName}>: "${trimmed}"`;
}

interface ParsedArcPath {
  sx: number;
  sy: number;
  ex: number;
  ey: number;
  r: number;
  large: 0 | 1;
  sweep: 0 | 1;
}

function parseArcPath(d: string): ParsedArcPath | null {
  const tokens = d.trim().split(/[\s,]+/).filter(Boolean);
  if (tokens.length !== 11) return null;
  if (tokens[0] !== 'M') return null;
  if (tokens[3] !== 'A') return null;
  const nums = tokens
    .filter((_, i) => i !== 0 && i !== 3)
    .map((t) => parseFloat(t));
  if (nums.some((n) => !isFinite(n))) return null;
  const [sx, sy, rx, ry, phi, large, sweep, ex, ey] = nums;
  if (!(rx > 0) || Math.abs(rx - ry) > 1e-6) return null;
  if (Math.abs(phi) > 1e-6) return null;
  if (large !== 0 && large !== 1) return null;
  if (sweep !== 0 && sweep !== 1) return null;
  return { sx, sy, ex, ey, r: rx, large: large as 0 | 1, sweep: sweep as 0 | 1 };
}

function arcFromPath(p: ParsedArcPath): {
  type: 'arc';
  center: { x: number; y: number };
  r: number;
  startAngle: number;
  endAngle: number;
  ccw: boolean;
} | null {
  const { sx, sy, ex, ey, r, large, sweep } = p;
  const x1 = (sx - ex) / 2;
  const y1 = (sy - ey) / 2;
  const denom = x1 * x1 + y1 * y1;
  if (denom < 1e-12) return null;
  let sq = (r * r - denom) / denom;
  if (sq < 0) {
    if (sq > -1e-6) sq = 0;
    else return null;
  }
  const factor = Math.sqrt(sq);
  const sign = large === sweep ? -1 : 1;
  const cxp = sign * factor * y1;
  const cyp = sign * factor * -x1;
  const cx = cxp + (sx + ex) / 2;
  const cy = cyp + (sy + ey) / 2;
  if (!isFinite(cx) || !isFinite(cy)) return null;
  const startAngle = Math.atan2(sy - cy, sx - cx);
  const endAngle = Math.atan2(ey - cy, ex - cx);
  return {
    type: 'arc',
    center: { x: cx, y: cy },
    r,
    startAngle,
    endAngle,
    ccw: sweep === 1,
  };
}

function isSvgElement(el: Element): boolean {
  return el.namespaceURI === SVG_NS || el.namespaceURI == null;
}

function parse(svgText: string): ImportResult {
  if (typeof svgText !== 'string' || svgText.trim() === '') {
    return { ok: false, error: 'Empty SVG content' };
  }
  let doc: XMLDocument;
  try {
    doc = new DOMParser().parseFromString(svgText, 'image/svg+xml');
  } catch {
    return { ok: false, error: 'Invalid SVG: parse failed' };
  }
  const perr = doc.querySelector('parsererror');
  if (perr) return { ok: false, error: 'Invalid SVG: XML parse error' };

  const root = doc.documentElement;
  if (!root || root.localName !== 'svg' || !isSvgElement(root)) {
    return { ok: false, error: 'Root element is not <svg>' };
  }
  if (transformError(root)) return { ok: false, error: transformError(root)! };

  const w = parseMm(root.getAttribute('width'));
  const h = parseMm(root.getAttribute('height'));
  if (w == null || h == null) {
    return { ok: false, error: 'Missing or invalid width/height in mm on <svg>' };
  }

  const vb = root.getAttribute('viewBox');
  if (vb && vb.trim() !== '') {
    const parts = vb.trim().split(/[\s,]+/).map(Number);
    if (parts.length !== 4 || parts.some((n) => !isFinite(n))) {
      return { ok: false, error: 'Invalid viewBox' };
    }
    if (Math.abs(parts[0]) > 1e-6 || Math.abs(parts[1]) > 1e-6) {
      return { ok: false, error: 'Unsupported viewBox: must start at 0 0' };
    }
    if (Math.abs(parts[2] - w) > 0.01 || Math.abs(parts[3] - h) > 0.01) {
      return { ok: false, error: 'viewBox does not match width/height (mm)' };
    }
  }

  const entities: Entity[] = [];
  let counter = 0;
  const nextId = (): string => {
    counter += 1;
    return 'e_' + counter;
  };

  function walk(parent: Element): string | null {
    const children = Array.from(parent.children);
    for (const child of children) {
      if (!isSvgElement(child)) {
        return `Unsupported foreign element: <${child.tagName}>`;
      }
      const name = child.localName;
      if (!ALLOWED_TAGS.has(name)) {
        return `Unsupported SVG element: <${name}>`;
      }
      const terr = transformError(child);
      if (terr) return terr;

      if (name === 'g') {
        const sub = walk(child);
        if (sub) return sub;
      } else if (name === 'line') {
        const x1 = parseFloat(child.getAttribute('x1') ?? '');
        const y1 = parseFloat(child.getAttribute('y1') ?? '');
        const x2 = parseFloat(child.getAttribute('x2') ?? '');
        const y2 = parseFloat(child.getAttribute('y2') ?? '');
        if (![x1, y1, x2, y2].every(isFinite)) {
          return 'Invalid <line>: non-numeric coordinates';
        }
        if (Math.abs(x1 - x2) < 1e-9 && Math.abs(y1 - y2) < 1e-9) {
          return 'Invalid <line>: zero-length segment';
        }
        entities.push({
          id: nextId(),
          type: 'line',
          p1: { x: x1, y: y1 },
          p2: { x: x2, y: y2 },
        });
      } else if (name === 'circle') {
        const cx = parseFloat(child.getAttribute('cx') ?? '');
        const cy = parseFloat(child.getAttribute('cy') ?? '');
        const r = parseFloat(child.getAttribute('r') ?? '');
        if (![cx, cy, r].every(isFinite) || !(r > 0)) {
          return 'Invalid <circle>';
        }
        entities.push({ id: nextId(), type: 'circle', center: { x: cx, y: cy }, r });
      } else if (name === 'path') {
        const d = child.getAttribute('d') ?? '';
        const parsed = parseArcPath(d);
        if (!parsed) {
          return 'Unsupported <path>: only single-arc paths "M x y A r r 0 f f x y" are supported';
        }
        const a = arcFromPath(parsed);
        if (!a) return 'Invalid arc geometry in <path>';
        entities.push({ id: nextId(), ...a });
      }
    }
    return null;
  }

  const err = walk(root);
  if (err) return { ok: false, error: err };

  return { ok: true, bounds: { w, h }, entities };
}

export const importSvg = { parse };

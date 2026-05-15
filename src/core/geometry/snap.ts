import { line } from '@/core/geometry/line.js';
import { arc } from '@/core/geometry/arc.js';
import { intersect } from '@/core/geometry/intersect.js';

function dist(a, b) {
  const dx = a.x - b.x,
    dy = a.y - b.y;
  return Math.sqrt(dx * dx + dy * dy);
}

/**
 * Coleta candidatos de snap das entities (e dos cantos/midpoints/centro do bed, se fornecido).
 * @param {Array} entities
 * @param {{w:number,h:number}} [bedBounds]
 * @returns {Array<{type:string, point:{x:number,y:number}}>}
 */
function collectCandidates(entities, bedBounds) {
  const out = [];
  for (let i = 0; i < entities.length; i++) {
    const e = entities[i];
    if (e.type === 'line') {
      out.push({ type: 'endpoint', point: { x: e.p1.x, y: e.p1.y } });
      out.push({ type: 'endpoint', point: { x: e.p2.x, y: e.p2.y } });
      out.push({ type: 'midpoint', point: line.midpoint(e) });
    } else if (e.type === 'circle') {
      out.push({ type: 'center', point: { x: e.center.x, y: e.center.y } });
    } else if (e.type === 'arc') {
      const ends = arc.endpoints(e);
      out.push({ type: 'endpoint', point: ends.start });
      out.push({ type: 'endpoint', point: ends.end });
      out.push({ type: 'center', point: { x: e.center.x, y: e.center.y } });
    }
  }
  if (bedBounds && bedBounds.w > 0 && bedBounds.h > 0) {
    const w = bedBounds.w,
      h = bedBounds.h;
    out.push({ type: 'endpoint', point: { x: 0, y: 0 } });
    out.push({ type: 'endpoint', point: { x: w, y: 0 } });
    out.push({ type: 'endpoint', point: { x: 0, y: h } });
    out.push({ type: 'endpoint', point: { x: w, y: h } });
    out.push({ type: 'midpoint', point: { x: w / 2, y: 0 } });
    out.push({ type: 'midpoint', point: { x: w / 2, y: h } });
    out.push({ type: 'midpoint', point: { x: 0, y: h / 2 } });
    out.push({ type: 'midpoint', point: { x: w, y: h / 2 } });
    out.push({ type: 'center', point: { x: w / 2, y: h / 2 } });
  }
  return out;
}

/**
 * Encontra interseções entre pares de entities (line×line, line×circle, circle×circle).
 * Limitado para performance: O(n²) com cap em 50 entities.
 */
function collectIntersections(entities) {
  const out = [];
  const n = Math.min(entities.length, 50);
  for (let i = 0; i < n; i++) {
    for (let j = i + 1; j < n; j++) {
      const a = entities[i],
        b = entities[j];
      let pts = null;
      if (a.type === 'line' && b.type === 'line') {
        const p = intersect.lineLine(a, b);
        if (p) pts = [p];
      } else if (a.type === 'line' && b.type === 'circle') pts = intersect.lineCircle(a, b);
      else if (a.type === 'circle' && b.type === 'line') pts = intersect.lineCircle(b, a);
      else if (a.type === 'circle' && b.type === 'circle') pts = intersect.circleCircle(a, b);
      if (pts)
        pts.forEach(function (p) {
          out.push({ type: 'intersection', point: p });
        });
    }
  }
  return out;
}

/**
 * @param {{entities:Array}} state
 * @param {{x:number,y:number}} worldPt
 * @param {number} tolMm tolerância em mm (= SNAP_TOLERANCE_PX / zoom)
 * @returns {{type:string, point:{x:number,y:number}}|null}
 */
function findBest(state, worldPt, tolMm) {
  if (!state.toggles.snap) return null;
  const candidates = collectCandidates(state.entities, state.documentBounds).concat(
    collectIntersections(state.entities),
  );
  // Prioridade: intersection > endpoint > midpoint > center
  const priority = { intersection: 0, endpoint: 1, midpoint: 2, center: 3 };
  let best = null;
  let bestD = tolMm;
  for (let i = 0; i < candidates.length; i++) {
    const c = candidates[i];
    const d = dist(c.point, worldPt);
    if (d <= bestD) {
      if (
        !best ||
        priority[c.type] < priority[best.type] ||
        (priority[c.type] === priority[best.type] && d < bestD)
      ) {
        best = c;
        bestD = d;
      }
    }
  }
  return best;
}

export const snap = { findBest: findBest, collectCandidates: collectCandidates };

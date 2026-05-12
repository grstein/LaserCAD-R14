(function (LaserCAD) {
  'use strict';

  const ns = LaserCAD.core.geometry;

  function dist(a, b) { const dx = a.x - b.x, dy = a.y - b.y; return Math.sqrt(dx*dx + dy*dy); }

  /**
   * Coleta candidatos de snap das entities.
   * @param {Array} entities
   * @returns {Array<{type:string, point:{x:number,y:number}}>}
   */
  function collectCandidates(entities) {
    const out = [];
    for (let i = 0; i < entities.length; i++) {
      const e = entities[i];
      if (e.type === 'line') {
        out.push({ type: 'endpoint', point: { x: e.p1.x, y: e.p1.y } });
        out.push({ type: 'endpoint', point: { x: e.p2.x, y: e.p2.y } });
        out.push({ type: 'midpoint', point: ns.line.midpoint(e) });
      } else if (e.type === 'circle') {
        out.push({ type: 'center', point: { x: e.center.x, y: e.center.y } });
      } else if (e.type === 'arc') {
        const ends = ns.arc.endpoints(e);
        out.push({ type: 'endpoint', point: ends.start });
        out.push({ type: 'endpoint', point: ends.end });
        out.push({ type: 'center', point: { x: e.center.x, y: e.center.y } });
      }
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
        const a = entities[i], b = entities[j];
        let pts = null;
        if (a.type === 'line'   && b.type === 'line')   { const p = ns.intersect.lineLine(a, b);  if (p) pts = [p]; }
        else if (a.type === 'line'   && b.type === 'circle') pts = ns.intersect.lineCircle(a, b);
        else if (a.type === 'circle' && b.type === 'line')   pts = ns.intersect.lineCircle(b, a);
        else if (a.type === 'circle' && b.type === 'circle') pts = ns.intersect.circleCircle(a, b);
        if (pts) pts.forEach(function (p) { out.push({ type: 'intersection', point: p }); });
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
    const candidates = collectCandidates(state.entities).concat(collectIntersections(state.entities));
    // Prioridade: intersection > endpoint > midpoint > center
    const priority = { intersection: 0, endpoint: 1, midpoint: 2, center: 3 };
    let best = null;
    let bestD = tolMm;
    for (let i = 0; i < candidates.length; i++) {
      const c = candidates[i];
      const d = dist(c.point, worldPt);
      if (d <= bestD) {
        if (!best || priority[c.type] < priority[best.type] || (priority[c.type] === priority[best.type] && d < bestD)) {
          best = c;
          bestD = d;
        }
      }
    }
    return best;
  }

  ns.snap = { findBest: findBest, collectCandidates: collectCandidates };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

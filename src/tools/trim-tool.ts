import { bus } from '@/app/event-bus.js';
import { commands } from '@/core/document/commands.js';
import { arc } from '@/core/geometry/arc.js';
import { intersect } from '@/core/geometry/intersect.js';
import { camera } from '@/render/camera.js';
import { toolManager } from '@/tools/tool-manager.js';

const TWO_PI = Math.PI * 2;
const PARAM_EPS = 1e-6;
const DEDUP_EPS = 1e-6;

function distPointSegment(p, a, b) {
  const dx = b.x - a.x,
    dy = b.y - a.y;
  const t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / Math.max(dx * dx + dy * dy, 1e-12);
  const tc = Math.max(0, Math.min(1, t));
  const cx = a.x + tc * dx,
    cy = a.y + tc * dy;
  return Math.sqrt((p.x - cx) * (p.x - cx) + (p.y - cy) * (p.y - cy));
}

function hitEntity(p, entities, tol) {
  for (let i = entities.length - 1; i >= 0; i--) {
    const e = entities[i];
    if (e.type === 'line') {
      if (distPointSegment(p, e.p1, e.p2) <= tol) return e;
    } else if (e.type === 'circle') {
      const dx = p.x - e.center.x,
        dy = p.y - e.center.y;
      if (Math.abs(Math.sqrt(dx * dx + dy * dy) - e.r) <= tol) return e;
    } else if (e.type === 'arc') {
      const dx = p.x - e.center.x,
        dy = p.y - e.center.y;
      const d = Math.sqrt(dx * dx + dy * dy);
      if (Math.abs(d - e.r) > tol) continue;
      const theta = Math.atan2(dy, dx);
      if (arc.containsAngle(e, theta)) return e;
    }
  }
  return null;
}

function paramOnLine(line, p) {
  const dx = line.p2.x - line.p1.x,
    dy = line.p2.y - line.p1.y;
  const len2 = dx * dx + dy * dy;
  if (len2 < 1e-18) return 0;
  return ((p.x - line.p1.x) * dx + (p.y - line.p1.y) * dy) / len2;
}

function pointAtLineParam(line, t) {
  return {
    x: line.p1.x + t * (line.p2.x - line.p1.x),
    y: line.p1.y + t * (line.p2.y - line.p1.y),
  };
}

function angleOf(center, p) {
  const a = Math.atan2(p.y - center.y, p.x - center.x);
  return a < 0 ? a + TWO_PI : a;
}

function withinLine(line, p) {
  const t = paramOnLine(line, p);
  return t >= -PARAM_EPS && t <= 1 + PARAM_EPS;
}

function pointOnEntityDomain(entity, p) {
  if (entity.type === 'line') return withinLine(entity, p);
  if (entity.type === 'circle') return true;
  if (entity.type === 'arc')
    return arc.containsAngle(entity, Math.atan2(p.y - entity.center.y, p.x - entity.center.x));
  return false;
}

/**
 * Lists the points where `target` is cut by other entities (filtered to the domain of both).
 * @returns {Array<{x:number,y:number}>}
 */
function intersectionsOnTarget(target, entities) {
  const targetIsCircle = target.type === 'circle';
  const targetAsCircle = targetIsCircle ? target : null;
  const out = [];

  for (let i = 0; i < entities.length; i++) {
    const other = entities[i];
    if (other === target || other.id === target.id) continue;
    let pts = [];

    if (target.type === 'line' && other.type === 'line') {
      const p = intersect.lineLine(target, other);
      if (p) pts = [p];
    } else if (target.type === 'line' && other.type === 'circle') {
      pts = intersect.lineCircle(target, other);
    } else if (target.type === 'line' && other.type === 'arc') {
      const asCircle = { type: 'circle', center: other.center, r: other.r };
      pts = intersect.lineCircle(target, asCircle);
    } else if (targetIsCircle && other.type === 'line') {
      pts = intersect.lineCircle(other, targetAsCircle);
    } else if (targetIsCircle && other.type === 'circle') {
      pts = intersect.circleCircle(target, other);
    } else if (targetIsCircle && other.type === 'arc') {
      const asCircle = { type: 'circle', center: other.center, r: other.r };
      pts = intersect.circleCircle(target, asCircle);
    }

    for (let k = 0; k < pts.length; k++) {
      const p = pts[k];
      if (!pointOnEntityDomain(target, p)) continue;
      if (!pointOnEntityDomain(other, p)) continue;
      out.push(p);
    }
  }
  return out;
}

function dedupByDist(arr, key, eps) {
  arr.sort(function (a, b) {
    return key(a) - key(b);
  });
  const out = [];
  for (let i = 0; i < arr.length; i++) {
    if (out.length === 0 || Math.abs(key(arr[i]) - key(out[out.length - 1])) > eps) {
      out.push(arr[i]);
    }
  }
  return out;
}

/**
 * Splits `line` by the intersections and returns the list of segments to keep
 * (discards the one containing `clickPt`). Returns null if there are no intersections.
 */
function splitLine(line, intersections, clickPt) {
  const tClick = paramOnLine(line, clickPt);
  const sorted = dedupByDist(
    intersections
      .map(function (p) {
        return { t: paramOnLine(line, p), p: p };
      })
      .filter(function (h) {
        return h.t > PARAM_EPS && h.t < 1 - PARAM_EPS;
      }),
    function (h) {
      return h.t;
    },
    DEDUP_EPS,
  );
  if (sorted.length === 0) return null;

  // tLeft = largest ti < tClick (or 0 if none)
  // tRight = smallest ti > tClick (or 1 if none)
  let tLeft = 0,
    pLeft = { x: line.p1.x, y: line.p1.y };
  let tRight = 1,
    pRight = { x: line.p2.x, y: line.p2.y };
  for (let i = 0; i < sorted.length; i++) {
    if (sorted[i].t < tClick) {
      tLeft = sorted[i].t;
      pLeft = sorted[i].p;
    } else if (tRight === 1) {
      tRight = sorted[i].t;
      pRight = sorted[i].p;
      break;
    }
  }

  const survivors = [];
  if (tLeft > PARAM_EPS) {
    survivors.push({
      type: 'line',
      p1: { x: line.p1.x, y: line.p1.y },
      p2: { x: pLeft.x, y: pLeft.y },
    });
  }
  if (tRight < 1 - PARAM_EPS) {
    survivors.push({
      type: 'line',
      p1: { x: pRight.x, y: pRight.y },
      p2: { x: line.p2.x, y: line.p2.y },
    });
  }
  return survivors;
}

/**
 * Splits `circle` by the intersections into arcs; discards the one containing the click angle.
 * Returns null if there are fewer than 2 intersections (cannot split).
 */
function splitCircle(circle, intersections, clickPt) {
  const sorted = dedupByDist(
    intersections.map(function (p) {
      return { theta: angleOf(circle.center, p), p: p };
    }),
    function (h) {
      return h.theta;
    },
    DEDUP_EPS,
  );
  if (sorted.length < 2) return null;

  const thetaClick = angleOf(circle.center, clickPt);
  const arcs = [];
  for (let i = 0; i < sorted.length; i++) {
    const a = sorted[i].theta;
    const b = sorted[(i + 1) % sorted.length].theta;
    const end = b > a ? b : b + TWO_PI;
    arcs.push({ start: a, end: end });
  }
  const survivors = [];
  for (let i = 0; i < arcs.length; i++) {
    const candidate = arc.make(circle.center, circle.r, arcs[i].start, arcs[i].end, true);
    if (arc.containsAngle(candidate, thetaClick)) continue;
    survivors.push(candidate);
  }
  return survivors;
}

export const trimTool = {
  id: 'trim',
  prompt: 'TRIM  Click segment to remove:',
  onArm(state) {
    state.setToolState('armed');
  },
  onCancel() {
    /* nothing */
  },
  onPointerDown(e, state) {
    const p = { x: e.worldX, y: e.worldY };
    const pxPerMm = camera.pxPerMm(state.camera);
    const tol = 6 / Math.max(pxPerMm, 0.0001);
    const hit = hitEntity(p, state.entities, tol);
    if (!hit) return;

    if (hit.type === 'arc') {
      bus.emit('command:error', { raw: 'trim', message: '! Trim on arc not implemented yet' });
      return;
    }

    const ips = intersectionsOnTarget(hit, state.entities);
    let survivors = null;
    if (hit.type === 'line') survivors = splitLine(hit, ips, p);
    else if (hit.type === 'circle') survivors = splitCircle(hit, ips, p);

    if (!survivors) {
      bus.emit('command:error', { raw: 'trim', message: '! No intersection nearby' });
      return;
    }

    toolManager.commit(commands.removeEntity(hit.id));
    for (let i = 0; i < survivors.length; i++) {
      toolManager.commit(commands.addEntity(survivors[i]));
    }
  },
};

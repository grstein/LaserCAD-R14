import { bus } from '@/app/event-bus.js';
import { commands } from '@/core/document/commands.js';
import { intersect } from '@/core/geometry/intersect.js';
import { camera } from '@/render/camera.js';
import { toolManager } from '@/tools/tool-manager.js';

let boundary = null;

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
    }
  }
  return null;
}

function extendLineToLine(target, edge, clickPt) {
  const ip = intersect.lineLine(target, edge);
  if (!ip) return null;
  const dP1 = Math.hypot(clickPt.x - target.p1.x, clickPt.y - target.p1.y);
  const dP2 = Math.hypot(clickPt.x - target.p2.x, clickPt.y - target.p2.y);
  if (dP1 < dP2) return { type: 'line', p1: { x: ip.x, y: ip.y }, p2: target.p2 };
  return { type: 'line', p1: target.p1, p2: { x: ip.x, y: ip.y } };
}

function extendLineToCircle(target, edge, clickPt) {
  const ips = intersect.lineCircle(target, edge);
  if (!ips || ips.length === 0) return null;
  // ponto de extensão mais próximo da extremidade clicada
  const dP1 = Math.hypot(clickPt.x - target.p1.x, clickPt.y - target.p1.y);
  const dP2 = Math.hypot(clickPt.x - target.p2.x, clickPt.y - target.p2.y);
  const nearEnd = dP1 < dP2 ? target.p1 : target.p2;
  let chosen = ips[0],
    best = Infinity;
  for (let i = 0; i < ips.length; i++) {
    const d = Math.hypot(ips[i].x - nearEnd.x, ips[i].y - nearEnd.y);
    if (d < best) {
      best = d;
      chosen = ips[i];
    }
  }
  if (dP1 < dP2) return { type: 'line', p1: { x: chosen.x, y: chosen.y }, p2: target.p2 };
  return { type: 'line', p1: target.p1, p2: { x: chosen.x, y: chosen.y } };
}

export const extendTool = {
  id: 'extend',
  prompt: 'EXTEND  Select boundary:',
  onArm(state) {
    boundary = null;
    state.setToolState('armed');
  },
  onCancel() {
    boundary = null;
  },
  onPointerDown(e, state) {
    const p = { x: e.worldX, y: e.worldY };
    const pxPerMm = camera.pxPerMm(state.camera);
    const tol = 6 / Math.max(pxPerMm, 0.0001);
    const hit = hitEntity(p, state.entities, tol);
    if (!hit) return;
    if (!boundary) {
      boundary = hit;
      return;
    }
    if (hit.id === boundary.id) return;
    let newGeom = null;
    if (hit.type === 'line' && boundary.type === 'line')
      newGeom = extendLineToLine(hit, boundary, p);
    else if (hit.type === 'line' && boundary.type === 'circle')
      newGeom = extendLineToCircle(hit, boundary, p);
    if (!newGeom) {
      bus.emit('command:error', { raw: 'extend', message: '! No intersection found' });
      return;
    }
    toolManager.commit(commands.removeEntity(hit.id));
    toolManager.commit(commands.addEntity(newGeom));
  },
};

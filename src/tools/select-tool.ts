import { commands } from '@/core/document/commands.js';
import { line } from '@/core/geometry/line.js';
import { circle } from '@/core/geometry/circle.js';
import { arc } from '@/core/geometry/arc.js';
import { camera } from '@/render/camera.js';
import { entityRenderers } from '@/render/entity-renderers.js';
import { toolManager } from '@/tools/tool-manager.js';

let boxStart = null;
let boxRect = null;

function distPointSegment(p, a, b) {
  const dx = b.x - a.x,
    dy = b.y - a.y;
  const t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / Math.max(dx * dx + dy * dy, 1e-12);
  const tc = Math.max(0, Math.min(1, t));
  const cx = a.x + tc * dx,
    cy = a.y + tc * dy;
  return Math.sqrt((p.x - cx) * (p.x - cx) + (p.y - cy) * (p.y - cy));
}
function hitTest(p, entity, tol) {
  if (entity.type === 'line') return distPointSegment(p, entity.p1, entity.p2) <= tol;
  if (entity.type === 'circle') {
    const dx = p.x - entity.center.x,
      dy = p.y - entity.center.y;
    return Math.abs(Math.sqrt(dx * dx + dy * dy) - entity.r) <= tol;
  }
  if (entity.type === 'arc') {
    const dx = p.x - entity.center.x,
      dy = p.y - entity.center.y;
    const d = Math.sqrt(dx * dx + dy * dy);
    if (Math.abs(d - entity.r) > tol) return false;
    const ang = Math.atan2(dy, dx);
    return arc.containsAngle(entity, ang);
  }
  return false;
}
function entityInBox(entity, minX, minY, maxX, maxY) {
  let bb;
  if (entity.type === 'line') bb = line.bbox(entity);
  else if (entity.type === 'circle') bb = circle.bbox(entity);
  else if (entity.type === 'arc') bb = arc.bbox(entity);
  if (!bb) return false;
  return bb.minX >= minX && bb.minY >= minY && bb.maxX <= maxX && bb.maxY <= maxY;
}

function renderBox() {
  if (!boxRect) return;
  const sr = toolManager.getSvgRoot();
  if (!sr) return;
  const layer = sr.getLayer('preview');
  while (layer.firstChild) layer.removeChild(layer.firstChild);
  const r = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
  const x = Math.min(boxRect.x1, boxRect.x2);
  const y = Math.min(boxRect.y1, boxRect.y2);
  const w = Math.abs(boxRect.x2 - boxRect.x1);
  const h = Math.abs(boxRect.y2 - boxRect.y1);
  r.setAttribute('x', String(x));
  r.setAttribute('y', String(y));
  r.setAttribute('width', String(w));
  r.setAttribute('height', String(h));
  const c = getComputedStyle(document.documentElement).getPropertyValue('--laser-450').trim();
  r.setAttribute('stroke', c);
  r.setAttribute('stroke-width', '0.1');
  r.setAttribute('fill', c);
  r.setAttribute('fill-opacity', '0.08');
  r.setAttribute('vector-effect', 'non-scaling-stroke');
  layer.appendChild(r);
}

export const selectTool = {
  id: 'select',
  prompt: 'Select objects:',
  onArm(state) {
    boxStart = null;
    boxRect = null;
    state.setToolState('armed');
  },
  onCancel() {
    boxStart = null;
    boxRect = null;
    const sr = toolManager.getSvgRoot();
    if (sr) entityRenderers.clearPreview(sr);
  },
  onPointerDown(e, state) {
    const p = { x: e.worldX, y: e.worldY };
    // tenta hit-test direto
    const pxPerMm = camera.pxPerMm(state.camera);
    const tol = 6 / Math.max(pxPerMm, 0.0001);
    for (let i = state.entities.length - 1; i >= 0; i--) {
      const ent = state.entities[i];
      if (hitTest(p, ent, tol)) {
        const sel = e.shiftKey ? state.selection.concat([ent.id]) : [ent.id];
        toolManager.commit(commands.setSelection(sel));
        return;
      }
    }
    // Início de drag-to-select
    boxStart = p;
    boxRect = { x1: p.x, y1: p.y, x2: p.x, y2: p.y };
    state.setToolState('preview');
  },
  onPointerMove(e) {
    if (boxStart && boxRect) {
      boxRect.x2 = e.worldX;
      boxRect.y2 = e.worldY;
      renderBox();
    }
  },
  onPointerUp(e, state) {
    if (!boxStart || !boxRect) return;
    const minX = Math.min(boxRect.x1, boxRect.x2),
      minY = Math.min(boxRect.y1, boxRect.y2);
    const maxX = Math.max(boxRect.x1, boxRect.x2),
      maxY = Math.max(boxRect.y1, boxRect.y2);
    // Drag pequeno (clique sem arrasto): limpa seleção
    const tinyMm = 5 / Math.max(camera.pxPerMm(state.camera), 0.0001);
    if (Math.abs(maxX - minX) < tinyMm && Math.abs(maxY - minY) < tinyMm) {
      toolManager.commit(commands.setSelection([]));
    } else {
      const picked = state.entities
        .filter(function (ent) {
          return entityInBox(ent, minX, minY, maxX, maxY);
        })
        .map(function (ent) {
          return ent.id;
        });
      const sel = e.shiftKey ? state.selection.concat(picked) : picked;
      toolManager.commit(commands.setSelection(sel));
    }
    boxStart = null;
    boxRect = null;
    const sr = toolManager.getSvgRoot();
    if (sr) entityRenderers.clearPreview(sr);
    state.setToolState('armed');
  },
};

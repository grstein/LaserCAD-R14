import { commands } from '@/core/document/commands.js';
import { entityRenderers } from '@/render/entity-renderers.js';
import { toolManager } from '@/tools/tool-manager.js';

let points = [];

function orthoSnap(p1, p2) {
  const dx = Math.abs(p2.x - p1.x),
    dy = Math.abs(p2.y - p1.y);
  return dx >= dy ? { x: p2.x, y: p1.y } : { x: p1.x, y: p2.y };
}

function preview(p, applyOrtho) {
  if (points.length === 0) return;
  const sr = toolManager.getSvgRoot();
  if (!sr) return;
  const layer = sr.getLayer('preview');
  while (layer.firstChild) layer.removeChild(layer.firstChild);
  const er = entityRenderers;
  const glow = getComputedStyle(document.documentElement).getPropertyValue('--laser-glow').trim();
  // segmentos comitados (preview até o cursor) e segmento ativo
  for (let i = 0; i < points.length - 1; i++) {
    const n = er.renderLine({ p1: points[i], p2: points[i + 1] }, layer);
    n.setAttribute('stroke-dasharray', '4 2');
    n.setAttribute('stroke', glow);
  }
  const last = points[points.length - 1];
  const end = applyOrtho ? orthoSnap(last, p) : p;
  const n = er.renderLine({ p1: last, p2: end }, layer);
  n.setAttribute('stroke-dasharray', '4 2');
  n.setAttribute('stroke', glow);
}

function commitAll() {
  for (let i = 0; i < points.length - 1; i++) {
    toolManager.commit(commands.addEntity({ type: 'line', p1: points[i], p2: points[i + 1] }));
  }
  points = [];
}

export const polylineTool = {
  id: 'polyline',
  prompt: 'POLYLINE  Specify next point or Esc to finish:',
  onArm(state) {
    points = [];
    state.setToolState('armed');
  },
  onCancel() {
    if (points.length >= 2) commitAll();
    points = [];
    const sr = toolManager.getSvgRoot();
    if (sr) entityRenderers.clearPreview(sr);
  },
  onPointerMove(e, state) {
    const ortho = e.shiftKey || state.toggles.ortho;
    preview({ x: e.worldX, y: e.worldY }, ortho);
  },
  onPointerDown(e, state) {
    const ortho = e.shiftKey || state.toggles.ortho;
    let p = { x: e.worldX, y: e.worldY };
    if (points.length > 0 && ortho) p = orthoSnap(points[points.length - 1], p);
    points.push(p);
    state.setToolState('preview');
  },
  onCommand(payload) {
    const p = payload.parsed;
    if (!p) return;
    if (p.kind === 'absolute') {
      points.push({ x: p.x, y: p.y });
    } else if (p.kind === 'relative' && points.length > 0) {
      const last = points[points.length - 1];
      points.push({ x: last.x + p.dx, y: last.y + p.dy });
    }
  },
};

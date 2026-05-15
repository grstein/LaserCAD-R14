import { commands } from '@/core/document/commands.js';
import { entityRenderers } from '@/render/entity-renderers.js';
import { toolManager } from '@/tools/tool-manager.js';

let firstPoint = null;

function rectAsLines(p1, p2) {
  return [
    { type: 'line', p1: { x: p1.x, y: p1.y }, p2: { x: p2.x, y: p1.y } },
    { type: 'line', p1: { x: p2.x, y: p1.y }, p2: { x: p2.x, y: p2.y } },
    { type: 'line', p1: { x: p2.x, y: p2.y }, p2: { x: p1.x, y: p2.y } },
    { type: 'line', p1: { x: p1.x, y: p2.y }, p2: { x: p1.x, y: p1.y } },
  ];
}

function previewRect(p2) {
  const sr = toolManager.getSvgRoot();
  if (!sr || !firstPoint) return;
  const layer = sr.getLayer('preview');
  while (layer.firstChild) layer.removeChild(layer.firstChild);
  const er = entityRenderers;
  rectAsLines(firstPoint, p2).forEach(function (l) {
    const node = er.renderLine(l, layer);
    node.setAttribute('stroke-dasharray', '4 2');
    const glow = getComputedStyle(document.documentElement).getPropertyValue('--laser-glow').trim();
    node.setAttribute('stroke', glow);
  });
}

function commitRect(p2) {
  const lines = rectAsLines(firstPoint, p2);
  lines.forEach(function (l) {
    toolManager.commit(commands.addEntity(l));
  });
  firstPoint = null;
  toolManager.cancel();
}

export const rectTool = {
  id: 'rect',
  prompt: 'RECT  Specify first corner:',
  onArm(state) {
    firstPoint = null;
    state.setToolState('armed');
  },
  onCancel() {
    firstPoint = null;
    const sr = toolManager.getSvgRoot();
    if (sr) entityRenderers.clearPreview(sr);
  },
  onPointerMove(e) {
    if (firstPoint) previewRect({ x: e.worldX, y: e.worldY });
  },
  onPointerDown(e, state) {
    const p = { x: e.worldX, y: e.worldY };
    if (!firstPoint) {
      firstPoint = p;
      state.setToolState('preview');
    } else commitRect(p);
  },
  onCommand(payload, state) {
    const p = payload.parsed;
    if (!p) return;
    if (p.kind === 'absolute') {
      const pt = { x: p.x, y: p.y };
      if (!firstPoint) {
        firstPoint = pt;
        state.setToolState('preview');
      } else commitRect(pt);
    } else if (p.kind === 'relative' && firstPoint) {
      commitRect({ x: firstPoint.x + p.dx, y: firstPoint.y + p.dy });
    }
  },
};

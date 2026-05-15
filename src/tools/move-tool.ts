import { state } from '@/app/state.js';
import { bus } from '@/app/event-bus.js';
import { commands } from '@/core/document/commands.js';
import { entityRenderers } from '@/render/entity-renderers.js';
import { toolManager } from '@/tools/tool-manager.js';

let basePoint = null;
let ids = null;

function previewMove(dx, dy) {
  const sr = toolManager.getSvgRoot();
  if (!sr || !ids) return;
  const layer = sr.getLayer('preview');
  while (layer.firstChild) layer.removeChild(layer.firstChild);
  const er = entityRenderers;
  const idSet = new Set(ids);
  state.entities.forEach(function (e) {
    if (!idSet.has(e.id)) return;
    let ghost = null;
    if (e.type === 'line') {
      ghost = {
        type: 'line',
        p1: { x: e.p1.x + dx, y: e.p1.y + dy },
        p2: { x: e.p2.x + dx, y: e.p2.y + dy },
      };
      const n = er.renderLine(ghost, layer);
      applyGhost(n);
    } else if (e.type === 'circle') {
      ghost = { type: 'circle', center: { x: e.center.x + dx, y: e.center.y + dy }, r: e.r };
      const n = er.renderCircle(ghost, layer);
      applyGhost(n);
    } else if (e.type === 'arc') {
      ghost = Object.assign({}, e, { center: { x: e.center.x + dx, y: e.center.y + dy } });
      const n = er.renderArc(ghost, layer);
      applyGhost(n);
    }
  });
}
function applyGhost(n) {
  n.setAttribute('stroke-dasharray', '4 2');
  n.setAttribute(
    'stroke',
    getComputedStyle(document.documentElement).getPropertyValue('--laser-glow').trim(),
  );
}

export const moveTool = {
  id: 'move',
  prompt: 'MOVE  Specify base point:',
  onArm(state) {
    basePoint = null;
    ids = state.selection.slice();
    if (ids.length === 0) {
      bus.emit('command:error', { raw: 'move', message: '! Nothing selected. Use Select first.' });
      toolManager.cancel();
      return;
    }
    state.setToolState('armed');
  },
  onCancel() {
    basePoint = null;
    ids = null;
    const sr = toolManager.getSvgRoot();
    if (sr) entityRenderers.clearPreview(sr);
  },
  onPointerMove(e) {
    if (basePoint) previewMove(e.worldX - basePoint.x, e.worldY - basePoint.y);
  },
  onPointerDown(e, state) {
    const p = { x: e.worldX, y: e.worldY };
    if (!basePoint) {
      basePoint = p;
      state.setToolState('preview');
    } else {
      toolManager.commit(commands.moveEntities(ids, p.x - basePoint.x, p.y - basePoint.y));
      basePoint = null;
      ids = null;
      toolManager.cancel();
    }
  },
  onCommand(payload) {
    const p = payload.parsed;
    if (!p) return;
    if (p.kind === 'absolute' && !basePoint) {
      basePoint = { x: p.x, y: p.y };
      state.setToolState('preview');
    } else if (p.kind === 'relative' && basePoint) {
      toolManager.commit(commands.moveEntities(ids, p.dx, p.dy));
      basePoint = null;
      ids = null;
      toolManager.cancel();
    }
  },
};

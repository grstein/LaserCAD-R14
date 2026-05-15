import { vec2 } from '@/core/geometry/vec2.js';
import { commands } from '@/core/document/commands.js';
import { entityRenderers } from '@/render/entity-renderers.js';
import { toolManager } from '@/tools/tool-manager.js';

// 3-point arc: center, start point on circle, end point on circle (CCW).
let center = null;
let startPt = null;

function angleOf(p, c) {
  return Math.atan2(p.y - c.y, p.x - c.x);
}

function previewArc(p) {
  if (!center) return;
  const sr = toolManager.getSvgRoot();
  if (!sr) return;
  if (!startPt) {
    const r = vec2.len(vec2.sub(p, center));
    entityRenderers.renderPreview(sr, { type: 'circle', center: center, r: r });
    return;
  }
  const r = vec2.len(vec2.sub(startPt, center));
  const s = angleOf(startPt, center);
  const e = angleOf(p, center);
  entityRenderers.renderPreview(sr, {
    type: 'arc',
    center: center,
    r: r,
    startAngle: s,
    endAngle: e,
    ccw: true,
  });
}

function commitArc(p) {
  const r = vec2.len(vec2.sub(startPt, center));
  const s = angleOf(startPt, center);
  const e = angleOf(p, center);
  toolManager.commit(
    commands.addEntity({
      type: 'arc',
      center: { x: center.x, y: center.y },
      r: r,
      startAngle: s,
      endAngle: e,
      ccw: true,
    }),
  );
  center = null;
  startPt = null;
  toolManager.cancel();
}

export const arcTool = {
  id: 'arc',
  prompt: 'ARC  Specify center:',
  onArm(state) {
    center = null;
    startPt = null;
    state.setToolState('armed');
  },
  onCancel() {
    center = null;
    startPt = null;
    const sr = toolManager.getSvgRoot();
    if (sr) entityRenderers.clearPreview(sr);
  },
  onPointerMove(e) {
    previewArc({ x: e.worldX, y: e.worldY });
  },
  onPointerDown(e, state) {
    const p = { x: e.worldX, y: e.worldY };
    if (!center) {
      center = p;
      state.setToolState('preview');
    } else if (!startPt) {
      startPt = p;
    } else commitArc(p);
  },
  onCommand(payload, state) {
    const p = payload.parsed;
    if (!p || p.kind !== 'absolute') return;
    const pt = { x: p.x, y: p.y };
    if (!center) {
      center = pt;
      state.setToolState('preview');
    } else if (!startPt) {
      startPt = pt;
    } else commitArc(pt);
  },
};

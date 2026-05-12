(function (LaserCAD) {
  'use strict';

  const vec2 = function () { return LaserCAD.core.geometry.vec2; };
  const commands = function () { return LaserCAD.core.document.commands; };
  const TM = function () { return LaserCAD.tools.toolManager; };

  // 3-point arc: center, start point on circle, end point on circle (CCW).
  let center = null;
  let startPt = null;

  function angleOf(p, c) { return Math.atan2(p.y - c.y, p.x - c.x); }

  function previewArc(p) {
    if (!center) return;
    const sr = TM().getSvgRoot();
    if (!sr) return;
    if (!startPt) {
      const r = vec2().len(vec2().sub(p, center));
      LaserCAD.render.entityRenderers.renderPreview(sr, { type: 'circle', center: center, r: r });
      return;
    }
    const r = vec2().len(vec2().sub(startPt, center));
    const s = angleOf(startPt, center);
    const e = angleOf(p, center);
    LaserCAD.render.entityRenderers.renderPreview(sr, { type: 'arc', center: center, r: r, startAngle: s, endAngle: e, ccw: true });
  }

  function commitArc(p) {
    const r = vec2().len(vec2().sub(startPt, center));
    const s = angleOf(startPt, center);
    const e = angleOf(p, center);
    TM().commit(commands().addEntity({ type: 'arc', center: { x: center.x, y: center.y }, r: r, startAngle: s, endAngle: e, ccw: true }));
    center = null; startPt = null;
    LaserCAD.tools.toolManager.cancel();
  }

  LaserCAD.tools.arcTool = {
    id: 'arc',
    prompt: 'ARC  Specify center:',
    onArm(state) { center = null; startPt = null; state.setToolState('armed'); },
    onCancel()   { center = null; startPt = null; const sr = TM().getSvgRoot(); if (sr) LaserCAD.render.entityRenderers.clearPreview(sr); },
    onPointerMove(e) { previewArc({ x: e.worldX, y: e.worldY }); },
    onPointerDown(e, state) {
      const p = { x: e.worldX, y: e.worldY };
      if (!center) { center = p; state.setToolState('preview'); }
      else if (!startPt) { startPt = p; }
      else commitArc(p);
    },
    onCommand(payload, state) {
      const p = payload.parsed;
      if (!p || p.kind !== 'absolute') return;
      const pt = { x: p.x, y: p.y };
      if (!center) { center = pt; state.setToolState('preview'); }
      else if (!startPt) { startPt = pt; }
      else commitArc(pt);
    }
  };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

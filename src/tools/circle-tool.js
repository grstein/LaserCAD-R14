(function (LaserCAD) {
  'use strict';

  const vec2 = function () { return LaserCAD.core.geometry.vec2; };
  const commands = function () { return LaserCAD.core.document.commands; };
  const TM = function () { return LaserCAD.tools.toolManager; };

  let center = null;

  function previewCircle(p2) {
    if (!center) return;
    const sr = TM().getSvgRoot();
    if (!sr) return;
    const r = vec2().len(vec2().sub(p2, center));
    if (r <= 0) return;
    LaserCAD.render.entityRenderers.renderPreview(sr, { type: 'circle', center: center, r: r });
  }

  function commitCircle(r) {
    if (!(r > 0)) return;
    TM().commit(commands().addEntity({ type: 'circle', center: { x: center.x, y: center.y }, r: r }));
    center = null;
    LaserCAD.tools.toolManager.cancel();
  }

  LaserCAD.tools.circleTool = {
    id: 'circle',
    prompt: 'CIRCLE  Specify center:',
    onArm(state) { center = null; state.setToolState('armed'); },
    onCancel()   { center = null; const sr = TM().getSvgRoot(); if (sr) LaserCAD.render.entityRenderers.clearPreview(sr); },
    onPointerMove(e) { if (center) previewCircle({ x: e.worldX, y: e.worldY }); },
    onPointerDown(e, state) {
      const p = { x: e.worldX, y: e.worldY };
      if (!center) { center = p; state.setToolState('preview'); }
      else commitCircle(vec2().len(vec2().sub(p, center)));
    },
    onCommand(payload, state) {
      const p = payload.parsed;
      if (!p) return;
      if (p.kind === 'absolute' && !center) {
        center = { x: p.x, y: p.y }; state.setToolState('preview');
      } else if (p.kind === 'distance' && center) {
        commitCircle(p.value);
      } else if (p.kind === 'absolute' && center) {
        commitCircle(vec2().len(vec2().sub({ x: p.x, y: p.y }, center)));
      }
    }
  };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

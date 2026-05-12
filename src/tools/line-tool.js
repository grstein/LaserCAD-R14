(function (LaserCAD) {
  'use strict';

  const vec2 = function () { return LaserCAD.core.geometry.vec2; };
  const commands = function () { return LaserCAD.core.document.commands; };
  const TM = function () { return LaserCAD.tools.toolManager; };

  let firstPoint = null;

  function orthoSnap(p1, p2) {
    const dx = Math.abs(p2.x - p1.x);
    const dy = Math.abs(p2.y - p1.y);
    return dx >= dy ? { x: p2.x, y: p1.y } : { x: p1.x, y: p2.y };
  }

  function previewTo(p2, applyOrtho) {
    if (!firstPoint) return;
    const sr = TM().getSvgRoot();
    if (!sr) return;
    const end = applyOrtho ? orthoSnap(firstPoint, p2) : p2;
    LaserCAD.render.entityRenderers.renderPreview(sr, {
      type: 'line', p1: firstPoint, p2: end
    });
  }

  function commitLine(p2) {
    const cmd = commands().addEntity({ type: 'line', p1: firstPoint, p2: { x: p2.x, y: p2.y } });
    TM().commit(cmd);
    firstPoint = p2;
  }

  LaserCAD.tools.lineTool = {
    id: 'line',
    prompt: 'LINE  Specify first point:',
    onArm(state) {
      firstPoint = null;
      state.setToolState('armed');
    },
    onCancel(_state) {
      firstPoint = null;
      const sr = TM().getSvgRoot();
      if (sr) LaserCAD.render.entityRenderers.clearPreview(sr);
    },
    onPointerMove(e, state) {
      if (firstPoint) {
        const ortho = e.shiftKey || state.toggles.ortho;
        previewTo({ x: e.worldX, y: e.worldY }, ortho);
      }
    },
    onPointerDown(e, state) {
      const p = { x: e.worldX, y: e.worldY };
      const ortho = e.shiftKey || state.toggles.ortho;
      if (!firstPoint) {
        firstPoint = p;
        state.setToolState('preview');
      } else {
        commitLine(ortho ? orthoSnap(firstPoint, p) : p);
      }
    },
    onCommand(payload, state) {
      const p = payload.parsed;
      if (!p) return;
      if (p.kind === 'absolute') {
        const pt = { x: p.x, y: p.y };
        if (!firstPoint) { firstPoint = pt; state.setToolState('preview'); }
        else commitLine(pt);
      } else if (p.kind === 'relative' && firstPoint) {
        commitLine({ x: firstPoint.x + p.dx, y: firstPoint.y + p.dy });
      } else if (p.kind === 'distance' && firstPoint && state.cursor) {
        // Direção do cursor a partir do primeiro ponto
        const cur = { x: state.cursor.worldX, y: state.cursor.worldY };
        const ortho = state.toggles.ortho;
        const dir = ortho ? orthoSnap(firstPoint, cur) : cur;
        const v = vec2().normalize(vec2().sub(dir, firstPoint));
        if (v.x === 0 && v.y === 0) return;
        commitLine({ x: firstPoint.x + v.x * p.value, y: firstPoint.y + v.y * p.value });
      }
    }
  };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

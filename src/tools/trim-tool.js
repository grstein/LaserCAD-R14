(function (LaserCAD) {
  'use strict';

  const commands = function () { return LaserCAD.core.document.commands; };
  const TM = function () { return LaserCAD.tools.toolManager; };
  const intersect = function () { return LaserCAD.core.geometry.intersect; };

  let cuttingEdge = null;

  function distPointSegment(p, a, b) {
    const dx = b.x - a.x, dy = b.y - a.y;
    const t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / Math.max(dx*dx + dy*dy, 1e-12);
    const tc = Math.max(0, Math.min(1, t));
    const cx = a.x + tc * dx, cy = a.y + tc * dy;
    return Math.sqrt((p.x - cx)*(p.x - cx) + (p.y - cy)*(p.y - cy));
  }
  function hitEntity(p, entities, tol) {
    for (let i = entities.length - 1; i >= 0; i--) {
      const e = entities[i];
      if (e.type === 'line') {
        if (distPointSegment(p, e.p1, e.p2) <= tol) return e;
      } else if (e.type === 'circle') {
        const dx = p.x - e.center.x, dy = p.y - e.center.y;
        if (Math.abs(Math.sqrt(dx*dx + dy*dy) - e.r) <= tol) return e;
      }
    }
    return null;
  }

  function trimLineByLine(target, edge, clickPt) {
    const ip = intersect().lineLine(target, edge);
    if (!ip) return null;
    const dP1 = Math.hypot(clickPt.x - target.p1.x, clickPt.y - target.p1.y);
    const dP2 = Math.hypot(clickPt.x - target.p2.x, clickPt.y - target.p2.y);
    // remove a metade do lado do clique
    if (dP1 < dP2) {
      return { type: 'line', p1: { x: ip.x, y: ip.y }, p2: { x: target.p2.x, y: target.p2.y } };
    } else {
      return { type: 'line', p1: { x: target.p1.x, y: target.p1.y }, p2: { x: ip.x, y: ip.y } };
    }
  }

  function trimLineByCircle(target, edge, clickPt) {
    const ips = intersect().lineCircle(target, edge);
    if (!ips || ips.length === 0) return null;
    // pega a interseção mais distante do click (mantém a parte do click)
    let kept = ips[0], dKept = Infinity;
    for (let i = 0; i < ips.length; i++) {
      const d = Math.hypot(ips[i].x - clickPt.x, ips[i].y - clickPt.y);
      if (d < dKept) { dKept = d; kept = ips[i]; }
    }
    // qual extremidade está mais próxima do click? remove dessa extremidade até kept
    const dP1 = Math.hypot(clickPt.x - target.p1.x, clickPt.y - target.p1.y);
    const dP2 = Math.hypot(clickPt.x - target.p2.x, clickPt.y - target.p2.y);
    if (dP1 < dP2) return { type: 'line', p1: { x: kept.x, y: kept.y }, p2: target.p2 };
    return { type: 'line', p1: target.p1, p2: { x: kept.x, y: kept.y } };
  }

  LaserCAD.tools.trimTool = {
    id: 'trim',
    prompt: 'TRIM  Select cutting edge:',
    onArm(state) { cuttingEdge = null; state.setToolState('armed'); },
    onCancel()   { cuttingEdge = null; },
    onPointerDown(e, state) {
      const p = { x: e.worldX, y: e.worldY };
      const pxPerMm = LaserCAD.render.camera.pxPerMm(state.camera);
      const tol = 6 / Math.max(pxPerMm, 0.0001);
      const hit = hitEntity(p, state.entities, tol);
      if (!hit) return;
      if (!cuttingEdge) {
        cuttingEdge = hit;
        return;
      }
      if (hit.id === cuttingEdge.id) return;
      let newGeom = null;
      if (hit.type === 'line' && cuttingEdge.type === 'line') newGeom = trimLineByLine(hit, cuttingEdge, p);
      else if (hit.type === 'line' && cuttingEdge.type === 'circle') newGeom = trimLineByCircle(hit, cuttingEdge, p);
      else if (hit.type === 'circle' && cuttingEdge.type === 'line') {
        LaserCAD.bus.emit('command:error', { raw: 'trim', message: '! Trim circle by line not implemented in Sprint 5' });
        return;
      }
      if (!newGeom) {
        LaserCAD.bus.emit('command:error', { raw: 'trim', message: '! No intersection found' });
        return;
      }
      TM().commit(commands().removeEntity(hit.id));
      TM().commit(commands().addEntity(newGeom));
    }
  };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

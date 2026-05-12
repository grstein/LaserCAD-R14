(function (LaserCAD) {
  'use strict';

  const ns = LaserCAD.render;

  ns.overlays = {
    mount(svgRoot) {
      const host = svgRoot.host;
      const svg  = svgRoot.element;

      const hLine = document.createElement('div');
      const vLine = document.createElement('div');
      const label = document.createElement('div');
      hLine.className = 'crosshair-h';
      vLine.className = 'crosshair-v';
      label.className = 'cursor-label';
      label.style.display = 'none';
      host.appendChild(hLine);
      host.appendChild(vLine);
      host.appendChild(label);

      let panActive = false;
      let panLast = null;
      let spaceHeld = false;

      // Snap marker (SVG node em #snaps)
      function snapColor(type) {
        const map = { endpoint: '--snap-endpoint', midpoint: '--snap-midpoint', center: '--snap-center', intersection: '--snap-intersection' };
        return getComputedStyle(document.documentElement).getPropertyValue(map[type] || '--snap-endpoint').trim();
      }
      function showSnapMarker(best) {
        const layer = svgRoot.getLayer('snaps');
        while (layer.firstChild) layer.removeChild(layer.firstChild);
        const pxPerMm = LaserCAD.render.camera.pxPerMm(LaserCAD.app.state.camera);
        const sizeMm = 10 / pxPerMm;
        const color = snapColor(best.type);
        let n;
        if (best.type === 'endpoint') {
          n = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
          n.setAttribute('x', best.point.x - sizeMm/2);
          n.setAttribute('y', best.point.y - sizeMm/2);
          n.setAttribute('width', sizeMm); n.setAttribute('height', sizeMm);
        } else if (best.type === 'midpoint') {
          n = document.createElementNS('http://www.w3.org/2000/svg', 'polygon');
          const h = sizeMm/2;
          n.setAttribute('points',
            (best.point.x) + ',' + (best.point.y - h) + ' ' +
            (best.point.x + h) + ',' + (best.point.y + h) + ' ' +
            (best.point.x - h) + ',' + (best.point.y + h));
        } else if (best.type === 'center') {
          n = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
          n.setAttribute('cx', best.point.x); n.setAttribute('cy', best.point.y); n.setAttribute('r', sizeMm/2);
        } else {
          n = document.createElementNS('http://www.w3.org/2000/svg', 'g');
          const l1 = document.createElementNS('http://www.w3.org/2000/svg', 'line');
          const l2 = document.createElementNS('http://www.w3.org/2000/svg', 'line');
          const h = sizeMm/2;
          l1.setAttribute('x1', best.point.x - h); l1.setAttribute('y1', best.point.y - h);
          l1.setAttribute('x2', best.point.x + h); l1.setAttribute('y2', best.point.y + h);
          l2.setAttribute('x1', best.point.x + h); l2.setAttribute('y1', best.point.y - h);
          l2.setAttribute('x2', best.point.x - h); l2.setAttribute('y2', best.point.y + h);
          [l1, l2].forEach(function (x) {
            x.setAttribute('stroke', color); x.setAttribute('stroke-width', '0.15');
            x.setAttribute('vector-effect', 'non-scaling-stroke');
          });
          n.appendChild(l1); n.appendChild(l2);
        }
        if (n && n.tagName !== 'g') {
          n.setAttribute('stroke', color);
          n.setAttribute('fill', 'none');
          n.setAttribute('stroke-width', '0.15');
          n.setAttribute('vector-effect', 'non-scaling-stroke');
        }
        layer.appendChild(n);
      }
      function hideSnapMarker() {
        const layer = svgRoot.getLayer('snaps');
        while (layer.firstChild) layer.removeChild(layer.firstChild);
      }

      function updateCrosshair(screenX, screenY) {
        const rect = host.getBoundingClientRect();
        hLine.style.top  = (screenY - rect.top)  + 'px';
        vLine.style.left = (screenX - rect.left) + 'px';
      }

      function updateLabel(world, screenX, screenY) {
        const rect = host.getBoundingClientRect();
        const armed = LaserCAD.app.state.toolState !== 'idle';
        if (!armed) { label.style.display = 'none'; return; }
        label.style.display = 'block';
        label.style.left = (screenX - rect.left + 16) + 'px';
        label.style.top  = (screenY - rect.top  + 16) + 'px';
        label.textContent = world.x.toFixed(3) + ', ' + world.y.toFixed(3) + ' mm';
      }

      function onPointerMove(e) {
        const world = LaserCAD.render.camera.worldFromScreen({ x: e.clientX, y: e.clientY });
        // Snap detection (mm tolerance = SNAP_TOLERANCE_PX / pxPerMm)
        const snap = LaserCAD.core.geometry.snap;
        let snapped = world;
        if (snap) {
          const pxPerMm = LaserCAD.render.camera.pxPerMm(LaserCAD.app.state.camera);
          const tolMm = LaserCAD.core.geometry.epsilon.SNAP_TOLERANCE_PX / Math.max(pxPerMm, 0.0001);
          const best = snap.findBest(LaserCAD.app.state, world, tolMm);
          if (best) {
            snapped = best.point;
            showSnapMarker(best);
          } else {
            hideSnapMarker();
          }
        }
        LaserCAD.app.state.setCursor({
          worldX: snapped.x, worldY: snapped.y, screenX: e.clientX, screenY: e.clientY
        });
        updateCrosshair(e.clientX, e.clientY);
        updateLabel(snapped, e.clientX, e.clientY);

        // Dispatch para a tool ativa (apenas quando não em pan).
        if (!panActive && LaserCAD.tools.toolManager) {
          LaserCAD.tools.toolManager.onPointerMove({
            clientX: e.clientX, clientY: e.clientY,
            worldX: world.x, worldY: world.y,
            shiftKey: e.shiftKey, ctrlKey: e.ctrlKey, button: e.button
          });
        }

        if (panActive && panLast) {
          const dxMm = (panLast.x - e.clientX) / LaserCAD.render.camera.pxPerMm(LaserCAD.app.state.camera);
          const dyMm = (panLast.y - e.clientY) / LaserCAD.render.camera.pxPerMm(LaserCAD.app.state.camera);
          LaserCAD.render.camera.panBy(dxMm, dyMm);
          panLast = { x: e.clientX, y: e.clientY };
        }
      }

      function onPointerDown(e) {
        if (e.button === 1 || (e.button === 0 && spaceHeld)) {
          panActive = true;
          panLast = { x: e.clientX, y: e.clientY };
          host.setPointerCapture && host.setPointerCapture(e.pointerId);
          e.preventDefault();
          return;
        }
        if (e.button === 0 && LaserCAD.tools.toolManager) {
          const w = LaserCAD.render.camera.worldFromScreen({ x: e.clientX, y: e.clientY });
          LaserCAD.tools.toolManager.onPointerDown({
            clientX: e.clientX, clientY: e.clientY,
            worldX: w.x, worldY: w.y,
            shiftKey: e.shiftKey, ctrlKey: e.ctrlKey, button: e.button
          });
        }
      }
      function onPointerUp(e) {
        if (panActive) {
          panActive = false; panLast = null;
          host.releasePointerCapture && host.releasePointerCapture(e.pointerId);
          return;
        }
        if (e.button === 0 && LaserCAD.tools.toolManager) {
          const w = LaserCAD.render.camera.worldFromScreen({ x: e.clientX, y: e.clientY });
          LaserCAD.tools.toolManager.onPointerUp({
            clientX: e.clientX, clientY: e.clientY,
            worldX: w.x, worldY: w.y,
            shiftKey: e.shiftKey, ctrlKey: e.ctrlKey, button: e.button
          });
        }
      }
      function onWheel(e) {
        e.preventDefault();
        const factor = e.deltaY < 0 ? 1.1 : 1 / 1.1;
        LaserCAD.render.camera.zoomAt({ x: e.clientX, y: e.clientY }, factor);
      }
      function onKeyDown(e) {
        if (e.code === 'Space' && !spaceHeld) { spaceHeld = true; host.style.cursor = 'grab'; }
      }
      function onKeyUp(e) {
        if (e.code === 'Space') { spaceHeld = false; host.style.cursor = ''; }
      }

      function syncArmedClass() {
        const armed = LaserCAD.app.state.toolState !== 'idle';
        host.classList.toggle('armed', armed);
      }

      host.addEventListener('pointermove', onPointerMove);
      host.addEventListener('pointerdown', onPointerDown);
      host.addEventListener('pointerup',   onPointerUp);
      host.addEventListener('pointercancel', onPointerUp);
      host.addEventListener('wheel', onWheel, { passive: false });
      window.addEventListener('keydown', onKeyDown);
      window.addEventListener('keyup', onKeyUp);

      LaserCAD.bus.on('tool:armed',  syncArmedClass);
      LaserCAD.bus.on('tool:cancel', syncArmedClass);
      LaserCAD.bus.on('app:ready',   syncArmedClass);
    }
  };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

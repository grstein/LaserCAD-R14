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
        LaserCAD.app.state.setCursor({
          worldX: world.x, worldY: world.y, screenX: e.clientX, screenY: e.clientY
        });
        updateCrosshair(e.clientX, e.clientY);
        updateLabel(world, e.clientX, e.clientY);

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

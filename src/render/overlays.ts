import { state } from '@/app/state.js';
import { bus } from '@/app/event-bus.js';
import { epsilon } from '@/core/geometry/epsilon.js';
import { snap } from '@/core/geometry/snap.js';
import { camera } from '@/render/camera.js';
import { toolManager } from '@/tools/tool-manager.js';

export const overlays = {
  mount(svgRoot) {
    const host = svgRoot.host;
    const svg = svgRoot.element;

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
      const map = {
        endpoint: '--snap-endpoint',
        midpoint: '--snap-midpoint',
        center: '--snap-center',
        intersection: '--snap-intersection',
      };
      return getComputedStyle(document.documentElement)
        .getPropertyValue(map[type] || '--snap-endpoint')
        .trim();
    }
    function showSnapMarker(best) {
      const layer = svgRoot.getLayer('snaps');
      while (layer.firstChild) layer.removeChild(layer.firstChild);
      const pxPerMm = camera.pxPerMm(state.camera);
      const sizeMm = 10 / pxPerMm;
      const color = snapColor(best.type);
      let n;
      if (best.type === 'endpoint') {
        n = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
        n.setAttribute('x', best.point.x - sizeMm / 2);
        n.setAttribute('y', best.point.y - sizeMm / 2);
        n.setAttribute('width', sizeMm);
        n.setAttribute('height', sizeMm);
      } else if (best.type === 'midpoint') {
        n = document.createElementNS('http://www.w3.org/2000/svg', 'polygon');
        const h = sizeMm / 2;
        n.setAttribute(
          'points',
          best.point.x +
            ',' +
            (best.point.y - h) +
            ' ' +
            (best.point.x + h) +
            ',' +
            (best.point.y + h) +
            ' ' +
            (best.point.x - h) +
            ',' +
            (best.point.y + h),
        );
      } else if (best.type === 'center') {
        n = document.createElementNS('http://www.w3.org/2000/svg', 'circle');
        n.setAttribute('cx', best.point.x);
        n.setAttribute('cy', best.point.y);
        n.setAttribute('r', sizeMm / 2);
      } else {
        n = document.createElementNS('http://www.w3.org/2000/svg', 'g');
        const l1 = document.createElementNS('http://www.w3.org/2000/svg', 'line');
        const l2 = document.createElementNS('http://www.w3.org/2000/svg', 'line');
        const h = sizeMm / 2;
        l1.setAttribute('x1', String(best.point.x - h));
        l1.setAttribute('y1', String(best.point.y - h));
        l1.setAttribute('x2', String(best.point.x + h));
        l1.setAttribute('y2', String(best.point.y + h));
        l2.setAttribute('x1', String(best.point.x + h));
        l2.setAttribute('y1', String(best.point.y - h));
        l2.setAttribute('x2', String(best.point.x - h));
        l2.setAttribute('y2', String(best.point.y + h));
        [l1, l2].forEach(function (x) {
          x.setAttribute('stroke', color);
          x.setAttribute('stroke-width', '1.5');
          x.setAttribute('vector-effect', 'non-scaling-stroke');
        });
        n.appendChild(l1);
        n.appendChild(l2);
      }
      if (n && n.tagName !== 'g') {
        n.setAttribute('stroke', color);
        n.setAttribute('fill', 'none');
        n.setAttribute('stroke-width', '1.5');
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
      hLine.style.top = screenY - rect.top + 'px';
      vLine.style.left = screenX - rect.left + 'px';
    }

    function updateLabel(world, screenX, screenY) {
      const rect = host.getBoundingClientRect();
      const armed = state.toolState !== 'idle';
      if (!armed) {
        label.style.display = 'none';
        return;
      }
      label.style.display = 'block';
      label.style.left = screenX - rect.left + 16 + 'px';
      label.style.top = screenY - rect.top + 16 + 'px';
      label.textContent = world.x.toFixed(3) + ', ' + world.y.toFixed(3) + ' mm';
    }

    function resolveSnap(clientX, clientY, opts?) {
      const world = camera.worldFromScreen({ x: clientX, y: clientY });
      let snapped = world;
      let best = null;
      if (snap) {
        const pxPerMm = camera.pxPerMm(state.camera);
        const tolMm = epsilon.SNAP_TOLERANCE_PX / Math.max(pxPerMm, 0.0001);
        best = snap.findBest(state, world, tolMm);
        if (best) snapped = best.point;
      }
      const b = state.documentBounds;
      if (b && b.w > 0 && b.h > 0) {
        snapped = {
          x: Math.max(0, Math.min(b.w, snapped.x)),
          y: Math.max(0, Math.min(b.h, snapped.y)),
        };
      }
      return { world: world, snapped: snapped, best: best };
    }

    function onPointerMove(e) {
      const r = resolveSnap(e.clientX, e.clientY);
      if (r.best) showSnapMarker(r.best);
      else hideSnapMarker();
      state.setCursor({
        worldX: r.snapped.x,
        worldY: r.snapped.y,
        screenX: e.clientX,
        screenY: e.clientY,
      });
      updateCrosshair(e.clientX, e.clientY);
      updateLabel(r.snapped, e.clientX, e.clientY);

      // Dispatch to the active tool (only when not panning).
      if (!panActive && toolManager) {
        toolManager.onPointerMove({
          clientX: e.clientX,
          clientY: e.clientY,
          worldX: r.snapped.x,
          worldY: r.snapped.y,
          shiftKey: e.shiftKey,
          ctrlKey: e.ctrlKey,
          button: e.button,
        });
      }

      if (panActive && panLast) {
        const dxMm = (panLast.x - e.clientX) / camera.pxPerMm(state.camera);
        const dyMm = (panLast.y - e.clientY) / camera.pxPerMm(state.camera);
        camera.panBy(dxMm, dyMm);
        panLast = { x: e.clientX, y: e.clientY };
      }
    }

    function onPointerDown(e) {
      if (e.button === 1 || (e.button === 0 && spaceHeld)) {
        panActive = true;
        panLast = { x: e.clientX, y: e.clientY };
        if (host.setPointerCapture) host.setPointerCapture(e.pointerId);
        e.preventDefault();
        return;
      }
      if (e.button === 0 && toolManager) {
        const r = resolveSnap(e.clientX, e.clientY);
        toolManager.onPointerDown({
          clientX: e.clientX,
          clientY: e.clientY,
          worldX: r.snapped.x,
          worldY: r.snapped.y,
          shiftKey: e.shiftKey,
          ctrlKey: e.ctrlKey,
          button: e.button,
        });
      }
    }
    function onPointerUp(e) {
      if (panActive) {
        panActive = false;
        panLast = null;
        if (host.releasePointerCapture) host.releasePointerCapture(e.pointerId);
        return;
      }
      if (e.button === 0 && toolManager) {
        const r = resolveSnap(e.clientX, e.clientY);
        toolManager.onPointerUp({
          clientX: e.clientX,
          clientY: e.clientY,
          worldX: r.snapped.x,
          worldY: r.snapped.y,
          shiftKey: e.shiftKey,
          ctrlKey: e.ctrlKey,
          button: e.button,
        });
      }
    }
    function onWheel(e) {
      e.preventDefault();
      const factor = e.deltaY < 0 ? 1.1 : 1 / 1.1;
      camera.zoomAt({ x: e.clientX, y: e.clientY }, factor);
    }
    function onKeyDown(e) {
      if (e.code === 'Space' && !spaceHeld) {
        spaceHeld = true;
        host.style.cursor = 'grab';
      }
    }
    function onKeyUp(e) {
      if (e.code === 'Space') {
        spaceHeld = false;
        host.style.cursor = '';
      }
    }

    function syncArmedClass() {
      const armed = state.toolState !== 'idle';
      host.classList.toggle('armed', armed);
    }

    host.addEventListener('pointermove', onPointerMove);
    host.addEventListener('pointerdown', onPointerDown);
    host.addEventListener('pointerup', onPointerUp);
    host.addEventListener('pointercancel', onPointerUp);
    host.addEventListener('wheel', onWheel, { passive: false });
    window.addEventListener('keydown', onKeyDown);
    window.addEventListener('keyup', onKeyUp);

    bus.on('tool:armed', syncArmedClass);
    bus.on('tool:cancel', syncArmedClass);
    bus.on('app:ready', syncArmedClass);
  },
};

(function (LaserCAD) {
  'use strict';

  const ns = LaserCAD.render;

  let svgRootHandle = null;

  function getCam(cam) {
    return cam || (LaserCAD.app.state && LaserCAD.app.state.camera);
  }

  function pxPerMm(cam) {
    const base = (LaserCAD.app.config && LaserCAD.app.config.get('initialZoomPxPerMm')) || 4;
    return base * cam.zoom;
  }

  ns.camera = {
    attach(svgRoot) { svgRootHandle = svgRoot; },
    mount(svgRoot)  { svgRootHandle = svgRoot; },
    isAttached()    { return svgRootHandle !== null; },
    get()           { return LaserCAD.app.state.camera; },

    /**
     * Conversão screen → world via getScreenCTM.inverse() do <svg> raiz.
     * @param {{x:number,y:number}} screenPt em coordenadas do cliente (clientX/clientY)
     * @param {object} [cam] estado da câmera; default = state.camera
     * @returns {{x:number,y:number}} ponto em mm
     */
    worldFromScreen(screenPt, cam) {
      if (!svgRootHandle) {
        throw new Error('[LaserCAD] camera.worldFromScreen: svgRoot not attached. See ADR 0002 §2.');
      }
      const svg = svgRootHandle.element;
      const ctm = svg.getScreenCTM();
      if (!ctm) return { x: 0, y: 0 };
      const pt = svg.createSVGPoint();
      pt.x = screenPt.x;
      pt.y = screenPt.y;
      const w = pt.matrixTransform(ctm.inverse());
      return { x: w.x, y: w.y };
    },

    /**
     * Conversão world → screen.
     * @param {{x:number,y:number}} worldPt em mm
     * @param {object} [cam]
     * @returns {{x:number,y:number}} em pixels do cliente
     */
    screenFromWorld(worldPt, cam) {
      if (!svgRootHandle) {
        throw new Error('[LaserCAD] camera.screenFromWorld: svgRoot not attached.');
      }
      const svg = svgRootHandle.element;
      const ctm = svg.getScreenCTM();
      if (!ctm) return { x: 0, y: 0 };
      const pt = svg.createSVGPoint();
      pt.x = worldPt.x;
      pt.y = worldPt.y;
      const s = pt.matrixTransform(ctm);
      return { x: s.x, y: s.y };
    },

    /** @param {number} dxMm @param {number} dyMm */
    panBy(dxMm, dyMm) {
      const cam = LaserCAD.app.state.camera;
      LaserCAD.app.state.setCamera({ cx: cam.cx + dxMm, cy: cam.cy + dyMm, zoom: cam.zoom });
    },

    /**
     * Zoom preservando o ponto sob `screenPt` (pivot).
     * @param {{x:number,y:number}} screenPt
     * @param {number} factor multiplicador (>1 zoom in, <1 zoom out)
     */
    zoomAt(screenPt, factor) {
      const config = LaserCAD.app.config;
      const min = config.get('zoomMin'), max = config.get('zoomMax');
      const cam = LaserCAD.app.state.camera;
      const worldBefore = ns.camera.worldFromScreen(screenPt);
      let nextZoom = cam.zoom * factor;
      if (nextZoom < min) nextZoom = min;
      if (nextZoom > max) nextZoom = max;
      LaserCAD.app.state.setCamera({ cx: cam.cx, cy: cam.cy, zoom: nextZoom });
      const worldAfter = ns.camera.worldFromScreen(screenPt);
      const dx = worldAfter.x - worldBefore.x;
      const dy = worldAfter.y - worldBefore.y;
      LaserCAD.app.state.setCamera({ cx: cam.cx - dx, cy: cam.cy - dy, zoom: cam.zoom });
    },

    /** Centraliza a câmera para enquadrar o documentBounds. */
    zoomExtents() {
      const cam = LaserCAD.app.state.camera;
      const bounds = LaserCAD.app.state.documentBounds;
      if (!cam.viewportW || !cam.viewportH) return;
      const config = LaserCAD.app.config;
      const margin = 1.1;
      const baseZ = config.get('initialZoomPxPerMm');
      const fitW = cam.viewportW / (bounds.w * margin * baseZ);
      const fitH = cam.viewportH / (bounds.h * margin * baseZ);
      const zoom = Math.min(fitW, fitH);
      LaserCAD.app.state.setCamera({ cx: bounds.w / 2, cy: bounds.h / 2, zoom: zoom });
    },

    pxPerMm: pxPerMm
  };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

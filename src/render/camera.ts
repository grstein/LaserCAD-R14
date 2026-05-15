import { state } from '@/app/state.js';
import { config } from '@/app/config.js';

let svgRootHandle = null;

function getCam(cam) {
  return cam || (state && state.camera);
}

function pxPerMm(cam) {
  const base = (config && config.get('initialZoomPxPerMm')) || 4;
  return base * cam.zoom;
}

export const camera = {
  attach(svgRoot) {
    svgRootHandle = svgRoot;
  },
  mount(svgRoot) {
    svgRootHandle = svgRoot;
  },
  isAttached() {
    return svgRootHandle !== null;
  },
  get() {
    return state.camera;
  },

  /**
   * Screen → world conversion via getScreenCTM.inverse() on the root <svg>.
   * @param {{x:number,y:number}} screenPt in client coordinates (clientX/clientY)
   * @param {object} [cam] camera state; default = state.camera
   * @returns {{x:number,y:number}} point in mm
   */
  worldFromScreen(screenPt, cam?) {
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
   * World → screen conversion.
   * @param {{x:number,y:number}} worldPt in mm
   * @param {object} [cam]
   * @returns {{x:number,y:number}} in client pixels
   */
  screenFromWorld(worldPt, cam?) {
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
    const cam = state.camera;
    state.setCamera({ cx: cam.cx + dxMm, cy: cam.cy + dyMm, zoom: cam.zoom });
  },

  /**
   * Zoom preserving the point under `screenPt` (pivot).
   * @param {{x:number,y:number}} screenPt
   * @param {number} factor multiplier (>1 zoom in, <1 zoom out)
   */
  zoomAt(screenPt, factor) {
    const min = config.get('zoomMin'),
      max = config.get('zoomMax');
    const cam = state.camera;
    const worldBefore = camera.worldFromScreen(screenPt);
    let nextZoom = cam.zoom * factor;
    if (nextZoom < min) nextZoom = min;
    if (nextZoom > max) nextZoom = max;
    state.setCamera({ cx: cam.cx, cy: cam.cy, zoom: nextZoom });
    const worldAfter = camera.worldFromScreen(screenPt);
    const dx = worldAfter.x - worldBefore.x;
    const dy = worldAfter.y - worldBefore.y;
    state.setCamera({ cx: cam.cx - dx, cy: cam.cy - dy, zoom: cam.zoom });
  },

  /** Centers the camera to frame the documentBounds. */
  zoomExtents() {
    const cam = state.camera;
    const bounds = state.documentBounds;
    if (!cam.viewportW || !cam.viewportH) return;
    const margin = 1.1;
    const baseZ = config.get('initialZoomPxPerMm');
    const fitW = cam.viewportW / (bounds.w * margin * baseZ);
    const fitH = cam.viewportH / (bounds.h * margin * baseZ);
    const zoom = Math.min(fitW, fitH);
    state.setCamera({ cx: bounds.w / 2, cy: bounds.h / 2, zoom: zoom });
  },

  pxPerMm: pxPerMm,
};

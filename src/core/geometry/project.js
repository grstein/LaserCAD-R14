(function (LaserCAD) {
  'use strict';

  const ns = LaserCAD.core.geometry;

  function getCamera() {
    const impl = window.LaserCAD.render && window.LaserCAD.render.camera;
    if (!impl) {
      throw new Error('[LaserCAD] render.camera not loaded — project.* requires script order per namespace.md (ADR 0002 §1).');
    }
    return impl;
  }

  ns.project = {
    /** @param {{x:number,y:number}} screenPt @param {object} [camera] @returns {{x:number,y:number}} */
    worldFromScreen(screenPt, camera) {
      return getCamera().worldFromScreen(screenPt, camera);
    },

    /** @param {{x:number,y:number}} worldPt @param {object} [camera] @returns {{x:number,y:number}} */
    screenFromWorld(worldPt, camera) {
      return getCamera().screenFromWorld(worldPt, camera);
    }
  };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

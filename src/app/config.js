(function (LaserCAD) {
  'use strict';

  const defaults = Object.freeze({
    documentBounds: { w: 128, h: 128 },
    initialZoomPxPerMm: 4,
    zoomMin: 0.01,
    zoomMax: 1000,
    commandHistoryCap: 50,
    historyStackCap: 200,
    snapTolerancePx: 8,
    coordinateDecimals: 3
  });

  LaserCAD.app.config = {
    defaults: defaults,

    /** @returns {object} */
    load() { return defaults; },

    /** @param {string} key @returns {*} */
    get(key) { return defaults[key]; }
  };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

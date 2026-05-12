(function (LaserCAD) {
  'use strict';

  const ns = LaserCAD.core.geometry;

  /** @typedef {{type:'circle', center:{x:number,y:number}, r:number}} Circle */

  ns.circle = {
    /** @param {{x:number,y:number}} center @param {number} r @returns {Circle} */
    make(center, r) {
      if (!(r > 0)) throw new Error('[LaserCAD] circle.make: r must be > 0, got ' + r);
      return { type: 'circle', center: { x: center.x, y: center.y }, r: r };
    },

    /** @param {Circle} c @returns {{minX:number,minY:number,maxX:number,maxY:number}} */
    bbox(c) {
      return {
        minX: c.center.x - c.r,
        minY: c.center.y - c.r,
        maxX: c.center.x + c.r,
        maxY: c.center.y + c.r
      };
    },

    /** @param {Circle} c @returns {number} */
    circumference(c) { return 2 * Math.PI * c.r; }
  };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

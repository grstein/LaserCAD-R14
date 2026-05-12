(function (LaserCAD) {
  'use strict';

  const ns = LaserCAD.core.geometry;

  /** @typedef {{type:'line', p1:{x:number,y:number}, p2:{x:number,y:number}}} Line */

  ns.line = {
    /** @param {{x:number,y:number}} p1 @param {{x:number,y:number}} p2 @returns {Line} */
    make(p1, p2) { return { type: 'line', p1: { x: p1.x, y: p1.y }, p2: { x: p2.x, y: p2.y } }; },

    /** @param {Line} l @returns {number} */
    length(l) {
      const dx = l.p2.x - l.p1.x, dy = l.p2.y - l.p1.y;
      return Math.sqrt(dx * dx + dy * dy);
    },

    /** @param {Line} l @returns {{x:number,y:number}} */
    direction(l) {
      return ns.vec2.normalize(ns.vec2.sub(l.p2, l.p1));
    },

    /** @param {Line} l @returns {{x:number,y:number}} */
    midpoint(l) {
      return { x: (l.p1.x + l.p2.x) * 0.5, y: (l.p1.y + l.p2.y) * 0.5 };
    },

    /** @param {Line} l @returns {{minX:number,minY:number,maxX:number,maxY:number}} */
    bbox(l) {
      return {
        minX: Math.min(l.p1.x, l.p2.x),
        minY: Math.min(l.p1.y, l.p2.y),
        maxX: Math.max(l.p1.x, l.p2.x),
        maxY: Math.max(l.p1.y, l.p2.y)
      };
    }
  };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

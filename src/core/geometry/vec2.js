(function (LaserCAD) {
  'use strict';

  const ns = LaserCAD.core.geometry;

  /** @typedef {{x:number, y:number}} Vec2 */

  ns.vec2 = {
    /** @param {number} x @param {number} y @returns {Vec2} */
    make(x, y) { return { x: x, y: y }; },

    /** @param {Vec2} a @param {Vec2} b @returns {Vec2} */
    add(a, b) { return { x: a.x + b.x, y: a.y + b.y }; },

    /** @param {Vec2} a @param {Vec2} b @returns {Vec2} */
    sub(a, b) { return { x: a.x - b.x, y: a.y - b.y }; },

    /** @param {Vec2} a @param {number} k @returns {Vec2} */
    scale(a, k) { return { x: a.x * k, y: a.y * k }; },

    /** @param {Vec2} a @param {Vec2} b @returns {number} */
    dot(a, b) { return a.x * b.x + a.y * b.y; },

    /** @param {Vec2} a @param {Vec2} b @returns {number} */
    cross(a, b) { return a.x * b.y - a.y * b.x; },

    /** @param {Vec2} a @returns {number} */
    lenSq(a) { return a.x * a.x + a.y * a.y; },

    /** @param {Vec2} a @returns {number} */
    len(a) { return Math.sqrt(a.x * a.x + a.y * a.y); },

    /** @param {Vec2} a @returns {Vec2} */
    normalize(a) {
      const lsq = a.x * a.x + a.y * a.y;
      if (lsq <= ns.epsilon.EPS * ns.epsilon.EPS) return { x: 0, y: 0 };
      const k = 1 / Math.sqrt(lsq);
      return { x: a.x * k, y: a.y * k };
    },

    /** @param {Vec2} a @param {Vec2} b @param {number} [tol] @returns {boolean} */
    equals(a, b, tol) {
      const e = ns.epsilon;
      return e.eq(a.x, b.x, tol) && e.eq(a.y, b.y, tol);
    }
  };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

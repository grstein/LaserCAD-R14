const TWO_PI = Math.PI * 2;

/** @typedef {{type:'arc', center:{x:number,y:number}, r:number, startAngle:number, endAngle:number, ccw:boolean}} Arc */

function normalizeAngle(a) {
  let v = a % TWO_PI;
  if (v < 0) v += TWO_PI;
  return v;
}

export const arc = {
  /**
   * @param {{x:number,y:number}} center
   * @param {number} r
   * @param {number} startAngle radians
   * @param {number} endAngle radians
   * @param {boolean} [ccw=true]
   * @returns {Arc}
   */
  make(center, r, startAngle, endAngle, ccw) {
    if (!(r > 0)) throw new Error('[LaserCAD] arc.make: r must be > 0');
    return {
      type: 'arc',
      center: { x: center.x, y: center.y },
      r: r,
      startAngle: startAngle,
      endAngle: endAngle,
      ccw: ccw !== false,
    };
  },

  /**
   * @param {Arc} arc
   * @param {number} theta radians
   * @returns {boolean}
   */
  containsAngle(arc, theta) {
    const s = normalizeAngle(arc.startAngle);
    const e = normalizeAngle(arc.endAngle);
    const t = normalizeAngle(theta);
    if (arc.ccw) {
      return s <= e ? t >= s && t <= e : t >= s || t <= e;
    } else {
      return s >= e ? t <= s && t >= e : t <= s || t >= e;
    }
  },

  /** @param {Arc} a @returns {{minX:number,minY:number,maxX:number,maxY:number}} */
  bbox(a) {
    const cx = a.center.x,
      cy = a.center.y,
      r = a.r;
    const sx = cx + r * Math.cos(a.startAngle),
      sy = cy + r * Math.sin(a.startAngle);
    const ex = cx + r * Math.cos(a.endAngle),
      ey = cy + r * Math.sin(a.endAngle);
    let minX = Math.min(sx, ex),
      maxX = Math.max(sx, ex);
    let minY = Math.min(sy, ey),
      maxY = Math.max(sy, ey);
    const cardinals = [
      { angle: 0, point: { x: cx + r, y: cy } },
      { angle: Math.PI / 2, point: { x: cx, y: cy + r } },
      { angle: Math.PI, point: { x: cx - r, y: cy } },
      { angle: (3 * Math.PI) / 2, point: { x: cx, y: cy - r } },
    ];
    for (let i = 0; i < cardinals.length; i++) {
      if (arc.containsAngle(a, cardinals[i].angle)) {
        minX = Math.min(minX, cardinals[i].point.x);
        maxX = Math.max(maxX, cardinals[i].point.x);
        minY = Math.min(minY, cardinals[i].point.y);
        maxY = Math.max(maxY, cardinals[i].point.y);
      }
    }
    return { minX: minX, minY: minY, maxX: maxX, maxY: maxY };
  },

  /** @param {Arc} arc @returns {{start:{x:number,y:number}, end:{x:number,y:number}}} */
  endpoints(arc) {
    const cx = arc.center.x,
      cy = arc.center.y,
      r = arc.r;
    return {
      start: { x: cx + r * Math.cos(arc.startAngle), y: cy + r * Math.sin(arc.startAngle) },
      end: { x: cx + r * Math.cos(arc.endAngle), y: cy + r * Math.sin(arc.endAngle) },
    };
  },
};

function isFiniteNumber(n) {
  return typeof n === 'number' && isFinite(n);
}

export const validators = {
  /** @param {*} p */
  isFinitePoint(p) {
    return !!p && isFiniteNumber(p.x) && isFiniteNumber(p.y);
  },

  /** @param {number} n */
  assertMm(n) {
    if (!isFiniteNumber(n)) throw new Error('[LaserCAD] assertMm: not finite number, got ' + n);
    return n;
  },

  /** @param {object} doc */
  isValidDoc(doc) {
    if (!doc || typeof doc !== 'object') return false;
    if (doc.schemaVersion !== 1) return false;
    if (doc.units !== 'mm') return false;
    if (!doc.documentBounds || !(doc.documentBounds.w > 0) || !(doc.documentBounds.h > 0))
      return false;
    if (!Array.isArray(doc.entities)) return false;
    if (!Array.isArray(doc.selection)) return false;
    return true;
  },
};

const EPS = 1e-9;
const SNAP_TOLERANCE_PX = 8;

export const epsilon = {
  EPS,
  SNAP_TOLERANCE_PX,

  /** @param {number} a @param {number} b @param {number} [tol] */
  eq(a, b, tol) {
    return Math.abs(a - b) <= (tol == null ? EPS : tol);
  },
  /** @param {number} a @param {number} b @param {number} [tol] */
  lt(a, b, tol) {
    return a < b - (tol == null ? EPS : tol);
  },
  /** @param {number} a @param {number} b @param {number} [tol] */
  gt(a, b, tol) {
    return a > b + (tol == null ? EPS : tol);
  },
  /** @param {number} a @param {number} b @param {number} [tol] */
  lte(a, b, tol) {
    return a <= b + (tol == null ? EPS : tol);
  },
  /** @param {number} a @param {number} b @param {number} [tol] */
  gte(a, b, tol) {
    return a >= b - (tol == null ? EPS : tol);
  },
};

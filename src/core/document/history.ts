const CAP = 200;

export const history = {
  /** @returns {{past:Array, future:Array}} */
  create() {
    return { past: [], future: [] };
  },

  /** @param {{past:Array, future:Array}} h @param {object} cmd */
  push(h, cmd) {
    h.past.push(cmd);
    if (h.past.length > CAP) h.past.shift();
    h.future.length = 0;
  },

  /** @param {{past:Array, future:Array}} h @returns {object|null} */
  undo(h) {
    const cmd = h.past.pop();
    if (cmd) h.future.push(cmd);
    return cmd || null;
  },

  /** @param {{past:Array, future:Array}} h @returns {object|null} */
  redo(h) {
    const cmd = h.future.pop();
    if (cmd) h.past.push(cmd);
    return cmd || null;
  },

  canUndo(h) {
    return h.past.length > 0;
  },
  canRedo(h) {
    return h.future.length > 0;
  },
};

(function (LaserCAD) {
  'use strict';

  const ns = LaserCAD.core.document;

  let idCounter = 0;
  function nextId() { idCounter += 1; return 'e_' + idCounter; }

  /**
   * @typedef {{type:string, do:(state:object)=>void, undo:(state:object)=>void, meta?:object}} Command
   */

  ns.commands = {
    /** @returns {string} */
    nextId: nextId,

    resetIdCounter() { idCounter = 0; },

    /**
     * @param {{type:string, do:Function, undo:Function, meta?:object}} cmd
     * @returns {Command}
     */
    make(cmd) {
      if (!cmd || typeof cmd.do !== 'function' || typeof cmd.undo !== 'function') {
        throw new Error('[LaserCAD] commands.make: cmd must have do and undo functions');
      }
      return { type: cmd.type || 'unknown', do: cmd.do, undo: cmd.undo, meta: cmd.meta || {} };
    },

    /** No-op command, useful for testing the pipeline. */
    noop() {
      return ns.commands.make({
        type: 'noop',
        do: function () { /* nothing */ },
        undo: function () { /* nothing */ }
      });
    },

    /**
     * Adiciona uma entidade ao documento.
     * @param {object} entity sem id; id é atribuído no do.
     * @returns {Command}
     */
    addEntity(entity) {
      const id = nextId();
      const stored = Object.assign({}, entity, { id: id });
      return ns.commands.make({
        type: 'addEntity',
        meta: { id: id, entity: stored },
        do: function (state) {
          state.entities.push(stored);
        },
        undo: function (state) {
          const idx = state.entities.findIndex(function (e) { return e.id === id; });
          if (idx >= 0) state.entities.splice(idx, 1);
          const selIdx = state.selection.indexOf(id);
          if (selIdx >= 0) state.selection.splice(selIdx, 1);
        }
      });
    },

    /**
     * Remove uma entidade por id.
     * @param {string} id
     * @returns {Command}
     */
    removeEntity(id) {
      let stored = null, storedIdx = -1;
      return ns.commands.make({
        type: 'removeEntity',
        meta: { id: id },
        do: function (state) {
          storedIdx = state.entities.findIndex(function (e) { return e.id === id; });
          if (storedIdx >= 0) {
            stored = state.entities[storedIdx];
            state.entities.splice(storedIdx, 1);
          }
        },
        undo: function (state) {
          if (stored && storedIdx >= 0) state.entities.splice(storedIdx, 0, stored);
        }
      });
    },

    /**
     * @param {{cx:number, cy:number, zoom:number}} next
     * @returns {Command}
     */
    setCamera(next) {
      if (typeof next.cx !== 'number' || typeof next.cy !== 'number' || typeof next.zoom !== 'number') {
        throw new Error('[LaserCAD] commands.setCamera: invalid payload');
      }
      let prev = null;
      return ns.commands.make({
        type: 'setCamera',
        meta: { next: next },
        do: function (state) {
          prev = { cx: state.camera.cx, cy: state.camera.cy, zoom: state.camera.zoom };
          state.camera.cx = next.cx;
          state.camera.cy = next.cy;
          state.camera.zoom = next.zoom;
        },
        undo: function (state) {
          if (!prev) return;
          state.camera.cx = prev.cx;
          state.camera.cy = prev.cy;
          state.camera.zoom = prev.zoom;
        }
      });
    }
  };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

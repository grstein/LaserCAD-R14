let idCounter = 0;
function nextId() {
  idCounter += 1;
  return 'e_' + idCounter;
}
function isFinitePoint(p) {
  return p && isFinite(p.x) && isFinite(p.y);
}
function pointEquals(a, b) {
  return Math.abs(a.x - b.x) < 1e-9 && Math.abs(a.y - b.y) < 1e-9;
}

/**
 * @typedef {{type:string, do:(state:object)=>void, undo:(state:object)=>void, meta?:object}} Command
 */

export const commands = {
  /** @returns {string} */
  nextId: nextId,

  resetIdCounter() {
    idCounter = 0;
  },

  seedIdCounter(n) {
    if (Number.isFinite(n) && n > idCounter) idCounter = n;
  },

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
    return commands.make({
      type: 'noop',
      do: function () {
        /* nothing */
      },
      undo: function () {
        /* nothing */
      },
    });
  },

  /**
   * Adiciona uma entidade ao documento.
   * @param {object} entity sem id; id é atribuído no do.
   * @returns {Command}
   */
  addEntity(entity) {
    if (!entity || !entity.type)
      throw new Error('[LaserCAD] commands.addEntity: entity.type required');
    if (entity.type === 'line') {
      if (!isFinitePoint(entity.p1) || !isFinitePoint(entity.p2))
        throw new Error('[LaserCAD] addEntity line: non-finite points');
      if (pointEquals(entity.p1, entity.p2))
        throw new Error('[LaserCAD] addEntity line: zero-length');
    } else if (entity.type === 'circle') {
      if (!isFinitePoint(entity.center) || !(entity.r > 0))
        throw new Error('[LaserCAD] addEntity circle: invalid');
    } else if (entity.type === 'arc') {
      if (
        !isFinitePoint(entity.center) ||
        !(entity.r > 0) ||
        !isFinite(entity.startAngle) ||
        !isFinite(entity.endAngle)
      )
        throw new Error('[LaserCAD] addEntity arc: invalid');
    }
    const id = nextId();
    const stored = Object.assign({}, entity, { id: id });
    return commands.make({
      type: 'addEntity',
      meta: { id: id, entity: stored },
      do: function (state) {
        state.entities.push(stored);
      },
      undo: function (state) {
        const idx = state.entities.findIndex(function (e) {
          return e.id === id;
        });
        if (idx >= 0) state.entities.splice(idx, 1);
        const selIdx = state.selection.indexOf(id);
        if (selIdx >= 0) state.selection.splice(selIdx, 1);
      },
    });
  },

  /**
   * Move um conjunto de entidades por dx,dy (mm).
   * @param {Array<string>} ids
   * @param {number} dx
   * @param {number} dy
   * @returns {Command}
   */
  moveEntities(ids, dx, dy) {
    const idSet = new Set(ids || []);
    return commands.make({
      type: 'moveEntities',
      meta: { ids: ids, dx: dx, dy: dy },
      do: function (state) {
        state.entities.forEach(function (e) {
          if (!idSet.has(e.id)) return;
          if (e.type === 'line') {
            e.p1.x += dx;
            e.p1.y += dy;
            e.p2.x += dx;
            e.p2.y += dy;
          } else if (e.type === 'circle' || e.type === 'arc') {
            e.center.x += dx;
            e.center.y += dy;
          }
        });
      },
      undo: function (state) {
        state.entities.forEach(function (e) {
          if (!idSet.has(e.id)) return;
          if (e.type === 'line') {
            e.p1.x -= dx;
            e.p1.y -= dy;
            e.p2.x -= dx;
            e.p2.y -= dy;
          } else if (e.type === 'circle' || e.type === 'arc') {
            e.center.x -= dx;
            e.center.y -= dy;
          }
        });
      },
    });
  },

  /**
   * Define a seleção (substitui).
   * @param {Array<string>} ids
   * @returns {Command}
   */
  setSelection(ids) {
    let prev = null;
    return commands.make({
      type: 'setSelection',
      meta: { ids: ids },
      do: function (state) {
        prev = state.selection.slice();
        state.selection.length = 0;
        (ids || []).forEach(function (id) {
          state.selection.push(id);
        });
      },
      undo: function (state) {
        if (!prev) return;
        state.selection.length = 0;
        prev.forEach(function (id) {
          state.selection.push(id);
        });
      },
    });
  },

  /**
   * Remove uma entidade por id.
   * @param {string} id
   * @returns {Command}
   */
  removeEntity(id) {
    let stored = null,
      storedIdx = -1;
    return commands.make({
      type: 'removeEntity',
      meta: { id: id },
      do: function (state) {
        storedIdx = state.entities.findIndex(function (e) {
          return e.id === id;
        });
        if (storedIdx >= 0) {
          stored = state.entities[storedIdx];
          state.entities.splice(storedIdx, 1);
        }
      },
      undo: function (state) {
        if (stored && storedIdx >= 0) state.entities.splice(storedIdx, 0, stored);
      },
    });
  },

  /**
   * @param {{cx:number, cy:number, zoom:number}} next
   * @returns {Command}
   */
  setCamera(next) {
    if (
      typeof next.cx !== 'number' ||
      typeof next.cy !== 'number' ||
      typeof next.zoom !== 'number'
    ) {
      throw new Error('[LaserCAD] commands.setCamera: invalid payload');
    }
    let prev = null;
    return commands.make({
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
      },
    });
  },
};

(function (LaserCAD) {
  'use strict';

  const bus = LaserCAD.bus;

  const state = {
    schemaVersion: 1,
    units: 'mm',
    documentBounds: { w: 128, h: 128 },
    entities: [],
    selection: [],
    camera: { cx: 0, cy: 0, zoom: 1, viewportW: 0, viewportH: 0 },
    activeTool: 'select',
    toolState: 'idle',
    cursor: { worldX: 0, worldY: 0, screenX: 0, screenY: 0 },
    toggles: { snap: true, grid: true, ortho: false },
    commandHistory: [],
    commandInput: '',

    init() { return state; },

    getState() { return state; },

    setCamera(next) {
      if (!next) return;
      if (typeof next.cx === 'number') state.camera.cx = next.cx;
      if (typeof next.cy === 'number') state.camera.cy = next.cy;
      if (typeof next.zoom === 'number') state.camera.zoom = next.zoom;
      bus.emit('camera:changed', { cx: state.camera.cx, cy: state.camera.cy, zoom: state.camera.zoom });
    },

    setViewportSize(size) {
      if (!size) return;
      state.camera.viewportW = size.w | 0;
      state.camera.viewportH = size.h | 0;
      bus.emit('viewport:resized', { w: state.camera.viewportW, h: state.camera.viewportH });
    },

    setCursor(c) {
      if (!c) return;
      state.cursor.worldX  = +c.worldX  || 0;
      state.cursor.worldY  = +c.worldY  || 0;
      state.cursor.screenX = +c.screenX || 0;
      state.cursor.screenY = +c.screenY || 0;
      bus.emit('cursor:moved', {
        worldX: state.cursor.worldX, worldY: state.cursor.worldY,
        screenX: state.cursor.screenX, screenY: state.cursor.screenY
      });
    },

    setActiveTool(toolId) {
      if (typeof toolId !== 'string') return;
      state.activeTool = toolId;
    },

    setToolState(name) {
      if (typeof name !== 'string') return;
      state.toolState = name;
    },

    setToggle(name, value) {
      if (!(name in state.toggles)) return;
      state.toggles[name] = !!value;
      bus.emit('toggle:changed', { name: name, value: state.toggles[name] });
    },

    setCommandInput(str) {
      state.commandInput = String(str == null ? '' : str);
    },

    pushCommandHistory(entry) {
      if (typeof entry !== 'string' || !entry) return;
      const cap = (LaserCAD.app.config && LaserCAD.app.config.get('commandHistoryCap')) || 50;
      state.commandHistory.push(entry);
      if (state.commandHistory.length > cap) state.commandHistory.shift();
    },

    setDocumentBounds(bounds) {
      if (!bounds || !(bounds.w > 0) || !(bounds.h > 0)) return;
      state.documentBounds.w = bounds.w;
      state.documentBounds.h = bounds.h;
    },

    applyCommand(cmd) {
      if (!cmd || typeof cmd.do !== 'function') {
        console.warn('[LaserCAD] state.applyCommand: invalid command', cmd);
        return;
      }
      try {
        cmd.do(state);
        if (LaserCAD.core.document.history) {
          historyInstance = historyInstance || LaserCAD.core.document.history.create();
          LaserCAD.core.document.history.push(historyInstance, cmd);
        }
      } catch (err) { console.error('[LaserCAD] command.do threw:', err); }
    },

    undo() {
      if (!historyInstance) return false;
      const cmd = LaserCAD.core.document.history.undo(historyInstance);
      if (!cmd) return false;
      try { cmd.undo(state); } catch (err) { console.error('[LaserCAD] cmd.undo threw:', err); }
      return true;
    },

    redo() {
      if (!historyInstance) return false;
      const cmd = LaserCAD.core.document.history.redo(historyInstance);
      if (!cmd) return false;
      try { cmd.do(state); } catch (err) { console.error('[LaserCAD] cmd.do (redo) threw:', err); }
      return true;
    },

    canUndo() { return !!(historyInstance && LaserCAD.core.document.history.canUndo(historyInstance)); },
    canRedo() { return !!(historyInstance && LaserCAD.core.document.history.canRedo(historyInstance)); },

    setSelection(ids) {
      if (!Array.isArray(ids)) return;
      state.selection.length = 0;
      ids.forEach(function (id) { state.selection.push(id); });
    }
  };

  let historyInstance = null;

  LaserCAD.app.state = state;
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

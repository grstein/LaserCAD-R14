(function (LaserCAD) {
  'use strict';

  const KEY = 'lasercad:r14:autosave';
  const DEBOUNCE_MS = 800;
  let timer = null;
  let lastSavedAt = null;

  function safeStorage() {
    try { return window.localStorage; } catch (e) { return null; }
  }

  function snapshot(state) {
    return {
      schemaVersion: state.schemaVersion,
      units: state.units,
      documentBounds: state.documentBounds,
      entities: state.entities,
      camera: state.camera,
      savedAt: Date.now()
    };
  }

  function scheduleSave() {
    if (timer) clearTimeout(timer);
    timer = setTimeout(saveNow, DEBOUNCE_MS);
  }

  function saveNow() {
    const ls = safeStorage();
    if (!ls) return;
    try {
      ls.setItem(KEY, JSON.stringify(snapshot(LaserCAD.app.state)));
      lastSavedAt = Date.now();
      LaserCAD.bus.emit('toggle:changed', { name: 'autosave', value: true });
    } catch (err) {
      console.warn('[LaserCAD] autosave failed:', err);
    }
  }

  function restore() {
    const ls = safeStorage();
    if (!ls) return false;
    const raw = ls.getItem(KEY);
    if (!raw) return false;
    try {
      const data = JSON.parse(raw);
      if (!data || data.schemaVersion !== 1) return false;
      const state = LaserCAD.app.state;
      if (Array.isArray(data.entities)) {
        state.entities.length = 0;
        data.entities.forEach(function (e) { state.entities.push(e); });
      }
      if (data.documentBounds) state.setDocumentBounds(data.documentBounds);
      if (data.camera) state.setCamera(data.camera);
      lastSavedAt = data.savedAt || Date.now();
      return true;
    } catch (err) {
      console.warn('[LaserCAD] autosave restore failed:', err);
      return false;
    }
  }

  function clear() {
    const ls = safeStorage();
    if (ls) ls.removeItem(KEY);
    lastSavedAt = null;
  }

  LaserCAD.io.autosave = {
    KEY: KEY,
    init() {
      LaserCAD.bus.on('command:submit', scheduleSave);
      // hook em applyCommand via wrapping: também salvamos em toggle:changed (afeta toggles persistidos)
      // mas evitamos save em camera:changed (muito frequente). Vamos disparar manualmente após commit.
      const origCommit = LaserCAD.tools.toolManager.commit;
      LaserCAD.tools.toolManager.commit = function (cmd) {
        origCommit.call(LaserCAD.tools.toolManager, cmd);
        scheduleSave();
      };
    },
    saveNow: saveNow,
    restore: restore,
    clear: clear,
    lastSavedAt() { return lastSavedAt; }
  };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

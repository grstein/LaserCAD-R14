import { bus } from '@/app/event-bus.js';
import { state } from '@/app/state.js';
import { commands } from '@/core/document/commands.js';
import { toolManager } from '@/tools/tool-manager.js';
import { isTauri, tauriStore } from '@/tauri-bridge.js';

const KEY = 'lasercad:r14:autosave';
const STORE_KEY = 'snapshot';
const DEBOUNCE_MS = 800;
let timer = null;
let lastSavedAt = null;

function safeStorage() {
  try {
    return window.localStorage;
  } catch (e) {
    return null;
  }
}

function snapshot(state) {
  return {
    schemaVersion: state.schemaVersion,
    units: state.units,
    documentBounds: state.documentBounds,
    entities: state.entities,
    camera: state.camera,
    savedAt: Date.now(),
  };
}

function scheduleSave() {
  if (timer) clearTimeout(timer);
  timer = setTimeout(saveNow, DEBOUNCE_MS);
}

async function saveNow() {
  const snap = snapshot(state);
  try {
    if (isTauri()) {
      const store = await tauriStore();
      if (store) {
        await store.set(STORE_KEY, snap);
        lastSavedAt = Date.now();
        bus.emit('toggle:changed', { name: 'autosave', value: true });
        return;
      }
    }
    const ls = safeStorage();
    if (!ls) return;
    ls.setItem(KEY, JSON.stringify(snap));
    lastSavedAt = Date.now();
    bus.emit('toggle:changed', { name: 'autosave', value: true });
  } catch (err) {
    console.warn('[LaserCAD] autosave failed:', err);
  }
}

async function restore() {
  let data = null;
  try {
    if (isTauri()) {
      const store = await tauriStore();
      if (store) {
        data = (await store.get(STORE_KEY)) ?? null;
      }
    }
    if (!data) {
      const ls = safeStorage();
      if (!ls) return false;
      const raw = ls.getItem(KEY);
      if (!raw) return false;
      data = JSON.parse(raw);
    }
    if (!data || data.schemaVersion !== 1) return false;
    if (Array.isArray(data.entities)) {
      state.entities.length = 0;
      data.entities.forEach(function (e) {
        state.entities.push(e);
      });
      let maxN = 0;
      for (let i = 0; i < state.entities.length; i++) {
        const m = /^e_(\d+)$/.exec(state.entities[i].id || '');
        if (m) {
          const n = parseInt(m[1], 10);
          if (n > maxN) maxN = n;
        }
      }
      commands.seedIdCounter(maxN);
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

async function clear() {
  try {
    if (isTauri()) {
      const store = await tauriStore();
      if (store) await store.set(STORE_KEY, null);
    }
    const ls = safeStorage();
    if (ls) ls.removeItem(KEY);
  } catch (err) {
    console.warn('[LaserCAD] autosave clear failed:', err);
  }
  lastSavedAt = null;
}

export const autosave = {
  KEY: KEY,
  init() {
    bus.on('command:submit', scheduleSave);
    // hook applyCommand via wrapping: we also save on toggle:changed (affects persisted toggles)
    // but avoid saving on camera:changed (too frequent). We trigger the save manually after commit.
    const origCommit = toolManager.commit;
    toolManager.commit = function (cmd) {
      origCommit.call(toolManager, cmd);
      scheduleSave();
    };
  },
  saveNow: saveNow,
  restore: restore,
  clear: clear,
  lastSavedAt() {
    return lastSavedAt;
  },
};

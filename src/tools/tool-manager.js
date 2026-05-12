(function (LaserCAD) {
  'use strict';

  const ns = LaserCAD.tools;
  const bus = LaserCAD.bus;
  const state = function () { return LaserCAD.app.state; };

  /** @type {Map<string, object>} */
  const registry = new Map();
  let activeId = 'select';

  function arm(toolId) {
    if (!registry.has(toolId)) {
      console.warn('[LaserCAD] tool-manager: tool not registered:', toolId);
      bus.emit('command:error', { raw: toolId, message: '! Tool not available: ' + toolId });
      return;
    }
    const prev = activeId;
    if (prev && prev !== toolId) {
      const prevDef = registry.get(prev);
      if (prevDef && typeof prevDef.onCancel === 'function') {
        try { prevDef.onCancel(state()); } catch (err) { console.error('[LaserCAD] tool onCancel threw:', err); }
      }
    }
    activeId = toolId;
    state().setActiveTool(toolId);
    state().setToolState('armed');
    const def = registry.get(toolId);
    if (def && typeof def.onArm === 'function') {
      try { def.onArm(state()); } catch (err) { console.error('[LaserCAD] tool onArm threw:', err); }
    }
    bus.emit('tool:armed', { toolId: toolId });
  }

  ns.toolManager = {
    init() {
      bus.on('tool:request', function (p) { if (p && p.toolId) arm(p.toolId); });
    },

    /** @param {string} toolId @param {{onArm?:Function,onCancel?:Function,onPointerDown?:Function,prompt?:string,id?:string}} def */
    register(toolId, def) {
      if (!toolId || !def) return;
      registry.set(toolId, def);
    },

    request(toolId) { arm(toolId); },

    cancel() {
      const def = registry.get(activeId);
      if (def && typeof def.onCancel === 'function') {
        try { def.onCancel(state()); } catch (err) { console.error('[LaserCAD] tool onCancel threw:', err); }
      }
      const cancelled = activeId;
      state().setToolState('cancel');
      bus.emit('tool:cancel', { toolId: cancelled });
      // Re-arm select to never leave the app toolless (R14 parity).
      if (registry.has('select') && cancelled !== 'select') {
        arm('select');
      } else {
        state().setToolState('idle');
      }
    },

    getActive() { return activeId; },
    getActiveDef() { return registry.get(activeId) || null; },
    getPrompt() {
      const def = registry.get(activeId);
      return (def && def.prompt) || 'Command:';
    }
  };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

(function (LaserCAD) {
  'use strict';

  const bus = LaserCAD.bus;

  const TOOL_KEYS = {
    l: 'line', p: 'polyline', r: 'rect', c: 'circle', a: 'arc',
    s: 'select', t: 'trim', e: 'extend', m: 'move'
  };

  function isTextEditingTarget(target) {
    if (!target) return false;
    const tag = (target.tagName || '').toUpperCase();
    if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return true;
    if (target.isContentEditable) return true;
    return false;
  }

  function onKeyDown(e) {
    const cmdLine = LaserCAD.ui.commandLine;
    if (cmdLine && typeof cmdLine.handleGlobalKey === 'function') {
      if (cmdLine.handleGlobalKey(e)) return;
    }

    if (isTextEditingTarget(e.target)) return;

    if ((e.ctrlKey || e.metaKey) && (e.key === 'z' || e.key === 'Z')) {
      e.preventDefault();
      if (LaserCAD.app.state.undo()) refreshAfterHistory();
      return;
    }
    if ((e.ctrlKey || e.metaKey) && (e.key === 'y' || e.key === 'Y')) {
      e.preventDefault();
      if (LaserCAD.app.state.redo()) refreshAfterHistory();
      return;
    }
    if ((e.ctrlKey || e.metaKey) && (e.key === 's' || e.key === 'S')) {
      e.preventDefault();
      if (LaserCAD.io && LaserCAD.io.exportSvg && LaserCAD.io.fileDownload) {
        const svg = LaserCAD.io.exportSvg.serialize(LaserCAD.app.state, { preset: 'cut' });
        LaserCAD.io.fileDownload.download('drawing.svg', svg);
      }
      return;
    }

    if (e.ctrlKey || e.metaKey || e.altKey) return;

    if (e.key === 'F1') {
      e.preventDefault();
      const menubar = LaserCAD.ui.menubar;
      if (LaserCAD.ui.dialogs) {
        LaserCAD.ui.dialogs.open({
          title: 'Keyboard shortcuts',
          body: 'See docs/atalhos.md or use Help menu.',
          actions: [{ label: 'Close', primary: true, onClick: function () { LaserCAD.ui.dialogs.close(); } }]
        });
      }
      return;
    }
    if (e.key === 'F3') { e.preventDefault(); toggle('snap'); return; }
    if (e.key === 'F7') { e.preventDefault(); toggle('grid'); return; }
    if (e.key === 'F8') { e.preventDefault(); toggle('ortho'); return; }

    if (e.key === 'Escape') {
      if (LaserCAD.tools && LaserCAD.tools.toolManager) LaserCAD.tools.toolManager.cancel();
      return;
    }

    if (e.key === 'Delete') {
      const state = LaserCAD.app.state;
      if (state.selection.length > 0) {
        const cmds = LaserCAD.core.document.commands;
        const ids = state.selection.slice();
        ids.forEach(function (id) { LaserCAD.tools.toolManager.commit(cmds.removeEntity(id)); });
        state.setSelection([]);
        const sr = LaserCAD.tools.toolManager.getSvgRoot();
        if (sr) LaserCAD.render.entityRenderers.renderAll(sr, state);
      }
      return;
    }

    const k = (e.key || '').toLowerCase();
    if (TOOL_KEYS[k]) {
      bus.emit('tool:request', { toolId: TOOL_KEYS[k] });
    }
  }

  function toggle(name) {
    const cur = LaserCAD.app.state.toggles[name];
    LaserCAD.app.state.setToggle(name, !cur);
  }

  function refreshAfterHistory() {
    const sr = LaserCAD.tools.toolManager && LaserCAD.tools.toolManager.getSvgRoot();
    if (sr) LaserCAD.render.entityRenderers.renderAll(sr, LaserCAD.app.state);
  }

  LaserCAD.app.shortcuts = {
    attach(target) {
      (target || window).addEventListener('keydown', onKeyDown);
    },
    detach(target) {
      (target || window).removeEventListener('keydown', onKeyDown);
    },
    TOOL_KEYS: TOOL_KEYS
  };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

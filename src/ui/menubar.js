(function (LaserCAD) {
  'use strict';

  const MENUS = [
    {
      label: 'File',
      items: [
        { label: 'New',           shortcut: 'Ctrl+N', action: function () {
          const state = LaserCAD.app.state;
          state.entities.length = 0; state.selection.length = 0;
          if (LaserCAD.io.autosave) LaserCAD.io.autosave.clear();
          refreshAll();
        } },
        { label: 'Save SVG (cut)…',     shortcut: 'Ctrl+S', action: function () { exportPreset('cut'); } },
        { label: 'Save SVG (mark)…',    action: function () { exportPreset('mark'); } },
        { label: 'Save SVG (engrave)…', action: function () { exportPreset('engrave'); } }
      ]
    },
    {
      label: 'Edit',
      items: [
        { label: 'Undo',  shortcut: 'Ctrl+Z', action: function () { if (LaserCAD.app.state.undo()) refreshAll(); } },
        { label: 'Redo',  shortcut: 'Ctrl+Y', action: function () { if (LaserCAD.app.state.redo()) refreshAll(); } },
        { label: 'Delete',shortcut: 'Del',    action: function () {
          const state = LaserCAD.app.state;
          if (state.selection.length === 0) return;
          const cmds = LaserCAD.core.document.commands;
          state.selection.slice().forEach(function (id) { LaserCAD.tools.toolManager.commit(cmds.removeEntity(id)); });
          state.setSelection([]); refreshAll();
        } }
      ]
    },
    {
      label: 'View',
      items: [
        { label: 'Zoom extents',  shortcut: 'Z E',  action: function () { LaserCAD.render.camera.zoomExtents(); } },
        { label: 'Toggle grid',   shortcut: 'F7',   action: function () { tog('grid'); } },
        { label: 'Toggle snap',   shortcut: 'F3',   action: function () { tog('snap'); } },
        { label: 'Toggle ortho',  shortcut: 'F8',   action: function () { tog('ortho'); } }
      ]
    },
    { label: 'Draw',   items: [
      { label: 'Line (L)',      shortcut: 'L', action: function () { req('line'); } },
      { label: 'Polyline (P)',  shortcut: 'P', action: function () { req('polyline'); } },
      { label: 'Rectangle (R)', shortcut: 'R', action: function () { req('rect'); } },
      { label: 'Circle (C)',    shortcut: 'C', action: function () { req('circle'); } },
      { label: 'Arc (A)',       shortcut: 'A', action: function () { req('arc'); } }
    ] },
    { label: 'Modify', items: [
      { label: 'Select (S)',  shortcut: 'S', action: function () { req('select'); } },
      { label: 'Move (M)',    shortcut: 'M', action: function () { req('move'); } },
      { label: 'Trim (T)',    shortcut: 'T', action: function () { req('trim'); } },
      { label: 'Extend (E)',  shortcut: 'E', action: function () { req('extend'); } }
    ] },
    { label: 'Help',   items: [
      { label: 'About LaserCAD R14', action: function () {
        LaserCAD.ui.dialogs.open({
          title: 'About LaserCAD R14',
          body: 'Micro-CAD 2D no navegador.\nMVP — Sprint 2 (Alpha Zero).\nUnit: mm | Wavelength: 450nm.',
          actions: [{ label: 'Close', primary: true, onClick: function () { LaserCAD.ui.dialogs.close(); } }]
        });
      } }
    ] }
  ];

  function req(toolId) { LaserCAD.bus.emit('tool:request', { toolId: toolId }); }
  function tog(name) {
    const cur = LaserCAD.app.state.toggles[name];
    LaserCAD.app.state.setToggle(name, !cur);
  }
  function refreshAll() {
    const sr = LaserCAD.tools.toolManager && LaserCAD.tools.toolManager.getSvgRoot();
    if (sr) LaserCAD.render.entityRenderers.renderAll(sr, LaserCAD.app.state);
  }
  function exportPreset(preset) {
    if (!(LaserCAD.io && LaserCAD.io.exportSvg && LaserCAD.io.fileDownload)) return;
    const svg = LaserCAD.io.exportSvg.serialize(LaserCAD.app.state, { preset: preset });
    LaserCAD.io.fileDownload.download('drawing.' + preset + '.svg', svg);
  }

  let openDropdown = null;
  function closeAll() {
    if (openDropdown && openDropdown.parentNode) openDropdown.parentNode.removeChild(openDropdown);
    openDropdown = null;
  }
  document.addEventListener('click', function (e) {
    if (openDropdown && !openDropdown.contains(e.target)) closeAll();
  });
  document.addEventListener('keydown', function (e) { if (e.key === 'Escape') closeAll(); });

  LaserCAD.ui.menubar = {
    mount(hostSelector) {
      const host = (typeof hostSelector === 'string') ? document.querySelector(hostSelector) : hostSelector;
      if (!host) throw new Error('[LaserCAD] menubar.mount: host not found');

      MENUS.forEach(function (menu) {
        const item = document.createElement('span');
        item.className = 'menu-item';
        item.textContent = menu.label;
        item.addEventListener('click', function (e) {
          e.stopPropagation();
          if (openDropdown && openDropdown.parentNode === item) { closeAll(); return; }
          closeAll();
          const dd = document.createElement('div');
          dd.className = 'menu-dropdown';
          menu.items.forEach(function (row) {
            const r = document.createElement('div');
            r.className = 'menu-row' + (row.disabled ? ' disabled' : '');
            r.innerHTML = '<span>' + row.label + '</span>' + (row.shortcut ? '<span class="shortcut">' + row.shortcut + '</span>' : '');
            if (!row.disabled && typeof row.action === 'function') {
              r.addEventListener('click', function () { row.action(); closeAll(); });
            }
            dd.appendChild(r);
          });
          item.appendChild(dd);
          openDropdown = dd;
        });
        host.appendChild(item);
      });
    }
  };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

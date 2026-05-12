(function (LaserCAD) {
  'use strict';

  const MENUS = [
    {
      label: 'File',
      items: [
        { label: 'New',         shortcut: 'Ctrl+N', disabled: true },
        { label: 'Open…',       shortcut: 'Ctrl+O', disabled: true },
        { label: 'Save SVG…',   shortcut: 'Ctrl+S', disabled: true }
      ]
    },
    {
      label: 'Edit',
      items: [
        { label: 'Undo',        shortcut: 'Ctrl+Z', disabled: true },
        { label: 'Redo',        shortcut: 'Ctrl+Y', disabled: true },
        { label: 'Delete',      shortcut: 'Del',    disabled: true }
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
      { label: 'Line (L)',     action: function () { req('line'); } },
      { label: 'Polyline (P)', action: function () { req('polyline'); } },
      { label: 'Rectangle (R)',action: function () { req('rect'); } },
      { label: 'Circle (C)',   action: function () { req('circle'); } },
      { label: 'Arc (A)',      action: function () { req('arc'); } }
    ] },
    { label: 'Modify', items: [
      { label: 'Select (S)',  action: function () { req('select'); } },
      { label: 'Move (M)',    disabled: true },
      { label: 'Trim (T)',    disabled: true },
      { label: 'Extend (E)',  disabled: true }
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

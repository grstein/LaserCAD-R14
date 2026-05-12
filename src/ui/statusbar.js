(function (LaserCAD) {
  'use strict';

  const bus = LaserCAD.bus;

  LaserCAD.ui.statusbar = {
    mount(hostSelector) {
      const host = (typeof hostSelector === 'string') ? document.querySelector(hostSelector) : hostSelector;
      if (!host) throw new Error('[LaserCAD] statusbar.mount: host not found');

      const coords = document.createElement('span');
      coords.className = 'sb-coords sb-slot';
      coords.textContent = '0.000, 0.000  mm';

      const div1 = document.createElement('span'); div1.className = 'sb-divider'; div1.textContent = '│';
      const div2 = document.createElement('span'); div2.className = 'sb-divider'; div2.textContent = '│';
      const div3 = document.createElement('span'); div3.className = 'sb-divider'; div3.textContent = '│';

      const snap  = makeToggle('snap',  'SNAP');
      const grid  = makeToggle('grid',  'GRID');
      const ortho = makeToggle('ortho', 'ORTHO');

      const autosave = document.createElement('span');
      autosave.className = 'sb-autosave';
      autosave.textContent = '● not yet';
      function refreshAutosave() {
        const at = LaserCAD.io && LaserCAD.io.autosave && LaserCAD.io.autosave.lastSavedAt();
        if (!at) { autosave.textContent = '● not yet'; return; }
        const sec = Math.max(1, Math.round((Date.now() - at) / 1000));
        autosave.textContent = '● saved ' + sec + 's ago';
      }
      setInterval(refreshAutosave, 1000);

      host.appendChild(coords);
      host.appendChild(div1);
      host.appendChild(snap.el);
      host.appendChild(div2);
      host.appendChild(grid.el);
      host.appendChild(div3);
      host.appendChild(ortho.el);
      host.appendChild(autosave);

      syncToggles();

      bus.on('cursor:moved', function (p) {
        coords.textContent = p.worldX.toFixed(3) + ', ' + p.worldY.toFixed(3) + '  mm';
      });
      bus.on('toggle:changed', syncToggles);
      bus.on('app:ready', syncToggles);

      function makeToggle(name, label) {
        const el = document.createElement('span');
        el.className = 'sb-toggle sb-slot';
        el.dataset.toggle = name;
        el.innerHTML = '<span class="dot">○</span> ' + label;
        el.addEventListener('click', function () {
          const cur = LaserCAD.app.state.toggles[name];
          LaserCAD.app.state.setToggle(name, !cur);
        });
        return { el: el, name: name };
      }

      function syncToggles() {
        host.querySelectorAll('.sb-toggle').forEach(function (el) {
          const name = el.dataset.toggle;
          const on = !!LaserCAD.app.state.toggles[name];
          el.classList.toggle('on', on);
          el.firstChild.textContent = on ? '◉' : '○';
        });
      }
    }
  };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

(function (LaserCAD) {
  'use strict';

  const bus = LaserCAD.bus;

  const TOOLS = [
    { id: 'line',     label: 'Line (L)',       glyph: '<line x1="3" y1="15" x2="15" y2="3"/><rect x="2" y="14" width="2" height="2"/><rect x="14" y="2" width="2" height="2"/>' },
    { id: 'polyline', label: 'Polyline (P)',   glyph: '<polyline points="2,15 6,5 11,11 16,3"/>' },
    { id: 'rect',     label: 'Rectangle (R)',  glyph: '<rect x="3" y="3" width="12" height="12"/>' },
    { id: 'circle',   label: 'Circle (C)',     glyph: '<circle cx="9" cy="9" r="6"/><circle cx="9" cy="9" r="0.6" fill="currentColor"/>' },
    { id: 'arc',      label: 'Arc (A)',        glyph: '<path d="M3 14 A 8 8 0 0 1 15 14"/><rect x="2" y="13" width="2" height="2"/><rect x="14" y="13" width="2" height="2"/>' },
    { id: 'select',   label: 'Select (S)',     glyph: '<path d="M3 3 L 12 8 L 8 9 L 12 14 L 10 15 L 7 10 L 4 12 Z"/>' },
    { id: 'trim',     label: 'Trim (T)',       glyph: '<line x1="3" y1="3" x2="15" y2="15"/><line x1="15" y1="3" x2="3" y2="15"/>' },
    { id: 'extend',   label: 'Extend (E)',     glyph: '<line x1="3" y1="9" x2="11" y2="9"/><polyline points="13,5 17,9 13,13"/>' },
    { id: 'move',     label: 'Move (M)',       glyph: '<line x1="9" y1="3" x2="9" y2="15"/><line x1="3" y1="9" x2="15" y2="9"/><polyline points="9,3 7,5"/><polyline points="9,3 11,5"/><polyline points="9,15 7,13"/><polyline points="9,15 11,13"/><polyline points="3,9 5,7"/><polyline points="3,9 5,11"/><polyline points="15,9 13,7"/><polyline points="15,9 13,11"/>' },
    { id: 'delete',   label: 'Delete (Del)',   glyph: '<line x1="4" y1="4" x2="14" y2="14"/><line x1="14" y1="4" x2="4" y2="14"/>' }
  ];

  LaserCAD.ui.toolbar = {
    mount(hostSelector) {
      const host = (typeof hostSelector === 'string') ? document.querySelector(hostSelector) : hostSelector;
      if (!host) throw new Error('[LaserCAD] toolbar.mount: host not found');

      const buttons = TOOLS.map(function (t, i) {
        const btn = document.createElement('button');
        btn.className = 'tool-button' + (t.id === 'select' ? ' active' : ' disabled');
        btn.title = t.label;
        btn.dataset.tool = t.id;
        btn.innerHTML = '<svg viewBox="0 0 18 18" aria-hidden="true">' + t.glyph + '</svg>';
        btn.addEventListener('click', function () { bus.emit('tool:request', { toolId: t.id }); });
        host.appendChild(btn);
        if (i === 4) {
          const sep = document.createElement('div');
          sep.style.height = '1px';
          sep.style.background = 'var(--border-subtle)';
          sep.style.margin = '4px 0';
          host.appendChild(sep);
        }
        return btn;
      });

      bus.on('tool:armed', function (p) {
        buttons.forEach(function (b) { b.classList.toggle('active', b.dataset.tool === p.toolId); });
      });
    }
  };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

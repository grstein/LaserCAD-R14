(function (LaserCAD) {
  'use strict';

  const bus = LaserCAD.bus;

  const TOOL_ALIASES = {
    l: 'line', line: 'line', p: 'polyline', polyline: 'polyline',
    r: 'rect', rect: 'rect', c: 'circle', circle: 'circle',
    a: 'arc',  arc: 'arc',
    s: 'select', select: 'select',
    t: 'trim', trim: 'trim',
    e: 'extend', extend: 'extend',
    m: 'move', move: 'move',
    delete: 'delete', del: 'delete'
  };
  const TOGGLE_CMDS = { snap: 'snap', grid: 'grid', ortho: 'ortho' };

  let inputEl = null;
  let historyEl = null;
  let promptEl = null;
  let echoEl = null;
  let historyIndex = -1;

  function renderActiveLine() {
    const prompt = (LaserCAD.tools.toolManager && LaserCAD.tools.toolManager.getPrompt()) || 'Command:';
    promptEl.innerHTML = '<span class="cmd-prompt">' + prompt + '</span>';
    promptEl.appendChild(inputEl);
  }

  function echo(text, isError) {
    echoEl.textContent = text;
    echoEl.classList.toggle('error', !!isError);
  }

  function execute(raw) {
    if (!raw) return;
    const trimmed = raw.trim().toLowerCase();
    LaserCAD.app.state.pushCommandHistory(raw);

    if (trimmed === 'pan') { echo('PAN: hold middle button or Space+drag'); return; }
    if (trimmed === 'zoom in')  { LaserCAD.render.camera.zoomAt({ x: window.innerWidth/2, y: window.innerHeight/2 }, 1.25); echo(''); return; }
    if (trimmed === 'zoom out') { LaserCAD.render.camera.zoomAt({ x: window.innerWidth/2, y: window.innerHeight/2 }, 1/1.25); echo(''); return; }
    if (trimmed === 'zoom extents' || trimmed === 'z e' || trimmed === 'ze') { LaserCAD.render.camera.zoomExtents(); echo(''); return; }

    if (TOGGLE_CMDS[trimmed]) {
      const name = TOGGLE_CMDS[trimmed];
      const cur = LaserCAD.app.state.toggles[name];
      LaserCAD.app.state.setToggle(name, !cur);
      echo(name + ': ' + (!cur ? 'ON' : 'OFF'));
      return;
    }

    if (TOOL_ALIASES[trimmed]) {
      bus.emit('tool:request', { toolId: TOOL_ALIASES[trimmed] });
      echo('');
      return;
    }

    // Coordenadas absolutas X,Y
    let m = trimmed.match(/^(-?\d+(?:\.\d+)?)\s*,\s*(-?\d+(?:\.\d+)?)$/);
    if (m) {
      bus.emit('command:submit', { raw: raw, parsed: { kind: 'absolute', x: parseFloat(m[1]), y: parseFloat(m[2]) } });
      return;
    }
    // Coordenadas relativas @X,Y
    m = trimmed.match(/^@\s*(-?\d+(?:\.\d+)?)\s*,\s*(-?\d+(?:\.\d+)?)$/);
    if (m) {
      bus.emit('command:submit', { raw: raw, parsed: { kind: 'relative', dx: parseFloat(m[1]), dy: parseFloat(m[2]) } });
      return;
    }
    // Distância simples (após primeiro ponto)
    m = trimmed.match(/^(-?\d+(?:\.\d+)?)$/);
    if (m) {
      bus.emit('command:submit', { raw: raw, parsed: { kind: 'distance', value: parseFloat(m[1]) } });
      return;
    }

    bus.emit('command:error', { raw: raw, message: '! Unknown command: ' + raw });
  }

  LaserCAD.ui.commandLine = {
    mount(hostSelector) {
      const host = (typeof hostSelector === 'string') ? document.querySelector(hostSelector) : hostSelector;
      if (!host) throw new Error('[LaserCAD] commandLine.mount: host not found');

      historyEl = document.createElement('div');
      promptEl  = document.createElement('div');
      echoEl    = document.createElement('div');
      historyEl.className = 'cmd-line';
      promptEl.className  = 'cmd-line active';
      echoEl.className    = 'cmd-line';
      host.appendChild(historyEl);
      host.appendChild(echoEl);
      host.appendChild(promptEl);

      inputEl = document.createElement('input');
      inputEl.className = 'cmd-input';
      inputEl.spellcheck = false;
      inputEl.autocomplete = 'off';
      inputEl.addEventListener('input', function () {
        LaserCAD.app.state.setCommandInput(inputEl.value);
      });
      inputEl.addEventListener('keydown', function (e) {
        const history = LaserCAD.app.state.commandHistory;
        if (e.key === 'Enter' || e.key === ' ' && inputEl.value === '') {
          e.preventDefault();
          const raw = inputEl.value;
          historyEl.textContent = raw ? '> ' + raw : historyEl.textContent;
          inputEl.value = '';
          historyIndex = -1;
          execute(raw);
          LaserCAD.app.state.setCommandInput('');
        } else if (e.key === 'Escape') {
          e.preventDefault();
          inputEl.value = '';
          LaserCAD.app.state.setCommandInput('');
          if (LaserCAD.tools.toolManager) LaserCAD.tools.toolManager.cancel();
        } else if (e.key === 'ArrowUp') {
          if (history.length === 0) return;
          e.preventDefault();
          if (historyIndex === -1) historyIndex = history.length - 1;
          else if (historyIndex > 0) historyIndex--;
          inputEl.value = history[historyIndex] || '';
        } else if (e.key === 'ArrowDown') {
          if (history.length === 0) return;
          e.preventDefault();
          if (historyIndex === -1) return;
          historyIndex++;
          if (historyIndex >= history.length) { historyIndex = -1; inputEl.value = ''; }
          else inputEl.value = history[historyIndex];
        }
      });

      renderActiveLine();
      bus.on('tool:armed', renderActiveLine);
      bus.on('tool:cancel', renderActiveLine);
      bus.on('command:error', function (p) { if (p && p.message) echo(p.message, true); });
      bus.on('app:ready', function () { setTimeout(function () { inputEl.focus(); }, 0); });
    },

    /**
     * Foco automático em qualquer alfanumérico fora de input (design.md L206).
     * Chamado pelo shortcuts.js antes do dispatch normal.
     * @param {KeyboardEvent} e
     * @returns {boolean} true se consumiu o evento
     */
    handleGlobalKey(e) {
      if (!inputEl) return false;
      if (document.activeElement === inputEl) return false;
      const tag = (e.target && e.target.tagName || '').toUpperCase();
      if (tag === 'INPUT' || tag === 'TEXTAREA' || (e.target && e.target.isContentEditable)) return false;
      if (e.ctrlKey || e.metaKey || e.altKey) return false;
      if (e.key && e.key.length === 1 && /[a-zA-Z0-9 ]/.test(e.key)) {
        inputEl.focus();
        return false;
      }
      return false;
    }
  };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

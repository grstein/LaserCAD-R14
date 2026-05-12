(function (LaserCAD) {
  'use strict';

  /*
   * Bootstrap order — ADR 0002 §2.3. Do not reorder without ADR.
   *
   *  1. validateNamespacePresence()
   *  2. state.init()
   *  3. config.load()
   *  4. validateDomHosts()
   *  5. svgRoot.mount('#viewport-host')
   *  6. state.setViewportSize(svgRoot.getSize())
   *  7. render.camera.attach(svgRoot)
   *  8. render.grid.mount(svgRoot)
   *  9. render.overlays.mount(svgRoot)
   * 10. ui.menubar.mount('#menubar-host')
   * 11. ui.toolbar.mount('#toolbar-host')
   * 12. ui.commandLine.mount('#commandline-host')
   * 13. ui.statusbar.mount('#statusbar-host')
   * 14. ui.dialogs.init()
   * 15. tools.toolManager.init()
   * 16. tools.toolManager.register('select', tools.selectTool)
   * 17. app.shortcuts.attach(window)
   * 18. wireResizeObserver(svgRoot)
   * 19. svgRoot.refreshViewBox()  -- first paint with valid size
   * 20. bus.emit('app:ready', {})
   */

  const NS_REQUIRED = [
    'core.geometry.epsilon', 'core.geometry.vec2', 'core.geometry.line',
    'core.geometry.circle',  'core.geometry.arc',  'core.geometry.project',
    'core.document.schema',  'core.document.validators',
    'core.document.commands','core.document.history',
    'app.state', 'app.config', 'app.shortcuts',
    'render.camera', 'render.svgRoot', 'render.grid', 'render.entityRenderers', 'render.overlays',
    'tools.toolManager', 'tools.selectTool',
    'ui.toolbar', 'ui.commandLine', 'ui.statusbar', 'ui.menubar', 'ui.dialogs',
    'bus.on', 'bus.emit'
  ];
  const HOSTS = ['menubar-host', 'toolbar-host', 'viewport-host', 'commandline-host', 'statusbar-host'];

  function banner(msg) {
    const b = document.createElement('div');
    b.className = 'lc-error-banner';
    b.textContent = '[LaserCAD] ' + msg;
    document.body.insertBefore(b, document.body.firstChild);
  }

  function get(path) {
    return path.split('.').reduce(function (o, k) { return o && o[k]; }, window.LaserCAD);
  }

  function validateNamespacePresence() {
    const missing = NS_REQUIRED.filter(function (p) { return typeof get(p) === 'undefined' || get(p) === null; });
    if (missing.length) {
      const msg = 'Missing namespace entries: ' + missing.join(', ');
      banner(msg);
      throw new Error(msg);
    }
  }

  function validateDomHosts() {
    const missing = HOSTS.filter(function (id) { return !document.getElementById(id); });
    if (missing.length) {
      const msg = 'Missing DOM host(s): ' + missing.join(', ');
      banner(msg);
      throw new Error(msg);
    }
  }

  LaserCAD.app.bootstrap = {
    start() {
      validateNamespacePresence();                                                  // 1
      LaserCAD.app.state.init();                                                    // 2
      LaserCAD.app.config.load();                                                   // 3
      validateDomHosts();                                                           // 4

      const svgRoot = LaserCAD.render.svgRoot.mount('#viewport-host');              // 5
      const size = svgRoot.getSize();
      LaserCAD.app.state.setViewportSize({ w: size.w, h: size.h });                 // 6
      LaserCAD.render.camera.attach(svgRoot);                                       // 7
      LaserCAD.render.grid.mount(svgRoot);                                          // 8
      LaserCAD.render.overlays.mount(svgRoot);                                      // 9
      LaserCAD.ui.menubar.mount('#menubar-host');                                   // 10
      LaserCAD.ui.toolbar.mount('#toolbar-host');                                   // 11
      LaserCAD.ui.commandLine.mount('#commandline-host');                           // 12
      LaserCAD.ui.statusbar.mount('#statusbar-host');                               // 13
      LaserCAD.ui.dialogs.init();                                                   // 14
      LaserCAD.tools.toolManager.init();                                            // 15
      LaserCAD.tools.toolManager.register('select', LaserCAD.tools.selectTool);     // 16
      LaserCAD.app.shortcuts.attach(window);                                        // 17

      wireResizeObserver(svgRoot);                                                  // 18
      LaserCAD.render.camera.zoomExtents();                                         // initial framing
      svgRoot.refreshViewBox();                                                     // 19
      LaserCAD.tools.toolManager.request('select');
      LaserCAD.bus.emit('app:ready', {});                                           // 20
    }
  };

  function wireResizeObserver(svgRoot) {
    if (typeof ResizeObserver === 'undefined') {
      window.addEventListener('resize', function () {
        const sz = svgRoot.getSize();
        LaserCAD.app.state.setViewportSize({ w: sz.w, h: sz.h });
      });
      return;
    }
    const ro = new ResizeObserver(function () {
      const sz = svgRoot.getSize();
      LaserCAD.app.state.setViewportSize({ w: sz.w, h: sz.h });
    });
    ro.observe(svgRoot.host);
  }
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

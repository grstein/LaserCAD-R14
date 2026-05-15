import { state } from '@/app/state.js';
import { config } from '@/app/config.js';
import { shortcuts } from '@/app/shortcuts.js';
import { bus } from '@/app/event-bus.js';
import { svgRoot } from '@/render/svg-root.js';
import { camera } from '@/render/camera.js';
import { grid } from '@/render/grid.js';
import { bed } from '@/render/bed.js';
import { overlays } from '@/render/overlays.js';
import { entityRenderers } from '@/render/entity-renderers.js';
import { menubar } from '@/ui/menubar.js';
import { toolbar } from '@/ui/toolbar.js';
import { commandLine } from '@/ui/command-line.js';
import { statusbar } from '@/ui/statusbar.js';
import { dialogs } from '@/ui/dialogs.js';
import { documentSizeDialog } from '@/ui/document-size-dialog.js';
import { toolManager } from '@/tools/tool-manager.js';
import { selectTool } from '@/tools/select-tool.js';
import { lineTool } from '@/tools/line-tool.js';
import { polylineTool } from '@/tools/polyline-tool.js';
import { rectTool } from '@/tools/rect-tool.js';
import { circleTool } from '@/tools/circle-tool.js';
import { arcTool } from '@/tools/arc-tool.js';
import { moveTool } from '@/tools/move-tool.js';
import { trimTool } from '@/tools/trim-tool.js';
import { extendTool } from '@/tools/extend-tool.js';
import { autosave } from '@/io/autosave.js';

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

const HOSTS = [
  'menubar-host',
  'toolbar-host',
  'viewport-host',
  'commandline-host',
  'statusbar-host',
];

function banner(msg) {
  const b = document.createElement('div');
  b.className = 'lc-error-banner';
  b.textContent = '[LaserCAD] ' + msg;
  document.body.insertBefore(b, document.body.firstChild);
}

function validateDomHosts() {
  const missing = HOSTS.filter(function (id) {
    return !document.getElementById(id);
  });
  if (missing.length) {
    const msg = 'Missing DOM host(s): ' + missing.join(', ');
    banner(msg);
    throw new Error(msg);
  }
}

export const bootstrap = {
  start() {
    // 1. validateNamespacePresence() obsolete under ES modules — Vite resolves
    //    imports at build time; missing modules fail at evaluation.
    state.init(); // 2
    config.load(); // 3
    validateDomHosts(); // 4

    const root = svgRoot.mount('#viewport-host'); // 5
    const size = root.getSize();
    state.setViewportSize({ w: size.w, h: size.h }); // 6
    camera.attach(root); // 7
    grid.mount(root); // 8
    bed.mount(root);
    overlays.mount(root); // 9
    menubar.mount('#menubar-host'); // 10
    toolbar.mount('#toolbar-host'); // 11
    commandLine.mount('#commandline-host'); // 12
    statusbar.mount('#statusbar-host'); // 13
    dialogs.init(); // 14
    toolManager.init(); // 15
    toolManager.attachSvgRoot(root);
    toolManager.register('select', selectTool); // 16
    if (lineTool) toolManager.register('line', lineTool);
    if (polylineTool) toolManager.register('polyline', polylineTool);
    if (rectTool) toolManager.register('rect', rectTool);
    if (circleTool) toolManager.register('circle', circleTool);
    if (arcTool) toolManager.register('arc', arcTool);
    if (moveTool) toolManager.register('move', moveTool);
    if (trimTool) toolManager.register('trim', trimTool);
    if (extendTool) toolManager.register('extend', extendTool);
    if (autosave) {
      autosave.init();
      if (autosave.restore()) {
        entityRenderers.renderAll(root, state);
      }
    }
    shortcuts.attach(window); // 17

    wireResizeObserver(root); // 18
    camera.zoomExtents(); // initial framing
    root.refreshViewBox(); // 19
    toolManager.request('select');
    bus.emit('app:ready', {}); // 20
  },
};

function wireResizeObserver(root) {
  if (typeof ResizeObserver === 'undefined') {
    window.addEventListener('resize', function () {
      const sz = root.getSize();
      state.setViewportSize({ w: sz.w, h: sz.h });
    });
    return;
  }
  const ro = new ResizeObserver(function () {
    const sz = root.getSize();
    state.setViewportSize({ w: sz.w, h: sz.h });
  });
  ro.observe(root.host);
}

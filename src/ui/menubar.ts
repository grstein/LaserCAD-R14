import { bus } from '@/app/event-bus.js';
import { state } from '@/app/state.js';
import { commands } from '@/core/document/commands.js';
import { toolManager } from '@/tools/tool-manager.js';
import { camera } from '@/render/camera.js';
import { entityRenderers } from '@/render/entity-renderers.js';
import { dialogs } from '@/ui/dialogs.js';
import { documentSizeDialog } from '@/ui/document-size-dialog.js';
import { fileActions } from '@/io/file-actions.js';
import { isTauri } from '@/tauri-bridge.js';

interface MenuItem {
  label?: string;
  action?: () => void;
  shortcut?: string;
  disabled?: boolean;
  separator?: boolean;
}

interface Menu {
  label: string;
  items: MenuItem[];
}

const MENUS: Menu[] = [
  {
    label: 'File',
    items: [
      {
        label: 'New',
        shortcut: 'Ctrl+N',
        action: function () {
          fileActions.newDocument();
        },
      },
      {
        label: 'Open SVG…',
        shortcut: 'Ctrl+O',
        action: function () {
          void fileActions.openSvg();
        },
      },
      {
        label: 'Save SVG…',
        shortcut: 'Ctrl+S',
        action: function () {
          void fileActions.saveSvg();
        },
      },
      { separator: true },
      {
        label: 'Bed size…',
        action: function () {
          documentSizeDialog.open();
        },
      },
      { separator: true },
      {
        label: 'Exit',
        disabled: !isTauri(),
        action: function () {
          void fileActions.exit();
        },
      },
    ],
  },
  {
    label: 'Edit',
    items: [
      {
        label: 'Undo',
        shortcut: 'Ctrl+Z',
        action: function () {
          if (state.undo()) refreshAll();
        },
      },
      {
        label: 'Redo',
        shortcut: 'Ctrl+Y',
        action: function () {
          if (state.redo()) refreshAll();
        },
      },
      {
        label: 'Delete',
        shortcut: 'Del',
        action: function () {
          if (state.selection.length === 0) return;
          state.selection.slice().forEach(function (id) {
            toolManager.commit(commands.removeEntity(id));
          });
          state.setSelection([]);
          refreshAll();
        },
      },
    ],
  },
  {
    label: 'View',
    items: [
      {
        label: 'Zoom extents',
        shortcut: 'Z E',
        action: function () {
          camera.zoomExtents();
        },
      },
      {
        label: 'Toggle grid',
        shortcut: 'F7',
        action: function () {
          tog('grid');
        },
      },
      {
        label: 'Toggle snap',
        shortcut: 'F3',
        action: function () {
          tog('snap');
        },
      },
      {
        label: 'Toggle ortho',
        shortcut: 'F8',
        action: function () {
          tog('ortho');
        },
      },
    ],
  },
  {
    label: 'Draw',
    items: [
      {
        label: 'Line (L)',
        shortcut: 'L',
        action: function () {
          req('line');
        },
      },
      {
        label: 'Polyline (P)',
        shortcut: 'P',
        action: function () {
          req('polyline');
        },
      },
      {
        label: 'Rectangle (R)',
        shortcut: 'R',
        action: function () {
          req('rect');
        },
      },
      {
        label: 'Circle (C)',
        shortcut: 'C',
        action: function () {
          req('circle');
        },
      },
      {
        label: 'Arc (A)',
        shortcut: 'A',
        action: function () {
          req('arc');
        },
      },
    ],
  },
  {
    label: 'Modify',
    items: [
      {
        label: 'Select (S)',
        shortcut: 'S',
        action: function () {
          req('select');
        },
      },
      {
        label: 'Move (M)',
        shortcut: 'M',
        action: function () {
          req('move');
        },
      },
      {
        label: 'Trim (T)',
        shortcut: 'T',
        action: function () {
          req('trim');
        },
      },
      {
        label: 'Extend (E)',
        shortcut: 'E',
        action: function () {
          req('extend');
        },
      },
    ],
  },
  {
    label: 'Help',
    items: [
      {
        label: 'Keyboard shortcuts',
        shortcut: 'F1',
        action: function () {
          dialogs.open({
            title: 'Keyboard shortcuts',
            body:
              'Tools:  L  P  R  C  A   (Line / Polyline / Rect / Circle / Arc)\n' +
              '        S  M  T  E      (Select / Move / Trim / Extend)\n' +
              'Del     Delete selection\n\n' +
              'Camera: Wheel = zoom (cursor pivot)\n' +
              '        Middle btn or Space+drag = pan\n\n' +
              'Modes:  F3 Snap   F7 Grid   F8 Ortho\n' +
              '        Shift held = ortho lock\n\n' +
              'Edit:   Esc   cancel current tool\n' +
              '        Ctrl+Z / Ctrl+Y   undo / redo\n' +
              '        Ctrl+N   new drawing\n' +
              '        Ctrl+O   open SVG\n' +
              '        Ctrl+S   save SVG\n\n' +
              'Command line:  124.5, 87.3   absolute coords\n' +
              '               @50, 0        relative coords\n' +
              '               50            distance (after first point)',
            actions: [
              {
                label: 'Close',
                primary: true,
                onClick: function () {
                  dialogs.close();
                },
              },
            ],
          });
        },
      },
      {
        label: 'About LaserCAD R14',
        action: function () {
          dialogs.open({
            title: 'About LaserCAD R14',
            body: 'Micro-CAD 2D no navegador.\nVersion 1.0 — 2026-05-12.\nUnit: mm | Wavelength: 450nm.\nOpens via file:// (no server).',
            actions: [
              {
                label: 'Close',
                primary: true,
                onClick: function () {
                  dialogs.close();
                },
              },
            ],
          });
        },
      },
    ],
  },
];

function req(toolId) {
  bus.emit('tool:request', { toolId: toolId });
}
function tog(name) {
  const cur = state.toggles[name];
  state.setToggle(name, !cur);
}
function refreshAll() {
  const sr = toolManager && toolManager.getSvgRoot();
  if (sr) entityRenderers.renderAll(sr, state);
}
let openDropdown = null;
function closeAll() {
  if (openDropdown && openDropdown.parentNode) openDropdown.parentNode.removeChild(openDropdown);
  openDropdown = null;
}
document.addEventListener('click', function (e) {
  if (openDropdown && !openDropdown.contains(e.target)) closeAll();
});
document.addEventListener('keydown', function (e) {
  if (e.key === 'Escape') closeAll();
});

export const menubar = {
  mount(hostSelector) {
    const host =
      typeof hostSelector === 'string' ? document.querySelector(hostSelector) : hostSelector;
    if (!host) throw new Error('[LaserCAD] menubar.mount: host not found');

    MENUS.forEach(function (menu) {
      const item = document.createElement('span');
      item.className = 'menu-item';
      item.textContent = menu.label;
      item.addEventListener('click', function (e) {
        e.stopPropagation();
        if (openDropdown && openDropdown.parentNode === item) {
          closeAll();
          return;
        }
        closeAll();
        const dd = document.createElement('div');
        dd.className = 'menu-dropdown';
        menu.items.forEach(function (row) {
          if (row.separator) {
            const sep = document.createElement('div');
            sep.className = 'menu-separator';
            dd.appendChild(sep);
            return;
          }
          const r = document.createElement('div');
          r.className = 'menu-row' + (row.disabled ? ' disabled' : '');
          r.innerHTML =
            '<span>' +
            row.label +
            '</span>' +
            (row.shortcut ? '<span class="shortcut">' + row.shortcut + '</span>' : '');
          if (!row.disabled && typeof row.action === 'function') {
            r.addEventListener('click', function () {
              row.action();
              closeAll();
            });
          }
          dd.appendChild(r);
        });
        item.appendChild(dd);
        openDropdown = dd;
      });
      host.appendChild(item);
    });
  },
};

import { bus } from '@/app/event-bus.js';
import { config } from '@/app/config.js';
import { state } from '@/app/state.js';
import { commandLine } from '@/ui/command-line.js';
import { dialogs } from '@/ui/dialogs.js';
import { menubar } from '@/ui/menubar.js';
import { toolManager } from '@/tools/tool-manager.js';
import { commands } from '@/core/document/commands.js';
import { entityRenderers } from '@/render/entity-renderers.js';
import { fileActions } from '@/io/file-actions.js';

const TOOL_KEYS = {
  l: 'line',
  p: 'polyline',
  r: 'rect',
  c: 'circle',
  a: 'arc',
  s: 'select',
  t: 'trim',
  e: 'extend',
  m: 'move',
};

function isTextEditingTarget(target) {
  if (!target) return false;
  const tag = (target.tagName || '').toUpperCase();
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return true;
  if (target.isContentEditable) return true;
  return false;
}

function onKeyDown(e) {
  if (commandLine && typeof commandLine.handleGlobalKey === 'function') {
    if (commandLine.handleGlobalKey(e)) return;
  }

  if (isTextEditingTarget(e.target)) return;

  if ((e.ctrlKey || e.metaKey) && (e.key === 'z' || e.key === 'Z')) {
    e.preventDefault();
    if (state.undo()) refreshAfterHistory();
    return;
  }
  if ((e.ctrlKey || e.metaKey) && (e.key === 'y' || e.key === 'Y')) {
    e.preventDefault();
    if (state.redo()) refreshAfterHistory();
    return;
  }
  if ((e.ctrlKey || e.metaKey) && (e.key === 's' || e.key === 'S')) {
    e.preventDefault();
    void fileActions.saveSvg();
    return;
  }
  if ((e.ctrlKey || e.metaKey) && (e.key === 'n' || e.key === 'N')) {
    e.preventDefault();
    fileActions.newDocument();
    return;
  }
  if ((e.ctrlKey || e.metaKey) && (e.key === 'o' || e.key === 'O')) {
    e.preventDefault();
    void fileActions.openSvg();
    return;
  }

  if (e.ctrlKey || e.metaKey || e.altKey) return;

  if (e.key === 'F1') {
    e.preventDefault();
    if (dialogs) {
      dialogs.open({
        title: 'Keyboard shortcuts',
        body: 'See docs/atalhos.md or use Help menu.',
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
    }
    return;
  }
  if (e.key === 'F3') {
    e.preventDefault();
    toggle('snap');
    return;
  }
  if (e.key === 'F7') {
    e.preventDefault();
    toggle('grid');
    return;
  }
  if (e.key === 'F8') {
    e.preventDefault();
    toggle('ortho');
    return;
  }

  if (e.key === 'Escape') {
    if (toolManager) toolManager.cancel();
    return;
  }

  if (e.key === 'Delete') {
    if (state.selection.length > 0) {
      const ids = state.selection.slice();
      ids.forEach(function (id) {
        toolManager.commit(commands.removeEntity(id));
      });
      state.setSelection([]);
      const sr = toolManager.getSvgRoot();
      if (sr) entityRenderers.renderAll(sr, state);
    }
    return;
  }

  const k = (e.key || '').toLowerCase();
  if (TOOL_KEYS[k]) {
    bus.emit('tool:request', { toolId: TOOL_KEYS[k] });
  }
}

function toggle(name) {
  const cur = state.toggles[name];
  state.setToggle(name, !cur);
}

function refreshAfterHistory() {
  const sr = toolManager && toolManager.getSvgRoot();
  if (sr) entityRenderers.renderAll(sr, state);
}

export const shortcuts = {
  attach(target) {
    (target || window).addEventListener('keydown', onKeyDown);
  },
  detach(target) {
    (target || window).removeEventListener('keydown', onKeyDown);
  },
  TOOL_KEYS: TOOL_KEYS,
};

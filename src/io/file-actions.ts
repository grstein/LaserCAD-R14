/**
 * File menu actions: New, Open SVG, Save SVG, Exit.
 *
 * The logic is centralized here so it can be reused between the menubar and the
 * keyboard shortcuts (Ctrl+N/O/S). Every document mutation goes through
 * `state.resetDocument` / `state.replaceDocument`, which clear entities, selection,
 * and history, and re-seed the ID counter.
 */

import { state } from '@/app/state.js';
import { camera } from '@/render/camera.js';
import { toolManager } from '@/tools/tool-manager.js';
import { entityRenderers } from '@/render/entity-renderers.js';
import { autosave } from '@/io/autosave.js';
import { exportSvg } from '@/io/export-svg.js';
import { importSvg } from '@/io/import-svg.js';
import { fileDownload } from '@/io/file-download.js';
import { dialogs } from '@/ui/dialogs.js';
import {
  isTauri,
  tauriSaveDialog,
  tauriOpenDialog,
  tauriReadText,
  tauriWriteText,
  tauriCloseWindow,
} from '@/tauri-bridge.js';

function refresh(): void {
  const sr = toolManager && toolManager.getSvgRoot();
  if (sr) entityRenderers.renderAll(sr, state);
}

function confirmDiscard(onConfirm: () => void): void {
  if (state.entities.length === 0) {
    onConfirm();
    return;
  }
  dialogs.open({
    title: 'Discard current drawing?',
    body: 'The current drawing has unsaved entities. This action will clear them along with the undo history.',
    actions: [
      { label: 'Cancel', onClick: () => dialogs.close() },
      {
        label: 'Discard',
        primary: true,
        onClick: () => {
          dialogs.close();
          onConfirm();
        },
      },
    ],
  });
}

function showError(title: string, message: string): void {
  dialogs.open({
    title,
    body: message,
    actions: [{ label: 'Close', primary: true, onClick: () => dialogs.close() }],
  });
}

function applyNew(): void {
  state.resetDocument();
  refresh();
  if (camera && camera.zoomExtents) camera.zoomExtents();
  if (autosave && autosave.saveNow) void autosave.saveNow();
}

function applyImport(svgText: string): boolean {
  if (!importSvg) return false;
  const result = importSvg.parse(svgText);
  if (!result.ok) {
    showError('Could not open SVG', result.error ?? 'Unsupported SVG.');
    return false;
  }
  state.replaceDocument({ entities: result.entities!, bounds: result.bounds! });
  refresh();
  if (camera && camera.zoomExtents) camera.zoomExtents();
  if (autosave && autosave.saveNow) void autosave.saveNow();
  return true;
}

function pickFileWeb(accept: string): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = accept;
    input.style.position = 'fixed';
    input.style.left = '-9999px';
    input.addEventListener(
      'change',
      () => {
        const f = input.files && input.files[0] ? input.files[0] : null;
        document.body.removeChild(input);
        resolve(f);
      },
      { once: true },
    );
    document.body.appendChild(input);
    input.click();
  });
}

async function readFileText(f: File): Promise<string> {
  if (typeof f.text === 'function') return f.text();
  return await new Promise<string>((resolve, reject) => {
    const fr = new FileReader();
    fr.onload = () => resolve(String(fr.result ?? ''));
    fr.onerror = () => reject(fr.error ?? new Error('FileReader failed'));
    fr.readAsText(f);
  });
}

export const fileActions = {
  newDocument(): void {
    confirmDiscard(applyNew);
  },

  async openSvg(): Promise<void> {
    confirmDiscard(async () => {
      try {
        if (isTauri()) {
          const path = await tauriOpenDialog();
          if (!path) return;
          const text = await tauriReadText(path);
          if (text == null) {
            showError('Could not open SVG', 'Failed to read selected file.');
            return;
          }
          applyImport(text);
          return;
        }
        const file = await pickFileWeb('.svg,image/svg+xml');
        if (!file) return;
        const text = await readFileText(file);
        applyImport(text);
      } catch (err) {
        console.error('[LaserCAD] openSvg failed:', err);
        showError('Could not open SVG', String((err as Error)?.message || err));
      }
    });
  },

  async saveSvg(): Promise<void> {
    if (!(exportSvg && fileDownload)) return;
    const svg = exportSvg.serialize(state, { preset: 'cut' });
    const name = 'drawing.svg';
    if (isTauri()) {
      try {
        const path = await tauriSaveDialog(name);
        if (path) {
          await tauriWriteText(path, svg);
          return;
        }
        return;
      } catch (err) {
        console.warn('[LaserCAD] tauri save failed, falling back to download:', err);
      }
    }
    fileDownload.download(name, svg);
  },

  async exit(): Promise<void> {
    if (isTauri()) {
      const closed = await tauriCloseWindow();
      if (closed) return;
      showError('Exit unavailable', 'Could not close the native window.');
      return;
    }
    showError('Exit', 'Exit is not available in the browser. Close the browser tab instead.');
  },
};

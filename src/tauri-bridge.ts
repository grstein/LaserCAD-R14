/**
 * Thin bridge that isolates Tauri vs. plain browser runtimes.
 *
 * When running inside the Tauri shell, `window.__TAURI_INTERNALS__` is present
 * and we can use native APIs (file system, dialogs, store). Otherwise, the app
 * falls back to localStorage and Blob downloads — exactly like the current web build.
 */

export const isTauri = (): boolean =>
  typeof window !== 'undefined' &&
  typeof (window as unknown as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ !==
    'undefined';

export async function tauriSaveDialog(defaultName: string): Promise<string | null> {
  if (!isTauri()) return null;
  const { save } = await import('@tauri-apps/plugin-dialog');
  return (await save({
    defaultPath: defaultName,
    filters: [
      { name: 'SVG', extensions: ['svg'] },
      { name: 'All Files', extensions: ['*'] },
    ],
  })) as string | null;
}

export async function tauriOpenDialog(): Promise<string | null> {
  if (!isTauri()) return null;
  const { open } = await import('@tauri-apps/plugin-dialog');
  const picked = await open({
    multiple: false,
    filters: [{ name: 'SVG', extensions: ['svg'] }],
  });
  return typeof picked === 'string' ? picked : null;
}

export async function tauriWriteText(path: string, content: string): Promise<void> {
  if (!isTauri()) return;
  const { writeTextFile } = await import('@tauri-apps/plugin-fs');
  await writeTextFile(path, content);
}

export async function tauriReadText(path: string): Promise<string | null> {
  if (!isTauri()) return null;
  const { readTextFile } = await import('@tauri-apps/plugin-fs');
  return (await readTextFile(path)) as string;
}

export async function tauriCloseWindow(): Promise<boolean> {
  if (!isTauri()) return false;
  try {
    const mod = await import('@tauri-apps/api/window');
    const w = mod.getCurrentWindow();
    await w.close();
    return true;
  } catch (err) {
    console.warn('[LaserCAD] tauriCloseWindow failed:', err);
    return false;
  }
}

interface TauriStore {
  get<T = unknown>(key: string): Promise<T | undefined>;
  set(key: string, value: unknown): Promise<void>;
  save(): Promise<void>;
}

let storeCache: TauriStore | null = null;

export async function tauriStore(): Promise<TauriStore | null> {
  if (!isTauri()) return null;
  if (storeCache) return storeCache;
  const { load } = await import('@tauri-apps/plugin-store');
  const s = await load('lasercad-autosave.json', { autoSave: true, defaults: {} });
  storeCache = s as unknown as TauriStore;
  return storeCache;
}

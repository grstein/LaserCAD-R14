import { describe, expect, it, beforeEach } from 'vitest';
import { menubar } from './menubar.js';

describe('menubar File menu', () => {
  beforeEach(() => {
    document.body.innerHTML = '<div id="menubar-host"></div>';
  });

  function openFile(): HTMLElement {
    const host = document.getElementById('menubar-host')!;
    menubar.mount('#menubar-host');
    const fileItem = Array.from(host.querySelectorAll('.menu-item')).find(
      (el) => el.textContent === 'File',
    ) as HTMLElement;
    expect(fileItem).toBeTruthy();
    fileItem.click();
    const dd = fileItem.querySelector('.menu-dropdown') as HTMLElement;
    expect(dd).toBeTruthy();
    return dd;
  }

  it('shows New / Open SVG / Save SVG / Bed size / Exit and no per-preset entries', () => {
    const dd = openFile();
    const rowLabels = Array.from(dd.querySelectorAll('.menu-row > span:first-child')).map((s) =>
      s.textContent?.trim(),
    );
    expect(rowLabels).toEqual(['New', 'Open SVG…', 'Save SVG…', 'Bed size…', 'Exit']);
    expect(rowLabels.some((l) => /cut|mark|engrave/i.test(l ?? ''))).toBe(false);
  });

  it('displays Ctrl+N, Ctrl+O, Ctrl+S shortcuts for the first three items', () => {
    const dd = openFile();
    const rows = Array.from(dd.querySelectorAll('.menu-row')) as HTMLElement[];
    const shortcuts = rows
      .slice(0, 3)
      .map((r) => r.querySelector('.shortcut')?.textContent?.trim());
    expect(shortcuts).toEqual(['Ctrl+N', 'Ctrl+O', 'Ctrl+S']);
  });

  it('disables Exit when not running inside Tauri', () => {
    const dd = openFile();
    const rows = Array.from(dd.querySelectorAll('.menu-row')) as HTMLElement[];
    const exit = rows.find((r) => r.querySelector('span')?.textContent?.trim() === 'Exit')!;
    expect(exit.classList.contains('disabled')).toBe(true);
  });

  it('renders separators between Save / Bed size / Exit groups', () => {
    const dd = openFile();
    const seps = dd.querySelectorAll('.menu-separator');
    expect(seps.length).toBe(2);
  });
});

import { bootstrap } from '@/app/bootstrap.js';

function go() {
  try {
    bootstrap.start();
  } catch (err) {
    console.error('[LaserCAD] bootstrap failed:', err);
    const banner = document.createElement('div');
    banner.className = 'lc-error-banner';
    banner.textContent = '[LaserCAD] bootstrap failed: ' + (err.message || err);
    document.body.insertBefore(banner, document.body.firstChild);
  }
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', go);
} else {
  go();
}

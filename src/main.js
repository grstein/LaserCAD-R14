(function () {
  'use strict';

  function go() {
    try {
      window.LaserCAD.app.bootstrap.start();
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
})();

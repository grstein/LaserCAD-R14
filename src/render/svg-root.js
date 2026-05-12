(function (LaserCAD) {
  'use strict';

  const ns = LaserCAD.render;
  const SVG_NS = 'http://www.w3.org/2000/svg';

  function el(tag, attrs) {
    const node = document.createElementNS(SVG_NS, tag);
    if (attrs) {
      for (const k in attrs) node.setAttribute(k, attrs[k]);
    }
    return node;
  }

  ns.svgRoot = {
    /**
     * @param {string|Element} hostOrSelector
     * @returns {{element:SVGSVGElement, host:Element, getSize:Function, refreshViewBox:Function, getLayer:Function}}
     */
    mount(hostOrSelector) {
      const host = (typeof hostOrSelector === 'string')
        ? document.querySelector(hostOrSelector)
        : hostOrSelector;
      if (!host) throw new Error('[LaserCAD] svgRoot.mount: host not found: ' + hostOrSelector);

      const svg = el('svg', {
        xmlns: SVG_NS,
        width: '100%',
        height: '100%',
        viewBox: '0 0 128 128',
        preserveAspectRatio: 'xMidYMid meet'
      });

      const gGrid     = el('g', { id: 'grid' });
      const gAxes     = el('g', { id: 'axes' });
      const gEntities = el('g', { id: 'entities' });
      const gPreview  = el('g', { id: 'preview' });
      const gOverlays = el('g', { id: 'overlays' });
      const gSnaps    = el('g', { id: 'snaps' });
      svg.appendChild(gGrid);
      svg.appendChild(gAxes);
      svg.appendChild(gEntities);
      svg.appendChild(gPreview);
      svg.appendChild(gOverlays);
      svg.appendChild(gSnaps);

      host.appendChild(svg);

      const handle = {
        element: svg,
        host: host,
        getSize() {
          const rect = host.getBoundingClientRect();
          return { w: rect.width, h: rect.height };
        },
        refreshViewBox() {
          const cam = LaserCAD.app.state.camera;
          const w = cam.viewportW || handle.getSize().w;
          const h = cam.viewportH || handle.getSize().h;
          const pxPerMm = LaserCAD.render.camera.pxPerMm(cam);
          const widthMm  = w / pxPerMm;
          const heightMm = h / pxPerMm;
          const x = cam.cx - widthMm / 2;
          const y = cam.cy - heightMm / 2;
          svg.setAttribute('viewBox', x + ' ' + y + ' ' + widthMm + ' ' + heightMm);
        },
        getLayer(name) { return svg.querySelector('#' + name); }
      };

      LaserCAD.bus.on('camera:changed', handle.refreshViewBox);
      LaserCAD.bus.on('viewport:resized', handle.refreshViewBox);

      return handle;
    }
  };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

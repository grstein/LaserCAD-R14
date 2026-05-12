(function (LaserCAD) {
  'use strict';

  const ns = LaserCAD.render;
  const SVG_NS = 'http://www.w3.org/2000/svg';

  function el(tag, attrs) {
    const n = document.createElementNS(SVG_NS, tag);
    for (const k in attrs) n.setAttribute(k, attrs[k]);
    return n;
  }

  function laserColor() {
    return getComputedStyle(document.documentElement).getPropertyValue('--laser-450').trim() || '#6E00FF';
  }

  ns.entityRenderers = {
    /** @param {{p1:{x:number,y:number},p2:{x:number,y:number}}} entity @param {Element} parent */
    renderLine(entity, parent) {
      const node = el('line', {
        x1: entity.p1.x, y1: entity.p1.y, x2: entity.p2.x, y2: entity.p2.y,
        stroke: laserColor(), 'stroke-width': '0.1', 'fill': 'none'
      });
      node.setAttribute('vector-effect', 'non-scaling-stroke');
      parent.appendChild(node);
      return node;
    },

    /** @param {{center:{x:number,y:number},r:number}} entity @param {Element} parent */
    renderCircle(entity, parent) {
      const node = el('circle', {
        cx: entity.center.x, cy: entity.center.y, r: entity.r,
        stroke: laserColor(), 'stroke-width': '0.1', 'fill': 'none'
      });
      node.setAttribute('vector-effect', 'non-scaling-stroke');
      parent.appendChild(node);
      return node;
    },

    /** Limpa a camada e renderiza todas as entities do state. @param {object} svgRoot @param {object} state */
    renderAll(svgRoot, state) {
      const layer = svgRoot.getLayer('entities');
      while (layer.firstChild) layer.removeChild(layer.firstChild);
      const r = LaserCAD.render.entityRenderers;
      state.entities.forEach(function (e) {
        if (e.type === 'line')   r.renderLine(e, layer);
        else if (e.type === 'circle') r.renderCircle(e, layer);
        else if (e.type === 'arc')    r.renderArc(e, layer);
      });
    },

    /** Renderiza ou atualiza o preview de uma entidade no layer #preview. */
    renderPreview(svgRoot, entity) {
      const layer = svgRoot.getLayer('preview');
      while (layer.firstChild) layer.removeChild(layer.firstChild);
      if (!entity) return;
      const r = LaserCAD.render.entityRenderers;
      let node = null;
      if (entity.type === 'line')   node = r.renderLine(entity, layer);
      else if (entity.type === 'circle') node = r.renderCircle(entity, layer);
      else if (entity.type === 'arc')    node = r.renderArc(entity, layer);
      if (node) {
        node.setAttribute('stroke-dasharray', '4 2');
        const glow = getComputedStyle(document.documentElement).getPropertyValue('--laser-glow').trim() || '#9D4DFF';
        node.setAttribute('stroke', glow);
      }
    },

    clearPreview(svgRoot) {
      const layer = svgRoot.getLayer('preview');
      while (layer.firstChild) layer.removeChild(layer.firstChild);
    },

    /** @param {{center:{x:number,y:number},r:number,startAngle:number,endAngle:number,ccw:boolean}} entity @param {Element} parent */
    renderArc(entity, parent) {
      const ends = LaserCAD.core.geometry.arc.endpoints(entity);
      const da = entity.ccw
        ? ((entity.endAngle - entity.startAngle + 2 * Math.PI) % (2 * Math.PI))
        : ((entity.startAngle - entity.endAngle + 2 * Math.PI) % (2 * Math.PI));
      const large = da > Math.PI ? 1 : 0;
      const sweep = entity.ccw ? 1 : 0;
      const d = 'M ' + ends.start.x + ' ' + ends.start.y +
                ' A ' + entity.r + ' ' + entity.r + ' 0 ' + large + ' ' + sweep + ' ' + ends.end.x + ' ' + ends.end.y;
      const node = el('path', { d: d, stroke: laserColor(), 'stroke-width': '0.1', 'fill': 'none' });
      node.setAttribute('vector-effect', 'non-scaling-stroke');
      parent.appendChild(node);
      return node;
    }
  };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

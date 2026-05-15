import { state } from '@/app/state.js';
import { bus } from '@/app/event-bus.js';

const SVG_NS = 'http://www.w3.org/2000/svg';

function el(tag, attrs) {
  const n = document.createElementNS(SVG_NS, tag);
  for (const k in attrs) n.setAttribute(k, attrs[k]);
  return n;
}

function clear(node) {
  while (node.firstChild) node.removeChild(node.firstChild);
}

function parseViewBox(s) {
  if (!s) return null;
  const parts = s.split(/\s+/).map(Number);
  if (parts.length !== 4 || parts.some(isNaN)) return null;
  return { minX: parts[0], minY: parts[1], w: parts[2], h: parts[3] };
}

function readVar(name, fallback) {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || fallback;
}

export const bed = {
  mount(svgRoot) {
    const layer = svgRoot.getLayer('bed');
    const svg = svgRoot.element;

    function render() {
      clear(layer);
      const b = state.documentBounds;
      if (!b || !(b.w > 0) || !(b.h > 0)) return;
      const vb = parseViewBox(svg.getAttribute('viewBox'));
      if (!vb) return;

      // Overlay externo via path evenodd: viewBox inteira menos a área do bed.
      const d =
        'M ' +
        vb.minX +
        ' ' +
        vb.minY +
        ' L ' +
        (vb.minX + vb.w) +
        ' ' +
        vb.minY +
        ' L ' +
        (vb.minX + vb.w) +
        ' ' +
        (vb.minY + vb.h) +
        ' L ' +
        vb.minX +
        ' ' +
        (vb.minY + vb.h) +
        ' Z ' +
        'M 0 0 L ' +
        b.w +
        ' 0 L ' +
        b.w +
        ' ' +
        b.h +
        ' L 0 ' +
        b.h +
        ' Z';
      const outside = el('path', { d: d, fill: 'rgba(0,0,0,0.45)', 'fill-rule': 'evenodd' });
      layer.appendChild(outside);

      // Borda do bed.
      const border = el('rect', {
        x: 0,
        y: 0,
        width: b.w,
        height: b.h,
        fill: 'none',
        stroke: readVar('--laser-450', '#6E00FF'),
        'stroke-width': '1.5',
      });
      border.setAttribute('vector-effect', 'non-scaling-stroke');
      layer.appendChild(border);
    }

    bus.on('camera:changed', render);
    bus.on('viewport:resized', render);
    bus.on('bounds:changed', render);
    bus.on('app:ready', render);

    render();
  },
};

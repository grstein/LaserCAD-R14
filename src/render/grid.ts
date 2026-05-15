import { state } from '@/app/state.js';
import { bus } from '@/app/event-bus.js';
import { camera } from '@/render/camera.js';

const SVG_NS = 'http://www.w3.org/2000/svg';
const MINOR_STEP_MM = 1;
const MAJOR_STEP_MM = 10;
const MIN_MINOR_PX = 6;
const MIN_MAJOR_PX = 4;

function el(tag, attrs) {
  const n = document.createElementNS(SVG_NS, tag);
  for (const k in attrs) n.setAttribute(k, attrs[k]);
  return n;
}

function clear(node) {
  while (node.firstChild) node.removeChild(node.firstChild);
}

function readVar(name) {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim() || '#444';
}

export const grid = {
  mount(svgRoot) {
    const layerGrid = svgRoot.getLayer('grid');
    const layerAxes = svgRoot.getLayer('axes');
    const svg = svgRoot.element;
    const colorMinor = readVar('--grid-minor');
    const colorMajor = readVar('--grid-major');
    const colorAxis = readVar('--grid-axis');

    function render() {
      if (!state.toggles.grid) {
        clear(layerGrid);
        renderAxes();
        return;
      }
      clear(layerGrid);
      const vb = parseViewBox(svg.getAttribute('viewBox'));
      if (!vb) return;
      const pxPerMm = camera.pxPerMm(state.camera);
      const minorPx = MINOR_STEP_MM * pxPerMm;
      const majorPx = MAJOR_STEP_MM * pxPerMm;

      if (majorPx >= MIN_MAJOR_PX) {
        renderLines(layerGrid, vb, MAJOR_STEP_MM, colorMajor, 0.5 / pxPerMm);
      }
      if (minorPx >= MIN_MINOR_PX) {
        renderDots(layerGrid, vb, MINOR_STEP_MM, colorMinor, pxPerMm);
      }
      renderAxes();
    }

    function renderAxes() {
      clear(layerAxes);
      const vb = parseViewBox(svg.getAttribute('viewBox'));
      if (!vb) return;
      const pxPerMm = camera.pxPerMm(state.camera);
      const w = 1 / pxPerMm;
      const xAxis = el('line', {
        x1: vb.minX,
        y1: 0,
        x2: vb.minX + vb.w,
        y2: 0,
        stroke: colorAxis,
        'stroke-width': w,
      });
      const yAxis = el('line', {
        x1: 0,
        y1: vb.minY,
        x2: 0,
        y2: vb.minY + vb.h,
        stroke: colorAxis,
        'stroke-width': w,
      });
      xAxis.setAttribute('vector-effect', 'non-scaling-stroke');
      yAxis.setAttribute('vector-effect', 'non-scaling-stroke');
      layerAxes.appendChild(xAxis);
      layerAxes.appendChild(yAxis);
    }

    function renderLines(parent, vb, step, color, strokeMm) {
      const xStart = Math.floor(vb.minX / step) * step;
      const xEnd = Math.ceil((vb.minX + vb.w) / step) * step;
      const yStart = Math.floor(vb.minY / step) * step;
      const yEnd = Math.ceil((vb.minY + vb.h) / step) * step;
      for (let x = xStart; x <= xEnd; x += step) {
        const line = el('line', {
          x1: x,
          y1: vb.minY,
          x2: x,
          y2: vb.minY + vb.h,
          stroke: color,
          'stroke-width': strokeMm,
        });
        line.setAttribute('vector-effect', 'non-scaling-stroke');
        parent.appendChild(line);
      }
      for (let y = yStart; y <= yEnd; y += step) {
        const line = el('line', {
          x1: vb.minX,
          y1: y,
          x2: vb.minX + vb.w,
          y2: y,
          stroke: color,
          'stroke-width': strokeMm,
        });
        line.setAttribute('vector-effect', 'non-scaling-stroke');
        parent.appendChild(line);
      }
    }

    function renderDots(parent, vb, step, color, pxPerMm) {
      const r = 0.6 / pxPerMm;
      const xStart = Math.floor(vb.minX / step) * step;
      const xEnd = Math.ceil((vb.minX + vb.w) / step) * step;
      const yStart = Math.floor(vb.minY / step) * step;
      const yEnd = Math.ceil((vb.minY + vb.h) / step) * step;
      for (let x = xStart; x <= xEnd; x += step) {
        for (let y = yStart; y <= yEnd; y += step) {
          const c = el('circle', { cx: x, cy: y, r: r, fill: color });
          parent.appendChild(c);
        }
      }
    }

    bus.on('camera:changed', render);
    bus.on('viewport:resized', render);
    bus.on('toggle:changed', function (p) {
      if (p && p.name === 'grid') render();
    });
    bus.on('app:ready', render);

    render();
  },
};

function parseViewBox(s) {
  if (!s) return null;
  const parts = s.split(/\s+/).map(Number);
  if (parts.length !== 4 || parts.some(isNaN)) return null;
  return { minX: parts[0], minY: parts[1], w: parts[2], h: parts[3] };
}

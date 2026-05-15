import { arc } from '@/core/geometry/arc.js';

function fmt(n) {
  if (!isFinite(n)) return '0';
  return (Math.round(n * 1000) / 1000).toString();
}

function entityToXml(e, stroke, strokeWidth) {
  if (e.type === 'line') {
    return (
      '  <line x1="' +
      fmt(e.p1.x) +
      '" y1="' +
      fmt(e.p1.y) +
      '" x2="' +
      fmt(e.p2.x) +
      '" y2="' +
      fmt(e.p2.y) +
      '"/>'
    );
  }
  if (e.type === 'circle') {
    return (
      '  <circle cx="' + fmt(e.center.x) + '" cy="' + fmt(e.center.y) + '" r="' + fmt(e.r) + '"/>'
    );
  }
  if (e.type === 'arc') {
    const ends = arc.endpoints(e);
    const da = e.ccw
      ? (e.endAngle - e.startAngle + 2 * Math.PI) % (2 * Math.PI)
      : (e.startAngle - e.endAngle + 2 * Math.PI) % (2 * Math.PI);
    const large = da > Math.PI ? 1 : 0;
    const sweep = e.ccw ? 1 : 0;
    return (
      '  <path d="M ' +
      fmt(ends.start.x) +
      ' ' +
      fmt(ends.start.y) +
      ' A ' +
      fmt(e.r) +
      ' ' +
      fmt(e.r) +
      ' 0 ' +
      large +
      ' ' +
      sweep +
      ' ' +
      fmt(ends.end.x) +
      ' ' +
      fmt(ends.end.y) +
      '"/>'
    );
  }
  return '';
}

/**
 * Serializa o documento atual como plain SVG compatível com LaserGRBL.
 * Cores: cut=#ff0000, mark=#0000ff, engrave=#00aa00. Stroke 0.1mm. fill=none.
 * @param {object} state
 * @param {{preset?:'cut'|'mark'|'engrave', strokeWidth?:number}} [opts]
 * @returns {string}
 */
function serialize(state, opts) {
  opts = opts || {};
  const preset = opts.preset || 'cut';
  const strokeMap = { cut: '#ff0000', mark: '#0000ff', engrave: '#00aa00' };
  const stroke = strokeMap[preset] || '#ff0000';
  const strokeWidth = opts.strokeWidth != null ? opts.strokeWidth : 0.5;
  const bounds = state.documentBounds;
  const w = bounds.w,
    h = bounds.h;

  const xml = [];
  xml.push('<?xml version="1.0" encoding="UTF-8"?>');
  xml.push(
    '<svg xmlns="http://www.w3.org/2000/svg" width="' +
      fmt(w) +
      'mm" height="' +
      fmt(h) +
      'mm" viewBox="0 0 ' +
      fmt(w) +
      ' ' +
      fmt(h) +
      '">',
  );
  xml.push(
    '  <g id="' +
      preset.toUpperCase() +
      '" fill="none" stroke="' +
      stroke +
      '" stroke-width="' +
      fmt(strokeWidth) +
      '">',
  );
  state.entities.forEach(function (e) {
    const s = entityToXml(e, stroke, strokeWidth);
    if (s) xml.push('  ' + s);
  });
  xml.push('  </g>');
  xml.push('</svg>');
  xml.push('');
  return xml.join('\n');
}

export const exportSvg = { serialize: serialize };

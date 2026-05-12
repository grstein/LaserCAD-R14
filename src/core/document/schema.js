(function (LaserCAD) {
  'use strict';

  const ns = LaserCAD.core.document;

  const SCHEMA_VERSION = 1;

  /**
   * @typedef {{id:string, type:'line', p1:{x:number,y:number}, p2:{x:number,y:number}}} LineEntity
   * @typedef {{id:string, type:'circle', center:{x:number,y:number}, r:number}} CircleEntity
   * @typedef {{id:string, type:'arc', center:{x:number,y:number}, r:number, startAngle:number, endAngle:number, ccw:boolean}} ArcEntity
   * @typedef {LineEntity|CircleEntity|ArcEntity} Entity
   */

  ns.schema = {
    SCHEMA_VERSION,

    /** @returns {object} new empty document */
    createDoc() {
      return {
        schemaVersion: SCHEMA_VERSION,
        units: 'mm',
        documentBounds: { w: 128, h: 128 },
        entities: [],
        selection: []
      };
    },

    /** @param {object} doc @returns {boolean} */
    isCompatible(doc) {
      return doc && doc.schemaVersion === SCHEMA_VERSION && doc.units === 'mm';
    }
  };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

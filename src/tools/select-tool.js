(function (LaserCAD) {
  'use strict';

  LaserCAD.tools.selectTool = {
    id: 'select',
    prompt: 'Select objects:',
    onArm(state) {
      // Sprint 2: idle/armed only. No entities to select yet.
      state.setToolState('armed');
    },
    onCancel(_state) { /* no-op */ },
    onPointerDown(_e, _state) { /* no entities yet */ },
    onPointerMove(_e, _state) { /* no preview */ },
    onPointerUp(_e, _state)   { /* no commit */ }
  };
})(window.LaserCAD = window.LaserCAD || {
  core: { geometry: {}, document: {} },
  render: {}, tools: {}, ui: {}, io: {}, app: {}, bus: {}
});

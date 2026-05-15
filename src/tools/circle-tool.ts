import { vec2 } from '@/core/geometry/vec2.js';
import { commands } from '@/core/document/commands.js';
import { entityRenderers } from '@/render/entity-renderers.js';
import { toolManager } from '@/tools/tool-manager.js';

let center = null;

function previewCircle(p2) {
  if (!center) return;
  const sr = toolManager.getSvgRoot();
  if (!sr) return;
  const r = vec2.len(vec2.sub(p2, center));
  if (r <= 0) return;
  entityRenderers.renderPreview(sr, { type: 'circle', center: center, r: r });
}

function commitCircle(r) {
  if (!(r > 0)) return;
  toolManager.commit(
    commands.addEntity({ type: 'circle', center: { x: center.x, y: center.y }, r: r }),
  );
  center = null;
  toolManager.cancel();
}

export const circleTool = {
  id: 'circle',
  prompt: 'CIRCLE  Specify center:',
  onArm(state) {
    center = null;
    state.setToolState('armed');
  },
  onCancel() {
    center = null;
    const sr = toolManager.getSvgRoot();
    if (sr) entityRenderers.clearPreview(sr);
  },
  onPointerMove(e) {
    if (center) previewCircle({ x: e.worldX, y: e.worldY });
  },
  onPointerDown(e, state) {
    const p = { x: e.worldX, y: e.worldY };
    if (!center) {
      center = p;
      state.setToolState('preview');
    } else commitCircle(vec2.len(vec2.sub(p, center)));
  },
  onCommand(payload, state) {
    const p = payload.parsed;
    if (!p) return;
    if (p.kind === 'absolute' && !center) {
      center = { x: p.x, y: p.y };
      state.setToolState('preview');
    } else if (p.kind === 'distance' && center) {
      commitCircle(p.value);
    } else if (p.kind === 'absolute' && center) {
      commitCircle(vec2.len(vec2.sub({ x: p.x, y: p.y }, center)));
    }
  },
};

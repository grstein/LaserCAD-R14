import { describe, expect, it, beforeEach } from 'vitest';
import { state } from './state.js';
import { commands } from '@/core/document/commands.js';

function makeLine(p1: { x: number; y: number }, p2: { x: number; y: number }) {
  return commands.addEntity({ type: 'line', p1, p2 });
}

describe('state.resetDocument / replaceDocument', () => {
  beforeEach(() => {
    state.resetDocument();
    state.setDocumentBounds({ w: 128, h: 128 });
  });

  it('resetDocument clears entities, selection and history', () => {
    state.applyCommand(makeLine({ x: 0, y: 0 }, { x: 10, y: 0 }));
    state.applyCommand(makeLine({ x: 0, y: 0 }, { x: 0, y: 10 }));
    state.setSelection([state.entities[0].id]);
    expect(state.entities).toHaveLength(2);
    expect(state.canUndo()).toBe(true);

    state.resetDocument();

    expect(state.entities).toHaveLength(0);
    expect(state.selection).toHaveLength(0);
    expect(state.canUndo()).toBe(false);
    expect(state.canRedo()).toBe(false);
  });

  it('replaceDocument seeds entities, bounds and id counter', () => {
    state.applyCommand(makeLine({ x: 0, y: 0 }, { x: 1, y: 1 }));
    expect(state.canUndo()).toBe(true);

    state.replaceDocument({
      entities: [
        { id: 'e_1', type: 'line', p1: { x: 0, y: 0 }, p2: { x: 5, y: 5 } },
        { id: 'e_2', type: 'circle', center: { x: 2, y: 2 }, r: 1 },
      ],
      bounds: { w: 200, h: 150 },
    });

    expect(state.entities).toHaveLength(2);
    expect(state.selection).toHaveLength(0);
    expect(state.canUndo()).toBe(false);
    expect(state.documentBounds).toEqual({ w: 200, h: 150 });

    state.applyCommand(makeLine({ x: 10, y: 10 }, { x: 20, y: 20 }));
    const newest = state.entities[state.entities.length - 1];
    expect(newest.id).toBe('e_3');
  });

  it('resetDocument resets id counter so subsequent adds start at e_1', () => {
    state.applyCommand(makeLine({ x: 0, y: 0 }, { x: 1, y: 0 }));
    state.applyCommand(makeLine({ x: 0, y: 0 }, { x: 0, y: 1 }));
    state.resetDocument();
    state.applyCommand(makeLine({ x: 0, y: 0 }, { x: 5, y: 5 }));
    expect(state.entities[0].id).toBe('e_1');
  });
});

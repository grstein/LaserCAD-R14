const CANONICAL_EVENTS = Object.freeze([
  'app:ready',
  'viewport:resized',
  'camera:changed',
  'cursor:moved',
  'tool:request',
  'tool:armed',
  'tool:cancel',
  'command:submit',
  'command:error',
  'toggle:changed',
  'bounds:changed',
]);
const canonicalSet = new Set(CANONICAL_EVENTS);

/** @type {Map<string, Set<Function>>} */
const listeners = new Map();

function warnIfUncanonical(evt) {
  if (!canonicalSet.has(evt)) {
    console.warn(
      '[LaserCAD] non-canonical event:',
      evt,
      '- ADR 0002 §1 requires ADR to add new events',
    );
  }
}

export const bus = {
  CANONICAL_EVENTS,

  on(evt, fn) {
    warnIfUncanonical(evt);
    if (!listeners.has(evt)) listeners.set(evt, new Set());
    listeners.get(evt).add(fn);
  },

  off(evt, fn) {
    const set = listeners.get(evt);
    if (set) set.delete(fn);
  },

  emit(evt, payload) {
    warnIfUncanonical(evt);
    const set = listeners.get(evt);
    if (!set) return;
    set.forEach(function (fn) {
      try {
        fn(payload);
      } catch (err) {
        console.error('[LaserCAD] listener error for', evt, err);
      }
    });
  },
};

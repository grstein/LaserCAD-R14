// Vitest setup — jsdom environment is already provided.
// Stub window.LaserCAD shim for legacy IIFE-style tests during migration.

declare global {
  interface Window {
    LaserCAD?: Record<string, unknown>;
  }
}

if (typeof window !== 'undefined' && !window.LaserCAD) {
  window.LaserCAD = {
    core: { geometry: {}, document: {} },
    render: {},
    tools: {},
    ui: {},
    io: {},
    app: {},
    bus: {},
  };
}

export {};

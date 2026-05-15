import type { Entity } from '@/core/types.js';

const SCHEMA_VERSION = 1 as const;

export interface Document {
  schemaVersion: typeof SCHEMA_VERSION;
  units: 'mm';
  documentBounds: { w: number; h: number };
  entities: Entity[];
  selection: string[];
}

export const schema = {
  SCHEMA_VERSION,

  createDoc(): Document {
    return {
      schemaVersion: SCHEMA_VERSION,
      units: 'mm',
      documentBounds: { w: 128, h: 128 },
      entities: [],
      selection: [],
    };
  },

  isCompatible(doc: unknown): doc is Document {
    if (!doc || typeof doc !== 'object') return false;
    const d = doc as Partial<Document>;
    return d.schemaVersion === SCHEMA_VERSION && d.units === 'mm';
  },
};

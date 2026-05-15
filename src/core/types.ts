/**
 * Core domain types for LaserCAD R14.
 *
 * These types form the shared contract between core/geometry, core/document,
 * render/, tools/, ui/, and io/. They are intentionally conservative and free
 * of dependencies — any file may import them.
 */

export interface Vec2 {
  x: number;
  y: number;
}

export interface Bounds {
  w: number;
  h: number;
}

export interface LineGeom {
  a: Vec2;
  b: Vec2;
}

export interface CircleGeom {
  center: Vec2;
  r: number;
}

export interface ArcGeom {
  center: Vec2;
  r: number;
  startAngle: number;
  endAngle: number;
  ccw: boolean;
}

export interface LineEntity {
  id: string;
  type: 'line';
  p1: Vec2;
  p2: Vec2;
}

export interface CircleEntity {
  id: string;
  type: 'circle';
  center: Vec2;
  r: number;
}

export interface ArcEntity {
  id: string;
  type: 'arc';
  center: Vec2;
  r: number;
  startAngle: number;
  endAngle: number;
  ccw: boolean;
}

export type Entity = LineEntity | CircleEntity | ArcEntity;
export type EntityType = Entity['type'];

export interface CameraState {
  cx: number;
  cy: number;
  zoom: number;
  viewportW: number;
  viewportH: number;
}

export interface CursorState {
  worldX: number;
  worldY: number;
  screenX: number;
  screenY: number;
}

export interface Toggles {
  snap: boolean;
  grid: boolean;
  ortho: boolean;
}

export interface AppState {
  schemaVersion: number;
  units: 'mm';
  documentBounds: Bounds;
  entities: Entity[];
  selection: string[];
  camera: CameraState;
  activeTool: string;
  toolState: string;
  cursor: CursorState;
  toggles: Toggles;
  commandHistory: string[];
  commandInput: string;
}

export interface Command {
  type: string;
  do: (state: AppState) => void;
  undo: (state: AppState) => void;
  meta?: Record<string, unknown>;
}

/** Minimal interface every drawing/editing tool implements. */
export interface Tool {
  id?: string;
  onActivate?: (ctx: ToolContext) => void;
  onDeactivate?: (ctx: ToolContext) => void;
  onPointerDown?: (ctx: ToolContext, e: PointerEvent, world: Vec2) => void;
  onPointerMove?: (ctx: ToolContext, e: PointerEvent, world: Vec2) => void;
  onPointerUp?: (ctx: ToolContext, e: PointerEvent, world: Vec2) => void;
  onKeyDown?: (ctx: ToolContext, e: KeyboardEvent) => void;
  onCommand?: (ctx: ToolContext, args: string[]) => void;
  cancel?: () => void;
}

export interface ToolContext {
  state: AppState;
  svgRoot: SvgRootHandle;
}

export interface SvgRootHandle {
  element: SVGSVGElement;
  host: HTMLElement;
  getSize(): { w: number; h: number };
  refreshViewBox(): void;
}

/** Snap result returned by the snap engine. */
export interface SnapResult {
  point: Vec2;
  type: 'endpoint' | 'midpoint' | 'center' | 'intersection' | 'none';
  entityId?: string;
  distance?: number;
}

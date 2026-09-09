export interface Point { x: number; y: number }
export interface Size { width: number; height: number }
export interface RenderConfig { canvas_width: number; canvas_height: number; scale: number; anchor_x: number; anchor_y: number }
export interface AnimationDefinition { fps: number; loop: boolean; priority: number; interruptible: boolean }
export interface LoadedAnimation extends AnimationDefinition { source: { type: 'png_sequence'; dir: string }; frames: string[] }
export type MouseButton = 'left' | 'right' | 'middle';
export type Trigger =
  | { type: 'timer'; min_ms: number; max_ms: number }
  | { type: 'cursor_distance_enter'; distance_px: number; reset_distance_px: number }
  | { type: 'hover'; dwell_ms: number }
  | { type: 'click'; button: MouseButton }
  | { type: 'drag_start' | 'drag_end' };
export interface WeightedAnimation { animation: string; weight: number }
export interface BehaviorDefinition { id: string; trigger: Trigger; only_when?: 'any' | 'fallback'; cooldown_ms?: number; choose: WeightedAnimation[] }
export interface LoadedPet { id: string; display_name: string; render: RenderConfig; fallback_animation: string; clips: Record<string, LoadedAnimation>; behaviors: BehaviorDefinition[] }
export interface Settings { active_pet: string | null; scale: number; always_on_top: boolean; last_position: Point | null; alpha_threshold: number }
export interface Catalog { pets: LoadedPet[]; warnings: string[] }
export interface Snapshot { catalog: Catalog; settings: Settings; content_root: string }
export interface DesktopSample { cursor: Point; origin: Point; size: Size; scale: number }
export interface Frame { image: CanvasImageSource; width: number; height: number; alpha: Uint8Array }
export interface Clip extends AnimationDefinition { frames: Frame[] }
export interface RuntimePet { id: string | null; displayName: string; render: RenderConfig; fallback: string; clips: Record<string, Clip>; behaviors: BehaviorDefinition[] }

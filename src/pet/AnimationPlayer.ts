import type { AnimationDefinition } from './types';
export interface Playable extends AnimationDefinition { frames: readonly unknown[] }

export class AnimationPlayer {
  current_animation: string;
  current_frame = 0;
  elapsed = 0;
  playing = true;
  pending_animation: string | null = null;

  constructor(readonly clips: Readonly<Record<string, Playable>>, readonly fallback: string) {
    if (!clips[fallback]?.frames.length) throw new Error('Fallback has no ready frames');
    this.current_animation = fallback;
  }
  get isFallback(): boolean { return this.current_animation === this.fallback; }
  private begin(name: string): void {
    this.current_animation = name; this.current_frame = 0; this.elapsed = 0; this.playing = true;
  }
  request(name: string): boolean {
    const next = this.clips[name];
    const current = this.clips[this.current_animation];
    if (!next?.frames.length || !current) return false;
    if (!this.playing || (current.interruptible && next.priority > current.priority)) {
      this.begin(name); return true;
    }
    // A loop has a completion boundary too, so uninterruptible loops cannot starve pending work.
    const pending = this.pending_animation ? this.clips[this.pending_animation] : undefined;
    if (!pending || next.priority >= pending.priority) this.pending_animation = name;
    return true;
  }
  tick(deltaMs: number): void {
    if (!this.playing || !Number.isFinite(deltaMs) || deltaMs < 0) return;
    this.elapsed += deltaMs;
    const clip = this.clips[this.current_animation];
    if (!clip) return;
    const duration = clip.frames.length * 1000 / clip.fps;
    if (this.elapsed >= duration) {
      if (this.pending_animation || (!clip.loop && !this.isFallback)) {
        const remainder = this.elapsed - duration;
        const next = this.pending_animation ?? this.fallback;
        this.pending_animation = null;
        this.begin(next);
        // Advance through at most a pending clip and then fallback, including long rAF pauses.
        this.tick(remainder);
        return;
      }
      this.elapsed %= duration;
    }
    this.current_frame = Math.min(clip.frames.length - 1, Math.floor(this.elapsed * clip.fps / 1000));
  }
}

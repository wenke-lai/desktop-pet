import type { BehaviorDefinition, MouseButton, WeightedAnimation } from './types';
import type { AnimationPlayer } from './AnimationPlayer';

export function weightedChoice(choices: readonly WeightedAnimation[], random: () => number = Math.random): string | null {
  const valid = choices.filter(c => Number.isFinite(c.weight) && c.weight > 0);
  // Normalize first to prevent a finite collection of large weights overflowing its sum.
  const max = valid.reduce((m,c) => Math.max(m,c.weight), 0);
  if (max === 0) return null;
  const rawSum = valid.reduce((total,c) => total + c.weight, 0);
  const divisor = Number.isFinite(rawSum) ? 1 : max;
  const sum = valid.reduce((total,c) => total + c.weight / divisor, 0);
  let value = Math.min(1 - Number.EPSILON, Math.max(0, random())) * sum;
  for (const choice of valid) { value -= choice.weight / divisor; if (value < 0) return choice.animation; }
  return valid.at(-1)?.animation ?? null;
}
export function interval(min: number, max: number, random: () => number): number {
  return min + Math.floor(Math.min(1 - Number.EPSILON, Math.max(0, random())) * (max - min + 1));
}
interface State { definition: BehaviorDefinition; nextAt: number; lastAt: number; near: boolean; hoverAt: number | null; hovered: boolean }
export class BehaviorEngine {
  private states: State[];
  constructor(behaviors: BehaviorDefinition[], private player: AnimationPlayer, now = 0, private random: () => number = Math.random) {
    this.states = behaviors.map(definition => ({ definition, nextAt: definition.trigger.type === 'timer' ? now + interval(definition.trigger.min_ms, definition.trigger.max_ms, random) : Infinity, lastAt: -Infinity, near: false, hoverAt: null, hovered: false }));
  }
  private fire(state: State, now: number): void {
    const b = state.definition;
    if ((b.only_when === 'fallback' && !this.player.isFallback) || now - state.lastAt < (b.cooldown_ms ?? 0)) return;
    const animation = weightedChoice(b.choose, this.random);
    if (!animation) { console.warn(`Skipped behavior ${b.id}: no valid weighted choices`); return; }
    if (this.player.request(animation)) state.lastAt = now;
  }
  tick(now: number): void {
    for (const state of this.states) {
      const trigger = state.definition.trigger;
      if (trigger.type === 'timer' && now >= state.nextAt) {
        state.nextAt = now + interval(trigger.min_ms, trigger.max_ms, this.random);
        this.fire(state, now);
      }
    }
  }
  sample(now: number, anchorDistanceLogical: number, alphaHit: boolean): void {
    for (const state of this.states) {
      const trigger = state.definition.trigger;
      if (trigger.type === 'cursor_distance_enter') {
        if (state.near && anchorDistanceLogical >= trigger.reset_distance_px) state.near = false;
        else if (!state.near && anchorDistanceLogical <= trigger.distance_px) { state.near = true; this.fire(state, now); }
      } else if (trigger.type === 'hover') {
        if (!alphaHit) { state.hoverAt = null; state.hovered = false; }
        else {
          state.hoverAt ??= now;
          if (!state.hovered && now - state.hoverAt >= trigger.dwell_ms) { state.hovered = true; this.fire(state, now); }
        }
      }
    }
  }
  event(type: 'click' | 'drag_start' | 'drag_end', now: number, button?: MouseButton): void {
    for (const state of this.states) {
      const trigger = state.definition.trigger;
      if (trigger.type === type && (trigger.type !== 'click' || trigger.button === button)) this.fire(state, now);
    }
  }
}

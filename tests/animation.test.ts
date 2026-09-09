import { describe, expect, it } from 'vitest';
import { AnimationPlayer, type Playable } from '../src/pet/AnimationPlayer';

function clips(): Record<string, Playable> {
  return {
    base: { frames: [0,1,2,3], fps: 4, loop: true, priority: 0, interruptible: true },
    small: { frames: [0,1], fps: 4, loop: false, priority: 10, interruptible: true },
    large: { frames: [0,1,2,3], fps: 4, loop: false, priority: 50, interruptible: false },
    other: { frames: [0,1], fps: 4, loop: false, priority: 50, interruptible: true },
    top: { frames: [0,1], fps: 4, loop: false, priority: 80, interruptible: true },
  };
}
describe('AnimationPlayer', () => {
  it('derives frames from elapsed time and wraps loops without interval drift', () => {
    const p = new AnimationPlayer(clips(), 'base'); p.tick(249); expect(p.current_frame).toBe(0); p.tick(251); expect(p.current_frame).toBe(2);
    p.tick(10000); expect(p.current_frame).toBe(2); expect(p.elapsed).toBe(500);
  });
  it('lets higher priority interrupt interruptible clips', () => {
    const p = new AnimationPlayer(clips(), 'base'); p.request('small'); p.tick(300); p.request('large');
    expect(p.current_animation).toBe('large'); expect(p.current_frame).toBe(0); expect(p.elapsed).toBe(0);
  });
  it('keeps noninterruptible playback and runs pending before fallback', () => {
    const p = new AnimationPlayer(clips(), 'base'); p.request('large'); p.request('top');
    expect(p.current_animation).toBe('large'); expect(p.pending_animation).toBe('top');
    p.tick(1000); expect(p.current_animation).toBe('top'); expect(p.pending_animation).toBeNull();
    p.tick(500); expect(p.current_animation).toBe('base');
  });
  it('retains only highest pending priority and replaces ties with newest', () => {
    const p = new AnimationPlayer(clips(), 'base'); p.request('large'); p.request('small'); p.request('other'); p.request('small');
    expect(p.pending_animation).toBe('other'); p.request('large'); expect(p.pending_animation).toBe('large');
  });
  it('queues equal and lower priority rather than interrupting', () => {
    const p = new AnimationPlayer(clips(), 'base'); p.request('top'); p.request('small');
    expect(p.current_animation).toBe('top'); p.tick(500); expect(p.current_animation).toBe('small');
  });
  it('returns nonlooping animation to the configured fallback with leftover time', () => {
    const p = new AnimationPlayer(clips(), 'base'); p.request('small'); p.tick(800);
    expect(p.current_animation).toBe('base'); expect(p.elapsed).toBe(300); expect(p.current_frame).toBe(1);
  });
  it('processes pending at loop boundary even when loop is noninterruptible', () => {
    const c = clips(); c.base = { ...c.base!, interruptible: false };
    const p = new AnimationPlayer(c, 'base'); p.request('small'); p.tick(1000); expect(p.current_animation).toBe('small');
  });
  it('handles long rAF suspension and nonlooping fallback without recursion overflow', () => {
    const c = clips(); c.base = { ...c.base!, loop: false };
    const p = new AnimationPlayer(c, 'base'); p.request('large'); p.request('top'); p.tick(1000000000);
    expect(p.current_animation).toBe('base'); expect(p.elapsed).toBeLessThan(1000);
  });
  it('ignores invalid requests and invalid clock deltas', () => {
    const p = new AnimationPlayer(clips(), 'base'); expect(p.request('missing')).toBe(false);
    p.tick(NaN); p.tick(-2); expect(p.elapsed).toBe(0);
  });
  it('new player starts with no old playback or pending state', () => {
    const old = new AnimationPlayer(clips(), 'base'); old.request('large'); old.request('top');
    const replacement = new AnimationPlayer(clips(), 'small');
    expect(replacement.current_animation).toBe('small'); expect(replacement.pending_animation).toBeNull(); expect(replacement.elapsed).toBe(0);
  });
});

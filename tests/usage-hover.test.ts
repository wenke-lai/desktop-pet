import { afterEach, expect, it, vi } from 'vitest';
import { InteractionController } from '../src/pet/InteractionController';
import type { DesktopBridge } from '../src/pet/DesktopBridge';
import type { BehaviorEngine } from '../src/pet/BehaviorEngine';
import type { Renderer } from '../src/pet/Renderer';
import type { DesktopSample } from '../src/pet/types';
afterEach(() => { vi.useRealTimers(); vi.unstubAllGlobals(); });
it('shows only on opaque hover, hides on press/exit/dispose, and preserves click-through', async () => {
  vi.useFakeTimers();
  vi.stubGlobal('window', new EventTarget());
  let sample: DesktopSample = { origin: { x: 0, y: 0 }, size: { width: 200, height: 200 }, scale: 1, cursor: { x: 250, y: 250 } };
  const bridge = {
    sample: vi.fn(async () => sample), settle: vi.fn(async () => sample), move: vi.fn(async () => sample),
    ignore: vi.fn(async () => {}), usageHover: vi.fn(async () => {}), log: vi.fn(async () => {}), savePosition: vi.fn(async () => {}),
  } as unknown as DesktopBridge;
  const canvas = Object.assign(new EventTarget(), {
    getBoundingClientRect: () => ({ left: 0, top: 0, width: 200, height: 200 }),
    setPointerCapture: vi.fn(), hasPointerCapture: () => false, releasePointerCapture: vi.fn(),
  });
  const renderer = { canvas, frame: { width: 2, height: 2, alpha: new Uint8Array([0, 255, 255, 255]) } } as unknown as Renderer;
  const engine = { event: vi.fn(), sample: vi.fn() } as unknown as BehaviorEngine;
  const controller = new InteractionController(renderer, { canvas_width: 2, canvas_height: 2, scale: 1, anchor_x: 0.5, anchor_y: 0.9 }, engine, bridge, sample, 20);
  try {
    await vi.advanceTimersByTimeAsync(45);
    expect(bridge.usageHover).not.toHaveBeenCalled();
    expect(bridge.ignore).toHaveBeenLastCalledWith(true);
    sample = { ...sample, cursor: { x: 150, y: 50 } };
    await vi.advanceTimersByTimeAsync(45);
    expect(bridge.usageHover).toHaveBeenLastCalledWith(true, { x: 0.5, y: 0 });
    expect(bridge.ignore).toHaveBeenLastCalledWith(false);
    const press = Object.assign(new Event('pointerdown'), { pointerId: 1, button: 0, clientX: 150, clientY: 50 });
    canvas.dispatchEvent(press);
    await vi.advanceTimersByTimeAsync(45);
    expect(bridge.usageHover).toHaveBeenLastCalledWith(false, { x: 0.5, y: 0 });
    canvas.dispatchEvent(Object.assign(new Event('pointerup'), { pointerId: 1 }));
    await vi.advanceTimersByTimeAsync(45);
    expect(bridge.usageHover).toHaveBeenLastCalledWith(true, { x: 0.5, y: 0 });
    sample = { ...sample, cursor: { x: 50, y: 50 } }; // Transparent pixel inside the canvas.
    await vi.advanceTimersByTimeAsync(45);
    expect(bridge.usageHover).toHaveBeenLastCalledWith(false, { x: 0.5, y: 0 });
    expect(bridge.ignore).toHaveBeenLastCalledWith(true);
    sample = { ...sample, cursor: { x: 150, y: 50 } };
    await vi.advanceTimersByTimeAsync(45);
  } finally { await controller.dispose(); }
  expect(bridge.usageHover).toHaveBeenLastCalledWith(false, { x: 0.5, y: 0 });
  expect(vi.getTimerCount()).toBe(0);
});

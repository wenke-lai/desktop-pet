import { alphaHit } from './HitTester';
import type { BehaviorEngine } from './BehaviorEngine';
import type { DesktopBridge } from './DesktopBridge';
import type { Renderer } from './Renderer';
import type { DesktopSample, MouseButton, Point, RenderConfig } from './types';

const POLL_MS = 40;
const DRAG_THRESHOLD_LOGICAL = 5;
const CLICK_MAX_MS = 600;
interface Press { pointer: number; button: MouseButton; start: Point; anchor: Point; at: number; dragging: boolean }
export class InteractionController {
  private hovered = false;
  private head: Point | undefined;
  private press: Press | null = null;
  private ignored: boolean | null = null;
  private sample: DesktopSample;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private disposed = false;
  private processing = Promise.resolve();
  private lastSettle = 0;
  private failureCount = 0;
  private events = new AbortController();
  constructor(private renderer: Renderer, private render: RenderConfig, private engine: BehaviorEngine, private bridge: DesktopBridge, sample: DesktopSample, private threshold: number) {
    this.sample = sample;
    const frame = renderer.frame;
    if (frame) {
      let top = frame.height, left = frame.width, right = 0;
      for (let y = 0; y < frame.height; y++) for (let x = 0; x < frame.width; x++) {
        if ((frame.alpha[y * frame.width + x] ?? 0) >= Math.max(1, threshold)) {
          top = Math.min(top, y); left = Math.min(left, x); right = Math.max(right, x);
        }
      }
      if (top < frame.height) this.head = { x: (left + right + 1) / (2 * frame.width), y: top / frame.height };
    }
    const options = { signal: this.events.signal };
    renderer.canvas.addEventListener('pointerdown', e => { this.enqueue(() => this.down(e)); }, options);
    renderer.canvas.addEventListener('pointerup', e => { this.enqueue(() => this.up(e.pointerId, false)); }, options);
    renderer.canvas.addEventListener('pointercancel', e => { this.enqueue(() => this.up(e.pointerId, true)); }, options);
    renderer.canvas.addEventListener('lostpointercapture', e => { this.enqueue(() => this.up(e.pointerId, true)); }, options);
    renderer.canvas.addEventListener('contextmenu', e => e.preventDefault(), options);
    window.addEventListener('blur', () => { this.enqueue(() => this.cancel()); }, options);
    this.enqueue(() => this.poll());
  }
  private enqueue(action: () => Promise<void>): void {
    this.processing = this.processing.then(async () => { if (!this.disposed) await action(); }).catch(error => {
      console.error('Desktop interaction failed', error);
      void this.bridge.log(`Desktop interaction failed: ${String(error)}`).catch(console.error);
    });
  }
  private anchor(sample = this.sample): Point { return { x: sample.origin.x + this.render.anchor_x * sample.size.width, y: sample.origin.y + this.render.anchor_y * sample.size.height }; }
  private hit(sample = this.sample): boolean {
    return alphaHit(this.renderer.frame, { x: sample.cursor.x - sample.origin.x, y: sample.cursor.y - sample.origin.y }, sample.size, this.threshold);
  }
  private async setHover(hovered: boolean): Promise<void> {
    if (hovered === this.hovered && !hovered) return;
    await this.bridge.usageHover(hovered, this.head);
    this.hovered = hovered;
  }
  private async setIgnored(ignore: boolean): Promise<void> {
    if (this.ignored === ignore) return;
    await this.bridge.ignore(ignore); this.ignored = ignore;
  }
  private async down(event: PointerEvent): Promise<void> {
    if (this.press) return;
    const button = (['left', 'middle', 'right'] as const)[event.button];
    if (!button) return;
    this.sample = await this.bridge.sample();
    // Verify the actual event pixel too: the polling snapshot may be up to 40ms old.
    const rect = this.renderer.canvas.getBoundingClientRect();
    if (!alphaHit(this.renderer.frame, { x: event.clientX - rect.left, y: event.clientY - rect.top }, rect, this.threshold)) return;
    await this.setHover(false);
    this.press = { pointer: event.pointerId, button, start: this.sample.cursor, anchor: this.anchor(), at: performance.now(), dragging: false };
    this.renderer.canvas.setPointerCapture(event.pointerId);
    await this.setIgnored(false);
  }
  private async updateDrag(): Promise<void> {
    const press = this.press;
    if (!press || press.button !== 'left') return;
    const dx = this.sample.cursor.x - press.start.x, dy = this.sample.cursor.y - press.start.y;
    if (!press.dragging && Math.hypot(dx, dy) / this.sample.scale >= DRAG_THRESHOLD_LOGICAL) {
      press.dragging = true; this.engine.event('drag_start', performance.now());
    }
    if (press.dragging) this.sample = await this.bridge.move({ x: press.anchor.x + dx, y: press.anchor.y + dy });
  }
  private async up(pointer: number, cancelled: boolean): Promise<void> {
    const press = this.press;
    if (!press || press.pointer !== pointer) return;
    this.sample = await this.bridge.sample();
    await this.updateDrag();
    this.press = null;
    if (this.renderer.canvas.hasPointerCapture(pointer)) this.renderer.canvas.releasePointerCapture(pointer);
    if (press.dragging) {
      this.sample = await this.bridge.settle();
      await this.bridge.savePosition();
      this.engine.event('drag_end', performance.now());
    } else if (!cancelled && this.hit() && performance.now() - press.at <= CLICK_MAX_MS) {
      this.engine.event('click', performance.now(), press.button);
    }
    await this.setIgnored(!this.hit());
  }
  private async cancel(): Promise<void> { if (this.press) await this.up(this.press.pointer, true); }
  private async poll(): Promise<void> {
    try {
      this.sample = await this.bridge.sample();
      const now = performance.now();
      await this.updateDrag();
      if (!this.press && now - this.lastSettle > 2000) {
        this.lastSettle = now;
        this.sample = await this.bridge.settle();
      }
      const hit = this.hit();
      await this.setIgnored(this.press ? false : !hit);
      await this.setHover(!this.press && hit);
      const anchor = this.anchor();
      this.engine.sample(now, Math.hypot(this.sample.cursor.x - anchor.x, this.sample.cursor.y - anchor.y) / this.sample.scale, !this.press && hit);
      this.failureCount = 0;
    } catch (error) {
      if (this.failureCount++ === 0) {
        console.error('Global cursor/click-through API unavailable on this platform', error);
        await this.bridge.log(`Platform cursor/click-through limitation: ${String(error)}`).catch(console.error);
      }
      await this.setHover(false).catch(console.error);
      // Retry, but restore input so the pet remains recoverable via drag/tray.
      await this.setIgnored(false).catch(console.error);
    } finally {
      if (!this.disposed) this.timer = setTimeout(() => this.enqueue(() => this.poll()), POLL_MS);
    }
  }
  async dispose(): Promise<void> {
    this.disposed = true; this.events.abort();
    if (this.timer) clearTimeout(this.timer);
    await this.processing;
    await this.cancel();
    await this.setHover(false);
    await this.setIgnored(true);
  }
}

import type { Frame } from './types';
export class Renderer {
  frame: Frame | null = null;
  private context: CanvasRenderingContext2D;
  constructor(readonly canvas: HTMLCanvasElement) {
    const context = canvas.getContext('2d');
    if (!context) throw new Error('Canvas 2D is unavailable');
    this.context = context;
  }
  draw(frame: Frame): void {
    const width = Math.max(1, Math.round(this.canvas.clientWidth * devicePixelRatio));
    const height = Math.max(1, Math.round(this.canvas.clientHeight * devicePixelRatio));
    const resized = width !== this.canvas.width || height !== this.canvas.height;
    if (resized) { this.canvas.width = width; this.canvas.height = height; }
    if (!resized && this.frame === frame) return;
    this.frame = frame;
    this.context.clearRect(0, 0, width, height);
    this.context.drawImage(frame.image, 0, 0, width, height);
  }
}

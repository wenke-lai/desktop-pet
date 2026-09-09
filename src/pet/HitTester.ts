import type { Frame, Point, Size } from './types';
export function alphaHit(frame: Frame | null, local: Point, viewport: Size, threshold: number): boolean {
  if (!frame || viewport.width <= 0 || viewport.height <= 0 || local.x < 0 || local.y < 0 || local.x >= viewport.width || local.y >= viewport.height) return false;
  const x = Math.floor(local.x / viewport.width * frame.width);
  const y = Math.floor(local.y / viewport.height * frame.height);
  return (frame.alpha[y * frame.width + x] ?? 0) >= Math.max(1, threshold);
}

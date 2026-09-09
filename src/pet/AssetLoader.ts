import { convertFileSrc } from '@tauri-apps/api/core';
import type { Clip, Frame, LoadedPet, RuntimePet } from './types';

export function frameFromCanvas(canvas: HTMLCanvasElement): Frame {
  const context = canvas.getContext('2d', { willReadFrequently: true });
  if (!context) throw new Error('Canvas 2D is unavailable');
  const rgba = context.getImageData(0, 0, canvas.width, canvas.height).data;
  const alpha = new Uint8Array(canvas.width * canvas.height);
  for (let i = 0; i < alpha.length; i++) alpha[i] = rgba[i * 4 + 3] ?? 0;
  return { image: canvas, width: canvas.width, height: canvas.height, alpha };
}
async function loadFrame(path: string, revision: number, width: number, height: number): Promise<Frame> {
  const image = new Image();
  image.crossOrigin = 'anonymous';
  const ready = new Promise<void>((resolve, reject) => {
    const timeout = setTimeout(() => { image.src = ''; reject(new Error(`Image load timeout: ${path}`)); }, 15000);
    image.onload = () => { clearTimeout(timeout); resolve(); };
    image.onerror = () => { clearTimeout(timeout); reject(new Error(`Cannot decode image: ${path}`)); };
  });
  image.src = `${convertFileSrc(path)}?revision=${revision}`;
  await ready;
  if (image.naturalWidth !== width || image.naturalHeight !== height) throw new Error(`Image size changed since scan: ${path}`);
  const canvas = document.createElement('canvas'); canvas.width = width; canvas.height = height;
  const context = canvas.getContext('2d');
  if (!context) throw new Error('Canvas 2D is unavailable');
  context.drawImage(image, 0, 0);
  return frameFromCanvas(canvas);
}
export async function loadPet(pet: LoadedPet, revision: number): Promise<RuntimePet> {
  const clips: Record<string, Clip> = Object.create(null) as Record<string, Clip>;
  const names = [pet.fallback_animation, ...Object.keys(pet.clips).filter(n => n !== pet.fallback_animation)];
  // Preload the entire active pack, fallback first. Sequential decoding limits memory spikes.
  for (const name of names) {
    const clip = pet.clips[name];
    if (!clip) continue;
    try {
      const frames: Frame[] = [];
      for (const path of clip.frames) frames.push(await loadFrame(path, revision, pet.render.canvas_width, pet.render.canvas_height));
      clips[name] = { ...clip, frames };
    } catch (error) {
      console.warn(`Skipped animation ${name}`, error);
      if (name === pet.fallback_animation) throw error;
    }
  }
  const behaviors = pet.behaviors.map(b => ({ ...b, choose: b.choose.filter(c => clips[c.animation]) })).filter(b => b.choose.length > 0);
  return { id: pet.id, displayName: pet.display_name, render: pet.render, fallback: pet.fallback_animation, clips, behaviors };
}

import { frameFromCanvas } from './AssetLoader';
import type { Clip, RuntimePet } from './types';

// This is an in-memory diagnostic fixture. Production packs have no debug source type.
export function createDebugPet(): RuntimePet {
  const clips: Record<string, Clip> = Object.create(null) as Record<string, Clip>;
  const designs = [
    { name: 'rest', color: '#78dbc0', priority: 0, loop: true, amplitude: 5 },
    { name: 'pulse', color: '#ffcf72', priority: 10, loop: false, amplitude: 12 },
    { name: 'greet', color: '#83baff', priority: 30, loop: false, amplitude: 8 },
    { name: 'spark', color: '#e39ff5', priority: 50, loop: false, amplitude: 18 },
  ];
  for (const design of designs) {
    const frames = Array.from({ length: 24 }, (_, i) => {
      const canvas = document.createElement('canvas'); canvas.width = 256; canvas.height = 256;
      const ctx = canvas.getContext('2d');
      if (!ctx) throw new Error('Canvas 2D is unavailable');
      const y = 152 + Math.sin(i / 24 * Math.PI * 2) * design.amplitude;
      ctx.fillStyle = design.color; ctx.beginPath(); ctx.arc(128, y, 57, 0, 2 * Math.PI); ctx.fill();
      ctx.fillStyle = '#19312d';
      for (const x of [109, 147]) { ctx.beginPath(); ctx.ellipse(x, y - 9, 5, 8, 0, 0, 2 * Math.PI); ctx.fill(); }
      ctx.strokeStyle = '#19312d'; ctx.lineWidth = 3;
      ctx.beginPath(); ctx.arc(128, y + 6, 10, 0.15, Math.PI - 0.15); ctx.stroke();
      return frameFromCanvas(canvas);
    });
    clips[design.name] = { fps: 24, loop: design.loop, priority: design.priority, interruptible: design.priority < 50, frames };
  }
  return {
    id: null, displayName: 'Debug Pet', render: { canvas_width: 256, canvas_height: 256, scale: 1, anchor_x: 0.5, anchor_y: 0.9 }, fallback: 'rest', clips,
    behaviors: [
      { id: 'timer', trigger: { type: 'timer', min_ms: 4000, max_ms: 7000 }, only_when: 'fallback', choose: [{ animation: 'pulse', weight: 1 }] },
      { id: 'near', trigger: { type: 'cursor_distance_enter', distance_px: 150, reset_distance_px: 210 }, cooldown_ms: 2000, choose: [{ animation: 'greet', weight: 1 }] },
      { id: 'hover', trigger: { type: 'hover', dwell_ms: 500 }, choose: [{ animation: 'greet', weight: 1 }] },
      ...(['left', 'right', 'middle'] as const).map(button => ({ id: `click-${button}`, trigger: { type: 'click' as const, button }, cooldown_ms: 400, choose: [{ animation: 'spark', weight: 1 }] })),
      { id: 'drag-start', trigger: { type: 'drag_start' }, choose: [{ animation: 'greet', weight: 1 }] },
      { id: 'drag-end', trigger: { type: 'drag_end' }, choose: [{ animation: 'pulse', weight: 1 }] },
    ],
  };
}

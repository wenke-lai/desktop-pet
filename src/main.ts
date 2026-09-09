import './styles.css';
import { listen } from '@tauri-apps/api/event';
import { AnimationPlayer } from './pet/AnimationPlayer';
import { BehaviorEngine } from './pet/BehaviorEngine';
import { loadPet } from './pet/AssetLoader';
import { createDebugPet } from './pet/DebugPet';
import { desktop } from './pet/DesktopBridge';
import { InteractionController } from './pet/InteractionController';
import { Renderer } from './pet/Renderer';
import type { RuntimePet, Snapshot } from './pet/types';

const canvas = document.querySelector<HTMLCanvasElement>('#pet');
const status = document.querySelector<HTMLParagraphElement>('#status');
if (!canvas || !status) throw new Error('Missing application elements');
const renderer = new Renderer(canvas);
let snapshot: Snapshot;
let pet: RuntimePet | null = null;
let player: AnimationPlayer | null = null;
let engine: BehaviorEngine | null = null;
let interactions: InteractionController | null = null;
let revision = Date.now();
let tasks = Promise.resolve();

function report(error: unknown): void {
  console.error(error);
  void desktop.log(String(error)).catch(console.error);
  if (status) { status.textContent = `載入失敗：${String(error)}。請查看 logs，再使用 Tray → Reload Content。`; status.hidden = false; }
}
function enqueue(action: () => Promise<void>): void { tasks = tasks.then(action).catch(report); }
async function activate(id: string | null): Promise<void> {
  const candidates = [...snapshot.catalog.pets];
  const index = candidates.findIndex(p => p.id === id);
  if (index > 0) candidates.unshift(...candidates.splice(index, 1));
  let ready: RuntimePet | null = null;
  for (const candidate of candidates) {
    try { ready = await loadPet(candidate, revision); break; } catch (e) { report(e); }
  }
  ready ??= createDebugPet();
  await interactions?.dispose(); interactions = null;
  const sample = await desktop.activate(ready.id);
  pet = ready;
  player = new AnimationPlayer(pet.clips, pet.fallback);
  engine = new BehaviorEngine(pet.behaviors, player, performance.now());
  const first = pet.clips[pet.fallback]?.frames[0];
  if (first) renderer.draw(first);
  interactions = new InteractionController(renderer, pet.render, engine, desktop, sample, snapshot.settings.alpha_threshold);
  if (status) status.hidden = true;
  console.info(`Active pet changed: ${pet.displayName}`);
}
async function reload(): Promise<void> {
  const active = pet?.id ?? snapshot?.settings.active_pet;
  snapshot = await desktop.reload(); revision++;
  snapshot.catalog.warnings.forEach(w => console.warn(w));
  await activate(active ?? null);
}
async function next(): Promise<void> {
  const pets = snapshot.catalog.pets;
  const index = pets.findIndex(p => p.id === pet?.id);
  await activate(pets[(index + 1) % Math.max(1, pets.length)]?.id ?? null);
}
let previous = performance.now();
function animate(now: number): void {
  const delta = now - previous; previous = now;
  if (player && pet && engine) {
    player.tick(delta); engine.tick(now);
    const frame = pet.clips[player.current_animation]?.frames[player.current_frame];
    if (frame) renderer.draw(frame);
  }
  requestAnimationFrame(animate);
}
async function start(): Promise<void> {
  await listen<string>('tray-action', ({ payload }) => {
    if (payload === 'reload') enqueue(reload);
    if (payload === 'next') enqueue(next);
  });
  snapshot = await desktop.snapshot();
  console.info(`Content root: ${snapshot.content_root}`);
  await activate(snapshot.settings.active_pet);
  requestAnimationFrame(animate);
}
enqueue(start);

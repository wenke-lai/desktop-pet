import { invoke } from '@tauri-apps/api/core';
import type { DesktopSample, Point, Snapshot } from './types';

export const desktop = {
  usageHover: (hovered: boolean, head?: Point) => invoke<void>('set_usage_hover', { hovered, head }),
  snapshot: () => invoke<Snapshot>('get_snapshot'),
  reload: () => invoke<Snapshot>('reload_content'),
  activate: (id: string | null) => invoke<DesktopSample>('activate_pet', { id }),
  sample: () => invoke<DesktopSample>('desktop_sample'),
  ignore: (ignore: boolean) => invoke<void>('set_cursor_passthrough', { ignore }),
  move: (anchor: Point) => invoke<DesktopSample>('move_pet', { anchor }),
  savePosition: () => invoke<void>('save_position'),
  settle: () => invoke<DesktopSample>('settle_window'),
  log: (message: string) => invoke<void>('frontend_log', { message }),
};
export type DesktopBridge = typeof desktop;

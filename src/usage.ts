import './usage.css';
import { invoke } from '@tauri-apps/api/core';
import { codexWindow, resetLabel, usageLabel, type CodexUsageSnapshot } from './pet/CodexUsage';
const usage = document.querySelector<HTMLElement>('#usage')!;
const reset = document.querySelector<HTMLElement>('#reset')!;
const state = document.querySelector<HTMLElement>('#usage-state')!;
let latest: CodexUsageSnapshot | null = null;
async function refresh(): Promise<void> {
  try {
    latest = await invoke<CodexUsageSnapshot>('get_codex_usage');
    const quota = codexWindow(latest);
    usage.textContent = usageLabel(quota);
    reset.textContent = resetLabel(quota?.resetsAt ?? null);
    state.textContent = quota ? '' : '目前沒有 Codex 配額資料';
  } catch (error) {
    console.warn('Codex usage unavailable', error);
    if (latest) {
      const updated = new Date(latest.fetchedAt * 1000).toLocaleTimeString('zh-TW', { hour: '2-digit', minute: '2-digit', hourCycle: 'h23' });
      state.textContent = `更新失敗 · 上次 ${updated}`;
    } else {
      usage.textContent = '無法取得'; reset.textContent = '—';
      state.textContent = '請確認 Codex 已安裝並登入';
    }
  } finally { setTimeout(() => void refresh(), 60_000); }
}
void refresh();

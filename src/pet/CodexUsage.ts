export interface QuotaWindow { usedPercent: number; windowDurationMins: number | null; resetsAt: number | null }
interface Quota { limitId?: string | null; primary?: QuotaWindow | null; secondary?: QuotaWindow | null }
export interface CodexUsageSnapshot {
  fetchedAt: number;
  source: string;
  account: unknown;
  rateLimits: { rateLimits?: Quota | null; rateLimitsByLimitId?: Record<string, Quota> | null };
  usage: unknown;
}
export function codexWindow(snapshot: CodexUsageSnapshot): QuotaWindow | null {
  const limits = snapshot.rateLimits;
  // A multi-bucket response is authoritative. Never display Spark as general Codex.
  const quota = limits.rateLimitsByLimitId != null
    ? limits.rateLimitsByLimitId.codex
    : limits.rateLimits?.limitId == null || limits.rateLimits.limitId === 'codex' ? limits.rateLimits : null;
  return quota?.primary ?? quota?.secondary ?? null;
}
export function resetLabel(seconds: number | null, timeZone?: string): string {
  if (seconds == null || !Number.isFinite(seconds)) return '—';
  const date = new Date(seconds * 1000);
  if (!Number.isFinite(date.getTime())) return '—';
  const parts = new Intl.DateTimeFormat('zh-TW', {
    timeZone, month: '2-digit', day: '2-digit', weekday: 'short', hour: '2-digit', minute: '2-digit', hourCycle: 'h23',
  }).formatToParts(date);
  const part = (type: Intl.DateTimeFormatPartTypes) => parts.find(p => p.type === type)?.value ?? '';
  const weekday = part('weekday').replace(/^(週|周|星期)/, '');
  return `${part('month')}/${part('day')}（${weekday}） ${part('hour')}:${part('minute')}`;
}
export function usageLabel(window: QuotaWindow | null): string {
  return window && Number.isFinite(window.usedPercent) ? `${window.usedPercent}%` : '—';
}

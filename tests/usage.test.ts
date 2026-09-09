import { expect, it } from 'vitest';
import { codexWindow, resetLabel, usageLabel, type CodexUsageSnapshot } from '../src/pet/CodexUsage';
const quota = { usedPercent: 80, windowDurationMins: 10080, resetsAt: 1789391012 };
const snapshot = (rateLimits: CodexUsageSnapshot['rateLimits']): CodexUsageSnapshot => ({ fetchedAt: 0, source: '', account: null, usage: null, rateLimits });
it('prefers the Codex bucket over the legacy or Spark bucket', () => {
  expect(codexWindow(snapshot({ rateLimits: { primary: { ...quota, usedPercent: 10 } }, rateLimitsByLimitId: { codex: { primary: quota }, spark: { primary: { ...quota, usedPercent: 0 } } } }))).toEqual(quota);
  expect(codexWindow(snapshot({ rateLimits: { primary: quota }, rateLimitsByLimitId: { spark: { primary: quota } } }))).toBeNull();
});
it('supports old responses and missing windows without inventing zero usage', () => {
  expect(codexWindow(snapshot({ rateLimits: { primary: quota } }))).toEqual(quota);
  expect(codexWindow(snapshot({ rateLimits: { limitId: 'spark', primary: quota } }))).toBeNull();
  expect(codexWindow(snapshot({ rateLimits: { secondary: quota } }))).toEqual(quota);
  expect(usageLabel(null)).toBe('—');
  expect(usageLabel({ ...quota, usedPercent: 0 })).toBe('0%');
  expect(usageLabel(quota)).toBe('80%');
});
it('formats Unix seconds with the local calendar weekday and 24-hour time', () => {
  expect(resetLabel(1789391012, 'Asia/Taipei')).toBe('09/14（一） 21:03');
  expect(resetLabel(1789391012, 'America/Los_Angeles')).toBe('09/14（一） 06:03');
  expect(resetLabel(Date.UTC(2026, 8, 13, 16) / 1000, 'Asia/Taipei')).toBe('09/14（一） 00:00');
  expect(resetLabel(null)).toBe('—');
  expect(resetLabel(NaN)).toBe('—');
});

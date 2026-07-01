import { describe, it, expect } from 'vitest';
import { formatDuration, formatClock } from './time';

describe('formatDuration', () => {
  it('formats mm:ss under an hour', () => {
    expect(formatDuration(0)).toBe('00:00');
    expect(formatDuration(65)).toBe('01:05');
    expect(formatDuration(1500)).toBe('25:00');
  });
  it('formats h:mm:ss at/over an hour', () => {
    expect(formatDuration(3661)).toBe('1:01:01');
  });
  it('clamps negatives to zero', () => {
    expect(formatDuration(-5)).toBe('00:00');
  });
});

describe('formatClock', () => {
  it('formats AM times with 오전', () => {
    expect(formatClock(new Date(2024, 0, 1, 8, 42))).toBe('오전 8:42');
    expect(formatClock(new Date(2024, 0, 1, 0, 0))).toBe('오전 12:00'); // midnight
  });
  it('formats PM times with 오후', () => {
    expect(formatClock(new Date(2024, 0, 1, 14, 5))).toBe('오후 2:05');
    expect(formatClock(new Date(2024, 0, 1, 12, 0))).toBe('오후 12:00'); // noon
  });
});

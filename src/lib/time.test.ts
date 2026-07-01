import { describe, it, expect } from 'vitest';
import { formatDuration } from './time';

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

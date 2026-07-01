import { describe, it, expect, beforeEach } from 'vitest';
import { invokeMock, resetTauri } from '../test/tauri-mock';
import { routinesStore } from './routines.svelte';

describe('routines store', () => {
  beforeEach(resetTauri);
  it('refresh loads routines and stats', async () => {
    invokeMock.mockImplementation((cmd: string) => {
      if (cmd === 'routines_list') return Promise.resolve([{ id: 1, name: '딥워크', icon: '🎯', color: null, target_seconds: 3600, pomodoro_enabled: true, focus_minutes: 25, break_minutes: 5, sort_order: 1, archived: false, created_at: 't' }]);
      if (cmd === 'stats_today') return Promise.resolve({ total_secs: 900, completed: 0, routine_count: 1, remaining_secs: 2700, streak: 2, best_streak: 5, per_routine: { 1: 900 } });
      return Promise.resolve();
    });
    await routinesStore.refresh();
    expect(routinesStore.list.length).toBe(1);
    expect(routinesStore.stats?.streak).toBe(2);
    expect(routinesStore.secondsFor(1)).toBe(900);
  });
});

import { describe, it, expect, beforeEach } from 'vitest';
import { emitTauri, resetTauri } from '../test/tauri-mock';
import { timer, initTimerListeners, resetTimer } from './timer.svelte';

describe('timer store', () => {
  beforeEach(() => { resetTauri(); resetTimer(); });

  it('updates fields and progress from timer://tick', async () => {
    await initTimerListeners();
    emitTauri('timer://tick', {
      state: 'Running', mode: 'Continuous', phase: 'Focus', routine_id: 1, pomodoro_index: 1,
      remaining_secs: 1200, session_seconds: 300, routine_today_secs: 300, target_secs: 1500,
      state_changed: false, event: null, remaining_label: '20:00',
    });
    expect(timer.remainingSecs).toBe(1200);
    expect(timer.routineId).toBe(1);
    expect(timer.progress).toBeCloseTo(0.2);
    expect(timer.label).toBe('20:00');
  });

  it('clamps progress to 0 when routine_today_secs is negative', async () => {
    await initTimerListeners();
    emitTauri('timer://tick', {
      state: 'Running', mode: 'Continuous', phase: 'Focus', routine_id: 1, pomodoro_index: 1,
      remaining_secs: 0, session_seconds: 0, routine_today_secs: -50, target_secs: 1500,
      state_changed: false, event: null, remaining_label: '00:00',
    });
    expect(timer.progress).toBe(0);
  });
});

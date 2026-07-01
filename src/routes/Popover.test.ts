import { describe, it, expect, beforeEach } from 'vitest';
import { render } from '@testing-library/svelte';
import { invokeMock, resetTauri, emitTauri } from '../test/tauri-mock';
import { resetTimer } from '$lib/timer.svelte';
import Popover from './Popover.svelte';

const idleSnapshot = {
  state: 'Idle', mode: 'Continuous', phase: 'Focus',
  routine_id: null, pomodoro_index: 1,
  remaining_secs: 0, session_seconds: 0,
  routine_today_secs: 0, target_secs: 0,
  state_changed: false, event: null, remaining_label: '00:00',
};

const todayStats = {
  total_secs: 1800, completed: 2, routine_count: 3,
  remaining_secs: 3600, streak: 3, best_streak: 7, per_routine: {},
};

describe('Popover', () => {
  beforeEach(() => {
    resetTauri();
    resetTimer();
    invokeMock.mockImplementation((cmd: string) =>
      cmd === 'routines_list' ? Promise.resolve([]) :
      cmd === 'stats_today' ? Promise.resolve(todayStats) :
      cmd === 'settings_get' ? Promise.resolve({ theme: 'system', streak_rule: 'focused' }) :
      cmd === 'timer_get_state' ? Promise.resolve(idleSnapshot) :
      Promise.resolve()
    );
  });

  it('mounts without throwing', () => {
    expect(() => render(Popover)).not.toThrow();
  });

  it('shows today summary after stats load', async () => {
    const { findByText } = render(Popover);
    // stats.remaining_secs = 3600 → formatDurationKo → '1시간', completed = 2
    const el = await findByText(/남은.*완료/);
    expect(el).toBeInTheDocument();
  });

  it('shows empty state when no routines', async () => {
    const { findByText } = render(Popover);
    expect(await findByText('루틴이 없습니다')).toBeInTheDocument();
  });

  it('summary stays live: updates after routines://changed fires', async () => {
    const { findByText } = render(Popover);

    // Wait for initial render with todayStats (completed: 2)
    await findByText(/남은.*완료/);

    // Update mock to return new stats (completed: 5)
    const updatedStats = { ...todayStats, completed: 5, remaining_secs: 1800 };
    invokeMock.mockImplementation((cmd: string) =>
      cmd === 'routines_list' ? Promise.resolve([]) :
      cmd === 'stats_today' ? Promise.resolve(updatedStats) :
      cmd === 'settings_get' ? Promise.resolve({ theme: 'system', streak_rule: 'focused' }) :
      cmd === 'timer_get_state' ? Promise.resolve(idleSnapshot) :
      Promise.resolve()
    );

    // Emit the routines://changed event — triggers routinesStore.refresh()
    emitTauri('routines://changed', {});

    // Wait for updated summary to appear (completed: 5, remaining: 1800s → 30분)
    const updated = await findByText(/5개 완료/);
    expect(updated).toBeInTheDocument();
  });
});

import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { TimerSnapshot } from './types';

class TimerStore {
  state = $state<TimerSnapshot['state']>('Idle');
  mode = $state<TimerSnapshot['mode']>('Continuous');
  phase = $state<TimerSnapshot['phase']>('Focus');
  routineId = $state<number | null>(null);
  pomodoroIndex = $state(1);
  remainingSecs = $state(0);
  sessionSeconds = $state(0);
  routineTodaySecs = $state(0);
  targetSecs = $state(0);
  label = $state('00:00');

  progress = $derived(this.targetSecs > 0 ? Math.max(0, Math.min(1, this.routineTodaySecs / this.targetSecs)) : 0);
  isActive = $derived(this.state === 'Running');

  apply(s: TimerSnapshot) {
    this.state = s.state; this.mode = s.mode; this.phase = s.phase;
    this.routineId = s.routine_id; this.pomodoroIndex = s.pomodoro_index;
    this.remainingSecs = s.remaining_secs; this.sessionSeconds = s.session_seconds;
    this.routineTodaySecs = s.routine_today_secs; this.targetSecs = s.target_secs;
    this.label = s.remaining_label;
  }
}
export const timer = new TimerStore();

export function resetTimer() {
  timer.state = 'Idle';
  timer.mode = 'Continuous';
  timer.phase = 'Focus';
  timer.routineId = null;
  timer.pomodoroIndex = 1;
  timer.remainingSecs = 0;
  timer.sessionSeconds = 0;
  timer.routineTodaySecs = 0;
  timer.targetSecs = 0;
  timer.label = '00:00';
}

export async function initTimerListeners(): Promise<UnlistenFn> {
  const unTick = await listen<TimerSnapshot>('timer://tick', (e) => timer.apply(e.payload));
  const unState = await listen<TimerSnapshot>('timer://state', (e) => timer.apply(e.payload));
  return () => { unTick(); unState(); };
}

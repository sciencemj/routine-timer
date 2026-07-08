import { invoke } from '@tauri-apps/api/core';
import type { Routine, NewRoutine, TimerSnapshot, TodayStats, ReportData } from './types';

export const commands = {
  routinesList: () => invoke<Routine[]>('routines_list'),
  routineCreate: (newRoutine: NewRoutine) => invoke<Routine>('routine_create', { new: newRoutine }),
  routineUpdate: (routine: Routine) => invoke<void>('routine_update', { routine }),
  routineDelete: (id: number) => invoke<void>('routine_delete', { id }),
  routineReorder: (orderedIds: number[]) => invoke<void>('routine_reorder', { orderedIds }),
  timerStart: (routineId: number) => invoke<TimerSnapshot>('timer_start', { routineId }),
  timerPause: () => invoke<TimerSnapshot>('timer_pause'),
  timerResume: () => invoke<TimerSnapshot>('timer_resume'),
  timerStop: () => invoke<void>('timer_stop'),
  timerSkipBreak: () => invoke<TimerSnapshot>('timer_skip_break'),
  timerSwitch: (routineId: number) => invoke<TimerSnapshot>('timer_switch', { routineId }),
  timerGetState: () => invoke<TimerSnapshot>('timer_get_state'),
  statsToday: () => invoke<TodayStats>('stats_today'),
  statsReport: () => invoke<ReportData>('stats_report'),
  settingsGet: () => invoke<Record<string, string>>('settings_get'),
  settingsSet: (key: string, value: string) => invoke<void>('settings_set', { key, value }),
  dbReset: () => invoke<TimerSnapshot>('db_reset'),
  timerResync: () => invoke<TimerSnapshot>('timer_resync'),
  isMobile: () => invoke<boolean>('is_mobile'),
};

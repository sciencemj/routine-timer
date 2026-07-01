export type Mode = 'Pomodoro' | 'Continuous';
export type TimerStateName = 'Idle' | 'Running' | 'Paused' | 'Break';
export type Phase = 'Focus' | 'Break';
export type TimerEventName = 'FocusEnded' | 'BreakEnded' | 'TargetReached';
export type StreakRule = 'focused' | 'any_completed' | 'all_completed';
export type ThemePref = 'system' | 'light' | 'dark';

export interface Routine {
  id: number; name: string; icon: string; color: string | null;
  target_seconds: number; pomodoro_enabled: boolean;
  focus_minutes: number; break_minutes: number;
  sort_order: number; archived: boolean; created_at: string;
}
export interface NewRoutine {
  name: string; icon: string; color: string | null;
  target_seconds: number; pomodoro_enabled: boolean;
  focus_minutes: number; break_minutes: number;
}
export interface TimerSnapshot {
  state: TimerStateName; mode: Mode; phase: Phase;
  routine_id: number | null; pomodoro_index: number;
  remaining_secs: number; session_seconds: number;
  routine_today_secs: number; target_secs: number;
  state_changed: boolean; event: TimerEventName | null; remaining_label: string;
}
export interface TodayStats {
  total_secs: number; completed: number; routine_count: number;
  remaining_secs: number; streak: number; best_streak: number; per_routine: Record<number, number>;
}
export interface HeatCell { date: string; secs: number; level: number }
export interface ReportData {
  heatmap: HeatCell[];
  this_week_secs: number;
  last_week_secs: number;
  daily_avg_secs: number;
  month_active_days: number;
  streak: number;
  best_streak: number;
  last7: number[];
}

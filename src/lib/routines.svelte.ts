import { listen } from '@tauri-apps/api/event';
import { commands } from './commands';
import type { Routine, TodayStats } from './types';

class RoutinesStore {
  list = $state<Routine[]>([]);
  stats = $state<TodayStats | null>(null);
  async refresh() {
    this.list = await commands.routinesList();
    this.stats = await commands.statsToday();
  }
  secondsFor(id: number): number { return this.stats?.per_routine?.[id] ?? 0; }
}
export const routinesStore = new RoutinesStore();

export async function initRoutinesListeners() {
  const un1 = await listen('routines://changed', () => routinesStore.refresh());
  const un2 = await listen('timer://state', () => routinesStore.refresh());
  return () => { un1(); un2(); };
}

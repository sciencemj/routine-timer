import { describe, it, expect, beforeEach } from 'vitest';
import { invokeMock, setThemeMock, resetTauri } from '../test/tauri-mock';
import { themeStore } from './theme.svelte';

describe('theme store', () => {
  beforeEach(resetTauri);
  it('system pref follows detected system theme', async () => {
    invokeMock.mockImplementation((cmd: string) => cmd === 'settings_get' ? Promise.resolve({ theme: 'system', streak_rule: 'focused' }) : Promise.resolve());
    await themeStore.init();
    expect(themeStore.pref).toBe('system');
    expect(themeStore.effective).toBe('light'); // system detected as light in mock
    expect(document.documentElement.dataset.theme).toBe('light');
  });
  it('explicit pref overrides system', async () => {
    invokeMock.mockImplementation((cmd: string) => cmd === 'settings_get' ? Promise.resolve({ theme: 'dark', streak_rule: 'focused' }) : Promise.resolve());
    await themeStore.init();
    expect(themeStore.effective).toBe('dark');
  });
  it('setPref system calls setTheme(null) to unforce window theme', async () => {
    invokeMock.mockImplementation((cmd: string) => cmd === 'settings_get' ? Promise.resolve({ theme: 'dark', streak_rule: 'focused' }) : Promise.resolve());
    await themeStore.init();
    await themeStore.setPref('system');
    expect(setThemeMock).toHaveBeenCalledWith(null);
  });
  it('setPref dark calls setTheme(dark)', async () => {
    invokeMock.mockImplementation((cmd: string) => cmd === 'settings_get' ? Promise.resolve({ theme: 'system', streak_rule: 'focused' }) : Promise.resolve());
    await themeStore.init();
    await themeStore.setPref('dark');
    expect(setThemeMock).toHaveBeenCalledWith('dark');
  });
});

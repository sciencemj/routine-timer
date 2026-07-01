import { getCurrentWindow } from '@tauri-apps/api/window';
import { commands } from './commands';
import type { ThemePref } from './types';

function createThemeStore() {
  let pref = $state<ThemePref>('system');
  let system = $state<'light' | 'dark'>('light');
  const effective = $derived<'light' | 'dark'>(pref === 'system' ? system : pref);

  function apply() {
    document.documentElement.dataset.theme = effective;
  }

  async function init() {
    const settings = await commands.settingsGet();
    const savedPref = (settings?.theme as ThemePref | undefined) ?? 'system';
    pref = savedPref;

    const win = getCurrentWindow();
    const sysTheme = await win.theme();
    system = (sysTheme === 'dark') ? 'dark' : 'light';

    await win.onThemeChanged(({ payload }: { payload: 'light' | 'dark' | null }) => {
      system = payload === 'dark' ? 'dark' : 'light';
      apply();
    });

    apply();
  }

  async function setPref(p: ThemePref) {
    pref = p;
    await commands.settingsSet('theme', p);
    if (p === 'light' || p === 'dark') {
      await getCurrentWindow().setTheme(p);
    }
    apply();
  }

  return {
    get pref() { return pref; },
    get system() { return system; },
    get effective() { return effective; },
    init,
    setPref,
  };
}

export const themeStore = createThemeStore();

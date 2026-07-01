<script lang="ts">
  import Router from 'svelte-spa-router';
  import Home from './routes/Home.svelte';
  import Focus from './routes/Focus.svelte';
  import Settings from './routes/Settings.svelte';
  import Report from './routes/Report.svelte';
  import Popover from './routes/Popover.svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { themeStore } from '$lib/theme.svelte';
  import { initRoutinesListeners } from '$lib/routines.svelte';

  const routes = {
    '/': Home,
    '/focus': Focus,
    '/settings': Settings,
    '/report': Report,
    '/popover': Popover,
  };

  $effect(() => {
    if (getCurrentWindow().label !== 'main') return;
    let alive = true;
    let cleanup: (() => void) | undefined;
    themeStore.init();
    initRoutinesListeners().then((fn) => { if (alive) cleanup = fn; else fn(); });
    return () => { alive = false; cleanup?.(); };
  });
</script>

<Router {routes} />

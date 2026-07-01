<script lang="ts">
  import Router from 'svelte-spa-router';
  import { router, push } from 'svelte-spa-router';
  import Home from './routes/Home.svelte';
  import Focus from './routes/Focus.svelte';
  import Settings from './routes/Settings.svelte';
  import Report from './routes/Report.svelte';
  import Popover from './routes/Popover.svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { themeStore } from '$lib/theme.svelte';
  import { initRoutinesListeners } from '$lib/routines.svelte';

  const isMain = getCurrentWindow().label === 'main';

  const routes = {
    '/': Home,
    '/focus': Focus,
    '/settings': Settings,
    '/report': Report,
    '/popover': Popover,
  };

  const tabs = [
    { label: '오늘', path: '/' },
    { label: '리포트', path: '/report' },
    { label: '설정', path: '/settings' },
  ];

  $effect(() => {
    if (!isMain) return;
    let alive = true;
    let cleanup: (() => void) | undefined;
    themeStore.init();
    initRoutinesListeners().then((fn) => { if (alive) cleanup = fn; else fn(); });
    return () => { alive = false; cleanup?.(); };
  });
</script>

{#if isMain}
  <div class="app-shell">
    <div class="top-bar">
      <div class="tab-pill">
        {#each tabs as tab}
          <button
            class="tab-seg"
            class:active={router.location === tab.path}
            onclick={() => push(tab.path)}
          >
            {tab.label}
          </button>
        {/each}
      </div>
    </div>
    <div class="content">
      <Router {routes} />
    </div>
  </div>
{:else}
  <Router {routes} />
{/if}

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  .top-bar {
    height: 44px;
    background: var(--chrome);
    border-bottom: 1px solid var(--hair);
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }

  .tab-pill {
    display: flex;
    background: var(--track);
    border-radius: 11px;
    padding: 3px;
    gap: 2px;
  }

  .tab-seg {
    border: none;
    background: transparent;
    color: var(--faint);
    font-family: var(--font-ui);
    font-weight: 600;
    font-size: 11.5px;
    padding: 4px 14px;
    border-radius: 8px;
    cursor: pointer;
    transition: color 0.15s, background 0.15s, box-shadow 0.15s;
  }

  .tab-seg.active {
    background: var(--card);
    color: var(--ink);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1), 0 0 0 0.5px rgba(0, 0, 0, 0.06);
  }

  .content {
    flex: 1;
    overflow-y: auto;
    background: var(--bg);
  }
</style>

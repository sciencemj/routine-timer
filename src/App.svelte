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
  import { commands } from '$lib/commands';

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

    // 모바일(iPhone/iPad)이면 .mobile 클래스로 safe-area·데스크톱 크롬 제거를 켠다.
    commands.isMobile().then((m) => {
      if (alive && m) document.documentElement.classList.add('mobile');
    });

    // 포그라운드 복귀 시 백그라운드 경과를 엔진에 반영(화면 보정).
    const onVisible = () => {
      if (document.visibilityState === 'visible') commands.timerResync();
    };
    document.addEventListener('visibilitychange', onVisible);

    return () => {
      alive = false;
      cleanup?.();
      document.removeEventListener('visibilitychange', onVisible);
    };
  });
</script>

{#if isMain}
  <div class="app-shell">
    <div class="top-bar" data-tauri-drag-region>
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
    /* leave room on the left for the macOS traffic-light buttons (Overlay title bar
       lets them float over this bar so the dots sit on the same row as the tabs) */
    padding: 0 84px;
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

  /* iOS/iPadOS: 노치·홈 인디케이터 여백 + 데스크톱 traffic-light 패딩 제거. */
  :global(html.mobile) .top-bar {
    padding-left: max(16px, env(safe-area-inset-left));
    padding-right: max(16px, env(safe-area-inset-right));
    padding-top: env(safe-area-inset-top);
    height: calc(44px + env(safe-area-inset-top));
  }
  :global(html.mobile) .content {
    padding-bottom: env(safe-area-inset-bottom);
  }
</style>

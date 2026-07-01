<script lang="ts">
  import { push } from 'svelte-spa-router';
  import { timer, initTimerListeners } from '$lib/timer.svelte';
  import { routinesStore } from '$lib/routines.svelte';
  import { commands } from '$lib/commands';
  import FocusView from '$lib/components/FocusView.svelte';

  // Alive-flag async cleanup pattern
  $effect(() => {
    let alive = true;
    let cleanup: (() => void) | undefined;
    routinesStore.refresh();
    commands.timerGetState().then((s) => { if (alive) timer.apply(s); });
    initTimerListeners().then((fn) => { if (alive) cleanup = fn; else fn(); });
    return () => { alive = false; cleanup?.(); };
  });
</script>

<FocusView onBack={() => push('/')} />

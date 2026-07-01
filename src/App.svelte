<script lang="ts">
  import Router from 'svelte-spa-router';
  import Home from './routes/Home.svelte';
  import Focus from './routes/Focus.svelte';
  import Settings from './routes/Settings.svelte';
  import Report from './routes/Report.svelte';
  import Popover from './routes/Popover.svelte';

  const routes = {
    '/': Home,
    '/focus': Focus,
    '/settings': Settings,
    '/report': Report,
    '/popover': Popover,
  };
</script>

<Router {routes} />

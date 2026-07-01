<script lang="ts">
  let { progress = 0, label = '', size = 180, accent = 'var(--accent)' }:
    { progress?: number; label?: string; size?: number; accent?: string } = $props();
  const stroke = 12;
  const r = $derived((size - stroke) / 2);
  const c = $derived(2 * Math.PI * r);
  const offset = $derived(c * (1 - Math.max(0, Math.min(1, progress))));
</script>
<svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}>
  <circle cx={size/2} cy={size/2} r={r} fill="none" stroke="var(--ring-track)" stroke-width={stroke} />
  <circle cx={size/2} cy={size/2} r={r} fill="none" stroke={accent} stroke-width={stroke}
    stroke-linecap="round" stroke-dasharray={c} stroke-dashoffset={offset}
    transform={`rotate(-90 ${size/2} ${size/2})`} />
  <text x="50%" y="50%" text-anchor="middle" dominant-baseline="central"
    class="timer-numerals" font-size={size*0.22} fill="var(--text)">{label}</text>
</svg>

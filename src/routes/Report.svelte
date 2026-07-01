<script lang="ts">
  import { commands } from '$lib/commands';
  import { formatDurationKo } from '$lib/time';
  import type { HeatCell, ReportData } from '$lib/types';

  const WEEKDAY_NAMES = ['일', '월', '화', '수', '목', '금', '토'];

  let data = $state<ReportData | null>(null);

  $effect(() => {
    let alive = true;
    commands.statsReport().then((d) => { if (alive) data = d; });
    return () => { alive = false; };
  });

  const delta = $derived(data ? data.this_week_secs - data.last_week_secs : 0);

  const last7Labels = $derived.by(() => {
    const today = new Date();
    const labels: string[] = [];
    for (let i = 0; i < 7; i++) {
      const d = new Date(today);
      d.setDate(today.getDate() - 6 + i);
      labels.push(WEEKDAY_NAMES[d.getDay()]);
    }
    return labels;
  });

  const last7Max = $derived(data ? Math.max(...data.last7, 1) : 1);

  const heatWeeks = $derived.by(() => {
    if (!data) return [] as HeatCell[][];
    const weeks: HeatCell[][] = [];
    for (let i = 0; i < data.heatmap.length; i += 7) {
      weeks.push(data.heatmap.slice(i, i + 7));
    }
    return weeks;
  });
</script>

<div class="report">
  <div class="content">
    {#if data}
      <h1>집중 기록</h1>

      <!-- KPI row -->
      <div class="card kpi-row">
        <div class="kpi">
          <p class="kpi-value">{formatDurationKo(data.this_week_secs)}</p>
          <p class="kpi-label">이번 주 집중</p>
        </div>
        <div class="kpi">
          <p class="kpi-value">{formatDurationKo(data.daily_avg_secs)}</p>
          <p class="kpi-label">일 평균</p>
        </div>
        <div class="kpi">
          <p class="kpi-value" style="color: {delta >= 0 ? 'var(--pos)' : 'var(--neg)'}">
            {delta === 0 ? '±0' : `${delta > 0 ? '+' : '-'}${formatDurationKo(Math.abs(delta))}`}
          </p>
          <p class="kpi-label">지난주 대비</p>
        </div>
        <div class="kpi">
          <p class="kpi-value">{data.streak}일 · 최고 {data.best_streak}일</p>
          <p class="kpi-label">연속</p>
        </div>
      </div>

      <!-- 7-day bar chart -->
      <div class="card chart-card">
        <p class="card-title">최근 7일</p>
        <div class="bars">
          {#each data.last7 as secs, i (i)}
            <div class="bar-col">
              <div class="bar-track">
                <div
                  class="bar-fill"
                  class:today={i === data.last7.length - 1}
                  style="height: {Math.max(3, (secs / last7Max) * 100)}%"
                ></div>
              </div>
              <span class="bar-label">{last7Labels[i]}</span>
            </div>
          {/each}
        </div>
      </div>

      <!-- 13-week heatmap -->
      <div class="card heatmap-card">
        <p class="card-title">13주 잔디 · 하루 집중 시간</p>
        <div class="heatmap-row">
          <div class="heatmap-weeks">
            {#each heatWeeks as week, wi (wi)}
              <div class="heatmap-col">
                {#each week as cell (cell.date)}
                  <div
                    class="heat-cell"
                    style="background: var(--g{cell.level})"
                    title={`${cell.date} · ${formatDurationKo(cell.secs)}`}
                  ></div>
                {/each}
              </div>
            {/each}
          </div>
          <div class="legend">
            <span class="legend-label">적음</span>
            <span class="heat-cell legend-cell" style="background: var(--g0)"></span>
            <span class="heat-cell legend-cell" style="background: var(--g1)"></span>
            <span class="heat-cell legend-cell" style="background: var(--g2)"></span>
            <span class="heat-cell legend-cell" style="background: var(--g3)"></span>
            <span class="heat-cell legend-cell" style="background: var(--g4)"></span>
            <span class="legend-label">많음</span>
          </div>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .report {
    min-height: 100%;
    background: var(--bg);
    display: flex;
    justify-content: center;
  }
  .content {
    width: 100%;
    max-width: 544px;
    padding: 28px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  h1 {
    margin: 0;
    font-size: 20px;
    font-weight: 600;
    color: var(--ink);
  }

  .card {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--r-card);
    padding: 16px;
  }

  .card-title {
    margin: 0 0 12px;
    font-size: 12px;
    color: var(--faint);
  }

  /* KPI row */
  .kpi-row {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
  }
  .kpi {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    text-align: center;
  }
  .kpi-value {
    margin: 0;
    font-family: var(--font-display);
    font-variant-numeric: tabular-nums;
    font-size: 22px;
    font-weight: 600;
    color: var(--ink);
    line-height: 1.2;
  }
  .kpi-label {
    margin: 0;
    font-size: 12px;
    color: var(--faint);
  }

  /* 7-day bar chart */
  .bars {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 8px;
    height: 120px;
  }
  .bar-col {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    height: 100%;
  }
  .bar-track {
    width: 100%;
    max-width: 22px;
    flex: 1;
    display: flex;
    align-items: flex-end;
    background: var(--track);
    border-radius: var(--r-bar);
    overflow: hidden;
  }
  .bar-fill {
    width: 100%;
    background: var(--bar-base);
    border-radius: var(--r-bar);
    transition: height 300ms;
  }
  .bar-fill.today {
    background: var(--accent);
  }
  .bar-label {
    font-size: 11px;
    color: var(--faint);
  }

  /* Heatmap */
  .heatmap-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }
  .heatmap-weeks {
    display: flex;
    gap: 3px;
    overflow-x: auto;
  }
  .heatmap-col {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .heat-cell {
    width: 11px;
    height: 11px;
    border-radius: 2px;
  }
  .legend {
    display: flex;
    align-items: center;
    gap: 3px;
    flex-shrink: 0;
  }
  .legend-cell {
    width: 10px;
    height: 10px;
  }
  .legend-label {
    font-size: 11px;
    color: var(--faint);
    margin: 0 2px;
  }
</style>

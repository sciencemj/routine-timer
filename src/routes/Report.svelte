<div class="report">
  <div class="content">
    <div class="card">
      <h1>집중 기록</h1>
      <p class="placeholder">리포트는 곧 추가됩니다</p>
    </div>
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
  }
  .card {
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--r-card);
    padding: 48px 24px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    text-align: center;
  }

  h1 {
    margin: 0;
    font-size: 20px;
    font-weight: 600;
    color: var(--ink);
  }

  .placeholder {
    margin: 0;
    font-size: 14px;
    color: var(--muted);
  }
</style>

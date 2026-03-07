export function MetricGauge({ label, value, percent, sub }: { label: string; value: string; percent: number; sub: string }) {
  const clamped = Math.max(0, Math.min(100, percent));
  const angle = Math.round((clamped / 100) * 360);

  return (
    <div className="metric-gauge shell-card">
      <div className="metric-ring" style={{ background: `conic-gradient(var(--purple) ${angle}deg, rgba(255,255,255,0.08) 0deg)` }}>
        <div className="metric-ring-inner">
          <div className="eyebrow">{label}</div>
          <div className="metric-value">{value}</div>
        </div>
      </div>
      <div className="row-sub">{sub}</div>
    </div>
  );
}

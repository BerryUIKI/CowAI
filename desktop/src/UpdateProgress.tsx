interface UpdateProgressProps {
  downloaded: number;
  total: number | null;
  className?: string;
}

export default function UpdateProgress({
  downloaded,
  total,
  className,
}: UpdateProgressProps) {
  const percent =
    total && total > 0 ? Math.min(100, Math.round((downloaded / total) * 100)) : null;

  return (
    <div
      className={`update-progress${className ? ` ${className}` : ""}`}
      aria-label="Download progress"
    >
      {percent !== null ? (
        <div className="update-progress-track">
          <div className="update-progress-fill" style={{ width: `${percent}%` }} />
        </div>
      ) : null}
      <span className="update-progress-label">
        {percent === null ? "Downloading…" : `Downloading… ${percent}%`}
      </span>
    </div>
  );
}

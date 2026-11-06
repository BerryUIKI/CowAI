import UpdateProgress from "./UpdateProgress";
import type { Updater } from "./updater";

interface UpdateBannerProps {
  updater: Updater;
  tasksRunning: boolean;
}

/**
 * Launch-time update prompt. It only appears after a successful check or an
 * installation attempt; manual check results live in Settings.
 */
export default function UpdateBanner({ updater, tasksRunning }: UpdateBannerProps) {
  const { state } = updater;
  const visible =
    !state.dismissed &&
    (state.phase === "available" ||
      state.phase === "downloading" ||
      state.phase === "ready" ||
      (state.phase === "error" && state.version !== null));

  if (!visible) return null;

  return (
    <div className="update-banner" role="status">
      <div className="update-banner-body">
        <span className="update-banner-title">
          {state.phase === "ready"
            ? `CowBot ${state.version} is installed`
            : `CowBot ${state.version} is available`}
        </span>

        {state.phase === "available" ? (
          <span className="update-banner-hint">
            {tasksRunning
              ? "Finish running tasks before updating."
              : "Download and install the new version now."}
          </span>
        ) : null}

        {state.phase === "downloading" ? (
          <UpdateProgress downloaded={state.downloaded} total={state.total} />
        ) : null}

        {state.phase === "ready" ? (
          <span className="update-banner-hint">Restart CowBot to finish updating.</span>
        ) : null}

        {state.phase === "error" && state.error ? (
          <span className="update-banner-hint is-error">{state.error}</span>
        ) : null}
      </div>

      <div className="update-banner-actions">
        {state.phase === "available" ? (
          <button
            type="button"
            className="primary-button"
            onClick={() => void updater.install()}
            disabled={tasksRunning}
          >
            Download and install
          </button>
        ) : null}

        {state.phase === "ready" ? (
          <button
            type="button"
            className="primary-button"
            onClick={() => void updater.restart()}
          >
            Restart now
          </button>
        ) : null}

        {state.phase === "error" ? (
          <button
            type="button"
            className="primary-button"
            onClick={() => void updater.install()}
          >
            Try again
          </button>
        ) : null}

        {state.phase !== "downloading" ? (
          <button type="button" className="ghost-button" onClick={updater.dismiss}>
            Later
          </button>
        ) : null}
      </div>
    </div>
  );
}

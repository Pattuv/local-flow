import React from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Info, X } from "lucide-react";
import "./overlay.css";

const TAP_ERROR_DISMISS_MS = 8000;
const DISMISS_RING_RADIUS = 10;
const DISMISS_RING_CIRCUMFERENCE = 2 * Math.PI * DISMISS_RING_RADIUS;

function TapErrorDismissButton({ durationMs }) {
  const progressRef = React.useRef(null);

  React.useEffect(() => {
    const circle = progressRef.current;
    if (!circle) {
      return;
    }

    const circumference = DISMISS_RING_CIRCUMFERENCE;
    circle.style.strokeDasharray = `${circumference} ${circumference}`;
    circle.style.strokeDashoffset = "0";
    circle.style.transition = "none";

    circle.getBoundingClientRect();

    circle.style.transition = `stroke-dashoffset ${durationMs}ms linear`;
    circle.style.strokeDashoffset = `${circumference}`;
  }, [durationMs]);

  return (
    <div className="taperror-dismiss" aria-hidden="true">
      <svg
        className="taperror-dismiss-ring"
        viewBox="0 0 24 24"
        aria-hidden="true"
      >
        <circle
          cx="12"
          cy="12"
          r={DISMISS_RING_RADIUS}
          fill="none"
          stroke="currentColor"
          strokeOpacity="0.2"
          strokeWidth="2"
        />
        <circle
          ref={progressRef}
          cx="12"
          cy="12"
          r={DISMISS_RING_RADIUS}
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          className="taperror-dismiss-progress"
        />
      </svg>
      <X className="taperror-dismiss-icon" strokeWidth={2.5} aria-hidden="true" />
    </div>
  );
}

function Overlay() {
  const [dictationActive, setDictationActive] = React.useState(false);
  const [showTapError, setShowTapError] = React.useState(false);
  const hasShownTapErrorRef = React.useRef(false);
  const dismissTimerRef = React.useRef(null);

  const tapErrorRef = React.useRef(null);

  const getTapErrorBounds = React.useCallback(async () => {
    const element = tapErrorRef.current;
    if (!element) {
      return null;
    }

    const rect = element.getBoundingClientRect();
    if (rect.width <= 0 || rect.height <= 0) {
      return null;
    }

    const window = getCurrentWindow();
    const scale = await window.scaleFactor();
    const position = await window.outerPosition();

    return {
      x: position.x + rect.left * scale,
      y: position.y + rect.top * scale,
      width: rect.width * scale,
      height: rect.height * scale,
    };
  }, []);

  const dismissTapError = React.useCallback(() => {
    if (dismissTimerRef.current) {
      clearTimeout(dismissTimerRef.current);
      dismissTimerRef.current = null;
    }

    setShowTapError(false);
  }, []);

  const openTapError = React.useCallback(() => {
    setShowTapError(true);

    if (dismissTimerRef.current) {
      clearTimeout(dismissTimerRef.current);
    }

    dismissTimerRef.current = setTimeout(() => {
      dismissTimerRef.current = null;
      dismissTapError();
    }, TAP_ERROR_DISMISS_MS);
  }, [dismissTapError]);

  React.useEffect(() => {
    if (!showTapError) {
      return;
    }

    let unlistenDismiss;
    let cancelled = false;

    const startDismissWatch = async () => {
      await new Promise((resolve) => requestAnimationFrame(resolve));
      if (cancelled) {
        return;
      }

      const bounds = await getTapErrorBounds();
      if (cancelled) {
        return;
      }

      await invoke("set_tap_error_click_dismiss_watch", {
        watching: true,
        bounds,
      });
    };

    startDismissWatch().catch(() => {});

    listen("taperror:dismiss", () => {
      dismissTapError();
    }).then((dispose) => {
      unlistenDismiss = dispose;
    });

    return () => {
      cancelled = true;
      unlistenDismiss?.();
      invoke("set_tap_error_click_dismiss_watch", {
        watching: false,
        bounds: null,
      }).catch(() => {});
    };
  }, [showTapError, dismissTapError, getTapErrorBounds]);

  React.useEffect(() => {
    let unlisten;

    listen("dictation:state", (event) => {
      const active = Boolean(event.payload?.active);
      setDictationActive(active);

      if (active || !event.payload?.tooShort || hasShownTapErrorRef.current) {
        return;
      }

      hasShownTapErrorRef.current = true;
      openTapError();
    }).then((dispose) => {
      unlisten = dispose;
    });

    return () => {
      unlisten?.();
      if (dismissTimerRef.current) {
        clearTimeout(dismissTimerRef.current);
      }
    };
  }, [openTapError]);

  React.useEffect(() => {
    const blockReloadAndInspect = (event) => {
      const key = event.key?.toLowerCase();
      const isMod = event.metaKey || event.ctrlKey;

      // Block reload (Cmd/Ctrl+R, F5) and common inspect shortcuts.
      if (key === "f5") {
        event.preventDefault();
        event.stopPropagation();
        return;
      }

      if (isMod && (key === "r" || key === "u")) {
        event.preventDefault();
        event.stopPropagation();
        return;
      }

      if (isMod && event.altKey && (key === "i" || key === "j" || key === "c")) {
        event.preventDefault();
        event.stopPropagation();
      }
    };

    const blockContextMenu = (event) => {
      event.preventDefault();
      event.stopPropagation();
    };

    window.addEventListener("keydown", blockReloadAndInspect, true);
    window.addEventListener("contextmenu", blockContextMenu, true);

    return () => {
      window.removeEventListener("keydown", blockReloadAndInspect, true);
      window.removeEventListener("contextmenu", blockContextMenu, true);
    };
  }, []);

  return (
    <main className="overlay-root">
      <div
        className={`mic-notch flex items-center gap-2 rounded-full border border-gray-700 bg-black p-2 px-4 ${dictationActive ? " mic-notch--visible" : ""}`}
        id="mic-notch"
      >
        <p className="text-sm text-white">- - - - - -</p>
      </div>

      <div
        ref={tapErrorRef}
        className={`taperror relative rounded-2xl border border-gray-700 bg-black py-6 pl-6 pr-11 text-lg${showTapError ? " taperror--visible" : ""}`}
      >
        {showTapError ? (
          <TapErrorDismissButton durationMs={TAP_ERROR_DISMISS_MS} />
        ) : null}
        <p className="taperror-message text-md mr-6 text-white">
          <Info className="h-5 w-5 shrink-0 text-yellow-500 mr-2" aria-hidden="true" />
          Hold control + ⌥ to dictate
        </p>
      </div>
    </main>
  );
}

ReactDOM.createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <Overlay />
  </React.StrictMode>,
);

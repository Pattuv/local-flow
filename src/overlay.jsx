import React from "react";
import ReactDOM from "react-dom/client";
import "./overlay.css";

function Overlay() {
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
      {/* Mic notch / prompt UI will live here. */}
    </main>
  );
}

ReactDOM.createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <Overlay />
  </React.StrictMode>,
);

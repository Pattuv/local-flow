import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

function CopyIcon({ className }) {
  return (
    <svg
      className={className}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <rect x="9" y="9" width="13" height="13" rx="2" />
      <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
    </svg>
  );
}

function CheckIcon({ className }) {
  return (
    <svg
      className={className}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2.5"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M20 6 9 17l-5-5" />
    </svg>
  );
}

function App() {
  const [draft, setDraft] = useState("");
  const [copied, setCopied] = useState(false);
  const copiedTimeoutRef = useRef(null);

  useEffect(() => {
    return () => {
      if (copiedTimeoutRef.current) {
        clearTimeout(copiedTimeoutRef.current);
      }
    };
  }, []);

  async function handleTitlebarMouseDown(event) {
    if (event.button !== 0) return;
    try {
      await getCurrentWindow().startDragging();
    } catch (error) {
      console.error("Failed to start window drag:", error);
    }
  }

  async function handleCopy() {
    const text = draft.trim();
    if (!text || copied) return;

    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      if (copiedTimeoutRef.current) {
        clearTimeout(copiedTimeoutRef.current);
      }
      copiedTimeoutRef.current = setTimeout(() => {
        setCopied(false);
      }, 1400);
    } catch (error) {
      console.error("Failed to copy text:", error);
    }
  }

  return (
    <main className="relative flex h-full w-full flex-col bg-white">
      <div
        className="absolute left-[-30px] right-[-30px] top-[-30px] z-10 h-[74px]"
        data-tauri-drag-region
        onMouseDown={handleTitlebarMouseDown}
      />
      <header
        className="relative z-20 flex h-14 w-full shrink-0 items-center select-none"
        data-tauri-drag-region
        onMouseDown={handleTitlebarMouseDown}
      >
        <p className="mt-7 text-xl font-semibold tracking-tight">
          Local<span className="text-purple-600">~</span>Flow
        </p>
      </header>
      <div
        className="main-content flex w-full flex-1 flex-col items-center justify-center"
        id="main-content"
      >
        <p className="text-3xl font-medium tracking-tight">Welcome, Pratyush</p>
        <p className="mt-3 text-gray-500">
          Press control + ⌥ to speak into any textbox.
        </p>

        <div className="relative mt-6 w-full max-w-[370px]">
          <textarea
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            className="h-20 w-full resize-none rounded-2xl border border-gray-200 bg-gray-50 p-4 pr-11 text-sm placeholder:text-gray-400 focus:outline-none focus:ring-1 focus:ring-gray-300/50 ease-in-out duration-300"
            placeholder="Try it out here!"
          />

          <button
            type="button"
            onClick={handleCopy}
            disabled={!draft.trim()}
            aria-label={copied ? "Copied" : "Copy text"}
            className="absolute top-2.5 right-2.5 flex h-7 w-7 items-center justify-center rounded-lg text-gray-400 transition-colors hover:bg-gray-200/70 hover:text-gray-700 disabled:pointer-events-none disabled:opacity-30"
          >
            <span className="relative flex h-4 w-4 items-center justify-center">
              <CopyIcon
                className={`absolute h-4 w-4 transition-all duration-200 ease-out ${
                  copied
                    ? "scale-0 opacity-0"
                    : "scale-100 opacity-100"
                }`}
              />
              <CheckIcon
                className={`absolute h-4 w-4 text-emerald-500 transition-all duration-200 ease-out ${
                  copied
                    ? "scale-100 opacity-100"
                    : "scale-0 opacity-0"
                }`}
              />
            </span>
          </button>
        </div>
      </div>
    </main>
  );
}

export default App;

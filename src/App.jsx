import { getCurrentWindow } from "@tauri-apps/api/window";

function App() {
  async function handleTitlebarMouseDown(event) {
    if (event.button !== 0) return;
    try {
      await getCurrentWindow().startDragging();
    } catch (error) {
      console.error("Failed to start window drag:", error);
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
        <p className="text-xl font-semibold tracking-tight mt-7">
          Local<span className="text-purple-600">~</span>Flow
        </p>
      </header>
      <div className="main-content h-full w-full flex-1" id="main-content">
      </div>
    </main>
  );
}

export default App;

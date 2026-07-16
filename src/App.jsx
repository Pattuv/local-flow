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
      <div
        className="main-content flex w-full flex-1 flex-col items-center justify-center"
        id="main-content"
      >
        <p className="text-3xl font-medium tracking-tight">Welcome, Pratyush</p>
        <p className="text-gray-500 mt-3">Press control + ⌥ to speak into any textbox.</p>
        <textarea className="w-full max-w-[370px] h-30  mt-6 p-4 text-sm border border-gray-200 bg-gray-50 rounded-2xl placeholder:text-gray-400 focus:outline-none focus:ring-1 focus:ring-gray-300/50 ease-in-out duration-300 resize-none" placeholder="Try it out here!" />
      </div>
    </main>
  );
}

export default App;

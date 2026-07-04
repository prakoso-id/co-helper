import { ChatWindow } from "./components/ChatWindow";
import { SourceSelector } from "./components/SourceSelector";
import { VadIndicator } from "./components/VadIndicator";
import { useChatStore } from "./store/chat";

export default function App() {
  const status = useChatStore((s) => s.status);

  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden">
      {/* Header */}
      <div className="flex items-center gap-2 px-3 py-2 bg-[#16213e] border-b border-[#0f3460]">
        <span className="text-lg font-bold text-[#e94560]">CO-Helper</span>
        <VadIndicator />
        <div className="ml-auto">
          <SourceSelector />
        </div>
      </div>

      {/* Chat */}
      <div className="flex-1 overflow-hidden">
        <ChatWindow />
      </div>
    </div>
  );
}
